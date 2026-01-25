use crate::gfx::pipeline::*;
use crate::{Parameters, RingBuffer, gfx::queue::Queue, gfx::*, io, warning};

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::time::{Duration, Instant};

const GEOMETRY_TEXTURE_ID: u32 = u32::MAX;

struct TestData {
    linear_sampler: Rc<Sampler>,
    nearest_sampler: Rc<Sampler>,
    test_pipeline: Rc<Pipeline>,
}

struct ImageUpload {
    source: Rc<MemoryBuffer>,
    destination: Rc<PixelBuffer>,
    image_id: u32,
}

struct BufferUpload {
    source: Rc<MemoryBuffer>,
    destination: Rc<MemoryBuffer>,
    destination_offset: usize,
    buffer_id: u32,
}

#[repr(C)]
struct PerCommand {
    scale: Vector2f,
}

/// The core graphics subsystem, managing Vulkan initialization and device selection.
///
/// `Core` encapsulates a Vulkan instance and a selected physical device.
/// It is responsible for creating and maintaining the graphics pipeline.
/// `Core` is thread-safe and can be shared across threads.
pub struct Gfx {
    test_data: TestData,
    pending_images: Arc<Mailbox<(u32, io::Image)>>,
    pending_image_uploads: VecDeque<ImageUpload>,
    pending_buffers: Arc<Mailbox<(u32, io::Buffer)>>,
    pending_buffer_uploads: VecDeque<BufferUpload>,
    render_commands: Arc<Mailbox<RenderFrame>>,
    cached_gui_commands: Option<RenderFrame>,
    /// Small pool of reusable RenderFrame instances for the GUI thread.
    render_frame_pool: Vec<RenderFrame>,
    next_resource_id: u32,
    pixel_buffers: HashMap<u32, Rc<PixelBuffer>>,
    memory_buffers: HashMap<u32, Rc<MemoryBuffer>>,
    // TODO: Maybe find a better way to ensure no accidental resurrection of deleted pixel buffers
    pixel_buffers_deleted: HashSet<u32>,
    memory_buffers_deleted: HashSet<u32>,
    released_images: Arc<Mailbox<u32>>,
    released_buffers: Arc<Mailbox<u32>>,
    frame_n: u64,
    frame_skips: u64,
    frame_cpu_times: RingBuffer<f64>,
    max_uploads_per_frame: usize,
    work_available: Arc<AutoResetEvent>,
    target_frame: Option<Rc<PixelBuffer>>,
    target_damaged: bool,
    queue: Queue,
    allocator: Rc<allocator::Allocator>,
    device: Rc<Device>,
    instance: Instance,
}

// SAFETY: Core manages a Vulkan instance which can be safely shared and accessed across threads.
// The Vulkan instance itself is thread-safe for most operations.
unsafe impl Send for Gfx {}
unsafe impl Sync for Gfx {}

/// Runs the main graphics loop.
///
/// Intended to be called from a dedicated graphics thread.
/// Behavior when calling this from the main thread is undefined.
pub fn run(am_gfx: Arc<Mutex<gfx::Gfx>>, exit_flag: &std::sync::atomic::AtomicBool, mode: RunMode) {
    Gfx::run(am_gfx, exit_flag, mode);
}

impl Drop for Gfx {
    fn drop(&mut self) {
        warning!("Gfx::drop");
    }
}

impl Gfx {
    /// Signals that work is available, waking up the GFX thread if it's waiting
    /// in event-driven mode. This is a no-op in continuous mode.
    pub fn wake_up(&self) {
        self.work_available.set();
    }

    fn run(am_gfx: Arc<Mutex<gfx::Gfx>>, exit_flag: &std::sync::atomic::AtomicBool, mode: RunMode) {
        log::set_current_thread_name("gfx");

        info!("GFX START");

        const TARGET_FPS: u32 = 120;
        let target_frame_time = Duration::from_secs_f64(1.0 / TARGET_FPS as f64);
        const SPIN_THRESHOLD: Duration = Duration::from_micros(500);

        let start_instant = Instant::now();
        let mut last_instant = Instant::now();
        let mut last_stat_instant = Instant::now();
        let mut frame_delta_times: RingBuffer<f64> = RingBuffer::new(120);

        let work_available = { am_gfx.lock().unwrap().work_available.clone() };

        loop {
            let frame_start = Instant::now();

            // Check for exit signal
            if exit_flag.load(Ordering::Relaxed) {
                break;
            }

            // Handle event-driven mode: wait for work notification
            if let RunMode::Event = mode {
                work_available.wait();

                if exit_flag.load(Ordering::Relaxed) {
                    break;
                }
            }

            {
                let now = Instant::now();
                let _t = now.duration_since(start_instant).as_secs_f64();
                let _dt = now.duration_since(last_instant).as_secs_f32();
                last_instant = now;
                frame_delta_times.push(_dt as f64);

                let mut gfx = am_gfx.lock().unwrap();
                gfx.render();

                let stat_now = Instant::now();
                if stat_now.duration_since(last_stat_instant).as_secs_f64() >= 1.0 {
                    let (allocations, deallocations) = alloc::counts();
                    let current_ram = alloc::rss_bytes().unwrap_or(0);
                    info!(
                        "RAM -> {} (live allocs: {}, total allocs: {})",
                        utilities::format_bytes_iec(current_ram),
                        allocations - deallocations,
                        allocations
                    );

                    let (vram_allocations, vram_deallocations) =
                        allocator::Allocator::alloc_counts();
                    let current_vram = allocator::Allocator::current_allocated_bytes();
                    info!(
                        "VRAM -> {} (live allocs: {}, total allocs: {})",
                        utilities::format_bytes_iec(current_vram),
                        vram_allocations - vram_deallocations,
                        vram_allocations
                    );

                    let full_frames = gfx.frame_n - gfx.frame_skips;
                    info!("GFX frame -> {} (full: {})", gfx.frame_n, full_frames);

                    if let Some((min, max, mean)) = gfx.frame_cpu_times.min_max_mean() {
                        debug!(
                            "GFX cpu -> min: {}, max: {}, mean: {}",
                            utilities::format_duration(min),
                            utilities::format_duration(max),
                            utilities::format_duration(mean)
                        );
                    }

                    if let Some((min, max, mean)) = frame_delta_times.min_max_mean() {
                        debug!(
                            "GFX dt -> min: {}, max: {}, mean: {}",
                            utilities::format_duration(min),
                            utilities::format_duration(max),
                            utilities::format_duration(mean)
                        );
                    }

                    last_stat_instant = stat_now;
                }
            }

            // Frame rate limiting only in continuous mode
            if let RunMode::Continuous = mode {
                let elapsed = frame_start.elapsed();
                if elapsed < target_frame_time {
                    let mut remaining = target_frame_time - elapsed;

                    // Sleep for bulk of remaining time
                    while remaining > SPIN_THRESHOLD {
                        std::thread::sleep(Duration::from_millis(1));
                        remaining = target_frame_time.saturating_sub(frame_start.elapsed());
                    }

                    // Spin for precise timing
                    while frame_start.elapsed() < target_frame_time {
                        std::hint::spin_loop();
                    }
                }
            }
        }

        info!("GFX STOP");
    }

    pub fn current_allocated_vram_bytes() -> usize {
        allocator::Allocator::current_allocated_bytes()
    }

    pub fn new(
        instance: Instance,
        device: Rc<Device>,
        allocator: Rc<allocator::Allocator>,
    ) -> Self {
        let linear_sampler = Sampler::new(
            device.clone(),
            Filter::Linear,
            SamplerAddressMode::ClampToBorder,
            SamplerBorderColor::FloatTransparentBlack,
        )
        .unwrap();

        let nearest_sampler = Sampler::new(
            device.clone(),
            Filter::Nearest,
            SamplerAddressMode::ClampToBorder,
            SamplerBorderColor::FloatOpaqueWhite,
        )
        .unwrap();

        let test_pipeline_params = pipeline::Parameters {
            shader: shaders::get_shaderprogram(shaders::OVERLAY_SHADER_ID)
                .expect("Failed to get overlay shader program"),
            topology: Topology::Triangles,
            winding: Winding::CounterClockwise,
            culling: Culling::Back,
            raster: Raster::Fill,
            samples: Samples::X1,
            depth_comparison: Comparison::Always,
            depth_write: false,
            depth_test: false,
            bindings: vec![Binding {
                binding: 0,
                stride: std::mem::size_of::<Gpu2DVertex>() as u32,
                rate: Rate::VERTEX,
            }],
            attributes: vec![
                Attribute {
                    binding: 0,
                    location: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: std::mem::offset_of!(Gpu2DVertex, position) as u32,
                },
                Attribute {
                    binding: 0,
                    location: 1,
                    format: Format::R32G32_SFLOAT,
                    offset: std::mem::offset_of!(Gpu2DVertex, texture) as u32,
                },
                Attribute {
                    binding: 0,
                    location: 2,
                    format: Format::U32,
                    offset: std::mem::offset_of!(Gpu2DVertex, color) as u32,
                },
            ],
            push_constants: vec![PushConstantRange {
                stage_flags: Stage::VERTEX | Stage::FRAGMENT,
                offset: 0,
                size: std::mem::size_of::<PerCommand>() as u32,
            }],
            descriptors: vec![
                Descriptor::new(0, DescriptorType::SampledImage, Stage::FRAGMENT),
                Descriptor::new(1, DescriptorType::Sampler, Stage::FRAGMENT),
            ],
            attachments: vec![Attachment::new(
                Format::R8G8B8A8_UNORM,
                AttachmentBlend::straight_alpha_blend(),
            )],
        };
        let test_pipeline = Pipeline::new(Rc::clone(&device), &test_pipeline_params).unwrap();

        // Create fallback texture: 1x1 white pixel
        let fallback_pixel_data = [0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8]; // RGBA white
        let fallback_upload = allocator
            .create_memory_buffer::<u8>(
                4,
                buffer::MemoryUsage::TRANSFER_SRC,
                buffer::MemoryAccess::WRITE,
            )
            .unwrap();
        fallback_upload.write(&fallback_pixel_data);

        let fallback_texture_buffer = allocator
            .create_pixel_buffer(
                Format::R8G8B8A8_UNORM,
                1,
                1,
                ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                ImageAspect::COLOR,
                Samples::X1,
            )
            .unwrap();
        let fallback_texture = Rc::new(fallback_texture_buffer);

        // Set up fallback texture to be uploaded in first render
        let fallback_texture_clone = Rc::clone(&fallback_texture);
        let pending_fallback = ImageUpload {
            source: Rc::new(fallback_upload),
            destination: fallback_texture_clone,
            image_id: GEOMETRY_TEXTURE_ID,
        };

        let mut pending_uploads = VecDeque::new();
        pending_uploads.push_back(pending_fallback);

        let mut pixel_buffers = HashMap::new();
        pixel_buffers.insert(GEOMETRY_TEXTURE_ID, Rc::clone(&fallback_texture));

        // Create all available font atlases
        for font_id in fonts::ALL_FONT_IDS {
            if let Some(font_data) = fonts::get_font_atlas(*font_id) {
                let font_upload = allocator
                    .create_memory_buffer::<u8>(
                        font_data.atlas_data.len(),
                        buffer::MemoryUsage::TRANSFER_SRC,
                        buffer::MemoryAccess::WRITE,
                    )
                    .unwrap();
                font_upload.write(font_data.atlas_data);

                let font_texture_buffer = allocator
                    .create_pixel_buffer(
                        Format::R8G8B8A8_UNORM,
                        font_data.width,
                        font_data.height,
                        ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                        ImageAspect::COLOR,
                        Samples::X1,
                    )
                    .unwrap();
                let font_texture = Rc::new(font_texture_buffer);

                pixel_buffers.insert(font_data.texture_id, Rc::clone(&font_texture));

                let font_texture_clone = Rc::clone(&font_texture);
                let pending_font = ImageUpload {
                    source: Rc::new(font_upload),
                    destination: font_texture_clone,
                    image_id: font_data.texture_id,
                };
                pending_uploads.push_back(pending_font);
            }
        }

        let test_data = TestData {
            linear_sampler: Rc::new(linear_sampler),
            nearest_sampler: Rc::new(nearest_sampler),
            test_pipeline: Rc::new(test_pipeline),
        };

        let command_queue = Queue::new(&device, 4);
        let work_available = Arc::new(AutoResetEvent::new());

        Gfx {
            instance,
            device,
            allocator,
            test_data,
            frame_n: 0,
            frame_skips: 0,
            queue: command_queue,
            pixel_buffers,
            pixel_buffers_deleted: HashSet::new(),
            memory_buffers_deleted: HashSet::new(),
            pending_images: Mailbox::<(u32, io::Image)>::new(Arc::clone(&work_available)),
            pending_image_uploads: pending_uploads,
            memory_buffers: HashMap::new(),
            released_buffers: Mailbox::new(Arc::clone(&work_available)),
            pending_buffer_uploads: VecDeque::new(),
            pending_buffers: Mailbox::new(Arc::clone(&work_available)),
            render_commands: Mailbox::<RenderFrame>::new(Arc::clone(&work_available)),
            cached_gui_commands: None,
            render_frame_pool: Vec::with_capacity(3),
            next_resource_id: 0,
            released_images: Mailbox::new(Arc::clone(&work_available)),
            frame_cpu_times: RingBuffer::new(120),
            max_uploads_per_frame: 4,
            work_available,
            target_frame: None,
            target_damaged: false,
        }
    }

    /// Enumerates all available physical devices (GPUs) accessible via the Vulkan instance.
    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let devices = self.instance.enumerate_physical_devices()?;
        Ok(devices)
    }

    /// Creates a new `GfxBuilder` to configure and build a `Gfx` instance.
    pub fn builder(platform: Platform) -> GfxBuilder {
        GfxBuilder::new(platform)
    }

    /// Returns a reference to the selected physical device.
    pub fn selected_physical_device(&self) -> &PhysicalDevice {
        self.device.physical_device()
    }

    pub fn vulkan_instance_handle(&self) -> vulkan::VkInstance {
        self.instance.handle()
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn device(&self) -> &Rc<Device> {
        &self.device
    }

    pub fn allocator(&self) -> &Rc<allocator::Allocator> {
        &self.allocator
    }

    pub fn next_frame(&mut self) -> u64 {
        self.frame_n += 1;
        self.frame_n
    }

    pub fn frame_cpu_times(&self) -> &RingBuffer<f64> {
        &self.frame_cpu_times
    }

    pub fn drop_presentation(&mut self) {
        self.queue.drop_presenter();
    }

    pub fn initialize_swapchain(&mut self, parameters: presentation::Parameters) {
        let swapchain_size = Vector2u::new(parameters.width, parameters.height);

        self.queue.swap_presenter(parameters);

        let target_frame = self
            .allocator
            .create_pixel_buffer(
                Format::R8G8B8A8_UNORM,
                swapchain_size.x,
                swapchain_size.y,
                ImageUsage::SAMPLED
                    | ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_DST
                    | ImageUsage::TRANSFER_SRC,
                ImageAspect::COLOR,
                Samples::X1,
            )
            .unwrap();
        self.target_frame = Some(Rc::new(target_frame));
        self.target_damaged = true;

        self.render();
    }

    pub fn create_pipeline(&self, parameters: pipeline::Parameters) -> Pipeline {
        Pipeline::new(self.device.clone(), &parameters).unwrap()
    }

    pub fn create_image(&mut self, io_image: io::Image) -> Image {
        let image_id = self.next_resource_id;
        self.next_resource_id += 1;

        let image = Image::new(
            image_id,
            io_image.width,
            io_image.height,
            Arc::downgrade(&self.released_images),
        );

        self.pending_images.push((image_id, io_image));

        image
    }

    pub fn create_dynamic_image(&mut self) -> Image {
        let image_id = self.next_resource_id;
        self.next_resource_id += 1;

        Image::new(image_id, 0, 0, Arc::downgrade(&self.released_images))
    }

    pub fn create_buffer(&mut self, size: usize) -> Buffer {
        let buffer_id = self.next_resource_id;
        self.next_resource_id += 1;

        warning!(
            "Renderer::create_buffer -> {}",
            utilities::format_bytes_iec(size)
        );

        let memory_buffer = self
            .allocator
            .create_memory_buffer::<u8>(
                size,
                buffer::MemoryUsage::TRANSFER_DST
                    | buffer::MemoryUsage::VERTEX_BUFFER
                    | buffer::MemoryUsage::INDEX_BUFFER,
                buffer::MemoryAccess::NONE,
            )
            .unwrap();
        self.memory_buffers
            .insert(buffer_id, Rc::new(memory_buffer));

        Buffer::new(buffer_id, size, Arc::downgrade(&self.released_buffers))
    }

    pub fn image_mailbox(&self) -> Arc<Mailbox<(u32, io::Image)>> {
        Arc::clone(&self.pending_images)
    }

    pub fn render_mailbox(&self) -> Arc<Mailbox<RenderFrame>> {
        Arc::clone(&self.render_commands)
    }

    /// Acquire a reusable `RenderFrame` for the GUI thread.
    ///
    /// If the GFX thread has a cached frame available, reuse its allocations
    /// by clearing the contents while keeping capacity. Otherwise allocate
    /// a fresh `RenderFrame` with the requested `size`.
    pub fn acquire_frame_for_gui(&mut self, size: Vector2u) -> RenderFrame {
        // Try to reuse a frame from the small pool first to avoid allocations.
        if let Some(mut frame) = self.render_frame_pool.pop() {
            frame.clear();
            frame.size = size;
            frame
        } else {
            warning!("Allocating new RenderFrame for GUI");
            RenderFrame::new(size)
        }
    }

    pub fn clear_render_commands(&mut self) {
        self.cached_gui_commands = None;
        self.render_commands.pop_back_and_discard();
    }

    fn acquire_upload_buffer(&mut self, required_size: usize) -> Rc<MemoryBuffer> {
        let buffer = self
            .allocator
            .create_memory_buffer::<u8>(
                required_size,
                buffer::MemoryUsage::TRANSFER_SRC,
                buffer::MemoryAccess::WRITE,
            )
            .unwrap();
        Rc::new(buffer)
    }

    fn drain_released_buffers(&mut self) {
        while let Some(buffer_id) = self.released_buffers.pop_front() {
            warning!("Releasing buffer: {}", buffer_id);

            self.memory_buffers.remove(&buffer_id);
            self.memory_buffers_deleted.insert(buffer_id);
            self.pending_buffer_uploads
                .retain(|upload| upload.buffer_id != buffer_id);

            self.pending_buffers.retain(|(id, _)| *id != buffer_id);
        }
    }

    fn stage_incoming_buffers(&mut self) {
        let mut keep_going = true;
        while keep_going {
            if let Some((_buffer_id, io_buffer)) = self.pending_buffers.pop_front() {
                let upload_buffer = self.acquire_upload_buffer(io_buffer.size);
                upload_buffer.write(&io_buffer.data);
            }

            keep_going = false;
        }
    }

    fn drain_released_images(&mut self) {
        while let Some(image_id) = self.released_images.pop_front() {
            warning!("Releasing image: {}", image_id);

            self.pixel_buffers.remove(&image_id);
            self.pixel_buffers_deleted.insert(image_id);
            // Also remove from pending uploads if not yet staged
            self.pending_image_uploads
                .retain(|upload| upload.image_id != image_id);

            // Also prune the pending images mailbox
            self.pending_images.retain(|(id, _)| *id != image_id);
        }
    }

    fn stage_incoming_images(&mut self) {
        // Process pending uploads up to max_uploads_per_frame
        let mut uploads_processed = 0;
        while uploads_processed < self.max_uploads_per_frame {
            if let Some((image_id, io_image)) = self.pending_images.pop_front() {
                let upload_buffer = self.acquire_upload_buffer(io_image.pixels.len());
                upload_buffer.write(&io_image.pixels);

                self.ensure_pixel_buffer_for(image_id, &io_image);

                if let Some(target_buffer) = self.pixel_buffers.get(&image_id) {
                    self.pending_image_uploads.push_back(ImageUpload {
                        source: upload_buffer,
                        destination: Rc::clone(target_buffer),
                        image_id,
                    });
                }
                uploads_processed += 1;
            } else {
                break;
            }
        }
    }

    /// Ensures a pixel buffer exists for the given image ID and matches the image's properties.
    fn ensure_pixel_buffer_for(&mut self, image_id: u32, io_image: &io::Image) {
        if let Some(pb) = self.pixel_buffers.get(&image_id)
            && pb.width() == io_image.width
            && pb.height() == io_image.height
            && pb.format() == io_image.format
        {
            return;
        }

        if image_id == INVALID_IMAGE_ID {
            error!("Attempted to create pixel buffer for invalid image ID");
            return;
        }

        // TODO: This is here to prevent errors on the client side,
        //       especially around dynamic images. Maybe remove?
        if self.pixel_buffers_deleted.contains(&image_id) {
            error!(
                "Attempted to create pixel buffer for deleted image ID: {}",
                image_id
            );

            // Prevent the set from growing indefinitely:
            if self.pixel_buffers_deleted.len() > 1000 {
                error!("pixel_buffers_deleted exceeded 1000 entries!");
                self.pixel_buffers_deleted.clear();
            }

            return;
        }

        let pixel_buffer = self
            .allocator
            .create_pixel_buffer(
                io_image.format,
                io_image.width,
                io_image.height,
                ImageUsage::SAMPLED
                    | ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_DST
                    | ImageUsage::TRANSFER_SRC,
                ImageAspect::COLOR,
                Samples::X1,
            )
            .unwrap();

        self.pixel_buffers.insert(image_id, Rc::new(pixel_buffer));
    }

    fn preprocess_render_frame(&mut self, frame: &mut RenderFrame) {
        for desc in &frame.image_descriptors {
            let pixel_buffer = self.pixel_buffers.get(&desc.image_id);
            let (img_w, img_h) = if let Some(pb) = pixel_buffer {
                (pb.width().max(1) as f32, pb.height().max(1) as f32)
            } else {
                // Fallback or default size
                (1.0, 1.0)
            };

            let (positions, uvs) = compute_fit(img_w, img_h, &desc.layout, desc.fit);

            let offset = desc.vertex_offset as usize;
            if offset + 3 < frame.vertices.len() {
                for i in 0..4 {
                    let gpu_vertex = Gpu2DVertex {
                        position: positions[i],
                        texture: uvs[i],
                        color: desc.color,
                    };
                    frame.vertices[offset + i] = gpu_vertex;
                }
            }
        }
    }

    fn render_empty(&mut self) {
        self.queue.enqueue_present(|cmd, present_image| {
            // Handle all pending uploads to their target buffers
            for upload in &self.pending_image_uploads {
                cmd.layout_barrier(&upload.destination, Layout::TRANSFER_DST);
                cmd.copy_buffer_to_image(&upload.source, &upload.destination);

                cmd.layout_barrier(&upload.destination, Layout::SHADER_READ_ONLY);
            }
            self.pending_image_uploads.clear();

            if let Some(target_frame) = self.target_frame.as_ref() {
                cmd.layout_barrier(&target_frame, Layout::TRANSFER_SRC);
                cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::TRANSFER_DST);
                cmd.blit_to_present(&target_frame, present_image, Filter::Linear);
                cmd.present_image_barrier(
                    present_image,
                    Layout::TRANSFER_DST,
                    Layout::PRESENT_SRC_KHR,
                );
            }
        });

        self.frame_skips += 1;
        self.next_frame();
    }

    pub fn render(&mut self) {
        let frame_start = Instant::now();

        // Perform resource management tasks
        self.drain_released_buffers();
        self.drain_released_images();
        self.stage_incoming_images();

        // Grab a reference to the target frame for rendering, if available.
        // It might not be available if we have no presenter (yet).
        // If not available, render an empty frame.
        let target_frame = match self.target_frame.as_ref() {
            Some(frame) => Rc::clone(frame),
            None => {
                self.render_empty();
                self.frame_cpu_times
                    .push(Instant::now().duration_since(frame_start).as_secs_f64());
                return;
            }
        };

        // Take the latest incoming overlay frame, return any older frames to the pool.
        if let Some(newest) = self.render_commands.pop_back() {
            const FRAME_POOL_CAPACITY: usize = 3;

            while let Some(mut old) = self.render_commands.pop_back() {
                if self.render_frame_pool.len() < FRAME_POOL_CAPACITY {
                    // Avoid stale data in the pool.
                    old.clear();
                    self.render_frame_pool.push(old);
                }
            }

            // Cache the newest frame for rendering, returning any previous frame to the pool.
            if let Some(mut current_cached) = self.cached_gui_commands.replace(newest) {
                if self.render_frame_pool.len() < FRAME_POOL_CAPACITY {
                    // Avoid stale data in the pool.
                    current_cached.clear();
                    self.render_frame_pool.push(current_cached);
                }
            }
        } else if self.pending_image_uploads.is_empty() && !self.target_damaged {
            // No work to do, present the previous frame, if available.
            self.render_empty();
            self.frame_cpu_times
                .push(Instant::now().duration_since(frame_start).as_secs_f64());
            return;
        }

        // Grab the cached gui commands for rendering.
        // It might not be available if no commands have been issued.
        // If not available, render an empty frame.
        // We take() here and return it back at the end of this render call.
        let mut gui_commands = match self.cached_gui_commands.take() {
            Some(f) => f,
            None => {
                self.render_empty();
                self.frame_cpu_times
                    .push(Instant::now().duration_since(frame_start).as_secs_f64());
                return;
            }
        };

        // Perform preprocessing on the render frame
        self.preprocess_render_frame(&mut gui_commands);

        // Allocate and upload vertex and index data.
        // This will rarely actually allocate anything,
        // as buffers are allocated from VMA internal pools.
        let vertex_count = gui_commands.vertices.len().max(1);
        let vertex_buffer = Rc::new(
            self.allocator
                .create_memory_buffer::<Gpu2DVertex>(
                    vertex_count,
                    buffer::MemoryUsage::VERTEX_BUFFER,
                    buffer::MemoryAccess::WRITE,
                )
                .unwrap(),
        );
        if !gui_commands.vertices.is_empty() {
            vertex_buffer.write(&gui_commands.vertices);
        }
        let index_count = gui_commands.indices.len().max(1);
        let index_buffer = Rc::new(
            self.allocator
                .create_memory_buffer::<u32>(
                    index_count,
                    buffer::MemoryUsage::INDEX_BUFFER,
                    buffer::MemoryAccess::WRITE,
                )
                .unwrap(),
        );
        if !gui_commands.indices.is_empty() {
            index_buffer.write(&gui_commands.indices);
        }

        self.queue.enqueue(|cmd| {
            // Perform pending image uploads
            for image_upload in &self.pending_image_uploads {
                cmd.layout_barrier(&image_upload.destination, Layout::TRANSFER_DST);
                cmd.copy_buffer_to_image(&image_upload.source, &image_upload.destination);
                cmd.layout_barrier(&image_upload.destination, Layout::SHADER_READ_ONLY);
            }
            self.pending_image_uploads.clear();

            cmd.layout_barrier(&target_frame, Layout::TRANSFER_DST);
            cmd.clear_pixel_buffer(&target_frame, &[0.0, 0.0, 0.0, 0.0]);
            cmd.layout_barrier(&target_frame, Layout::COLOR_ATTACHMENT);

            let render_area = Rectangle {
                x: 0.0,
                y: 0.0,
                w: target_frame.width() as f32,
                h: target_frame.height() as f32,
            };

            cmd.begin_rendering(&render_area, &[&target_frame]);
            cmd.bind_pipeline(&self.test_data.test_pipeline);
            cmd.bind_vertex_buffer(&vertex_buffer);
            cmd.bind_index_buffer(&index_buffer);
            cmd.set_viewport(&render_area);

            let push_constants = PerCommand {
                scale: Vector2f::new(
                    2.0 / target_frame.width() as f32,
                    2.0 / target_frame.height() as f32,
                ),
            };
            cmd.push_constants(
                &self.test_data.test_pipeline,
                Stage::VERTEX | Stage::FRAGMENT,
                &push_constants,
            );

            // Execute draw batches if frame is present
            for batch in &gui_commands.batches {
                for draw_cmd in &batch.commands {
                    let scissor = draw_cmd.scissor.unwrap_or(render_area);
                    cmd.set_scissor(&scissor);

                    let mut sampler = &self.test_data.linear_sampler;

                    // In short, image_id being Some indicates intention that this is
                    // a textured draw. If no image with that id exists (or otherwise invalid),
                    // skip draw.
                    let texture = if let Some(image_id) = draw_cmd.image_id {
                        self.pixel_buffers
                            .get(&image_id)
                            .and_then(|t| t.is_defined().then_some(t))
                    } else {
                        sampler = &self.test_data.nearest_sampler;
                        self.pixel_buffers
                            .get(&GEOMETRY_TEXTURE_ID)
                            .and_then(|t| t.is_defined().then_some(t))
                    };

                    if let Some(texture) = texture {
                        cmd.push_pixel_descriptor(
                            &self.test_data.test_pipeline,
                            0,
                            texture,
                            1,
                            sampler,
                        );
                        cmd.draw_indexed(draw_cmd.index_count, 1, draw_cmd.index_offset, 0, 0);
                    }
                }
            }

            cmd.end_rendering();
        });

        self.queue.enqueue_present(|cmd, present_image| {
            cmd.layout_barrier(&target_frame, Layout::TRANSFER_SRC);
            cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::TRANSFER_DST);
            cmd.blit_to_present(&target_frame, present_image, Filter::Linear);
            cmd.present_image_barrier(present_image, Layout::TRANSFER_DST, Layout::PRESENT_SRC_KHR);
            cmd.layout_barrier(&target_frame, Layout::TRANSFER_DST);
        });

        let frame_end = Instant::now();
        let frame_duration = frame_end.duration_since(frame_start).as_secs_f64();
        self.frame_cpu_times.push(frame_duration);

        self.cached_gui_commands = Some(gui_commands);
        self.target_damaged = false;

        self.next_frame();
    }
}

#[derive(Debug, Clone)]
pub enum DeviceSelector {
    /// Select the device with the best performance characteristics.
    Optimal,
    /// Select a specific device by its index in the enumerated device list.
    ByIndex(usize),
    /// Select a device by its UUID.
    ById(u128),
    /// Select a device by its LUID (Windows-specific identifier).
    ByLuid(u64),
    /// Select a device by its exact name.
    ByName(String),
}

/// Builder for creating and configuring a `Gfx` graphics instance.
#[derive(Debug)]
pub struct GfxBuilder {
    params: Parameters,
}

impl GfxBuilder {
    /// Creates a new `GfxBuilder` with the given platform.
    pub fn new(platform: Platform) -> Self {
        Self {
            params: Parameters {
                debugging: false,
                platform,
                app_version: Version::new(0, 1, 0),
                app_name: String::from("rustine"),
                device_selector: DeviceSelector::Optimal,
            },
        }
    }

    pub fn debugging(mut self, enabled: bool) -> Self {
        self.params.debugging = enabled;
        self
    }

    pub fn app_version(mut self, version: Version) -> Self {
        self.params.app_version = version;
        self
    }

    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.params.app_name = name.into();
        self
    }

    pub fn device_selector(mut self, selector: DeviceSelector) -> Self {
        self.params.device_selector = selector;
        self
    }

    /// Builds the `Gfx` instance.
    pub fn build(self) -> Result<Gfx> {
        // 1. Create Vulkan instance
        let vk_instance = Instance::new(&self.params)?;

        // 2. Select physical device
        let selected_device = {
            let devices = vk_instance.enumerate_physical_devices()?;

            let pick_type_score = |t: &PhysicalDeviceType| -> i32 {
                match t {
                    PhysicalDeviceType::Discrete => 3,
                    PhysicalDeviceType::Integrated => 2,
                    PhysicalDeviceType::Virtual => 1,
                    PhysicalDeviceType::Cpu => 0,
                    PhysicalDeviceType::Other => 0,
                }
            };

            let selected = match &self.params.device_selector {
                DeviceSelector::Optimal => devices
                    .into_iter()
                    .max_by_key(|d| (pick_type_score(&d.device_type), d.api, d.driver)),
                DeviceSelector::ByIndex(i) => devices.into_iter().nth(*i),
                DeviceSelector::ById(id) => devices.into_iter().find(|d| d.id == *id),
                DeviceSelector::ByLuid(luid) => devices.into_iter().find(|d| d.luid == *luid),
                DeviceSelector::ByName(name) => devices.into_iter().find(|d| d.name == *name),
            };

            match selected {
                Some(d) => d,
                None => return Err(Status::NotSupported(-1)),
            }
        };

        // 3. Create logical device
        let vk_device = Rc::new(Device::new(&self.params, selected_device)?);

        // 4. Create VMA
        let allocator = allocator::Allocator::new(&vk_instance, Rc::clone(&vk_device))?;

        Ok(Gfx::new(vk_instance, vk_device, allocator))
    }
}
