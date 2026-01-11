use crate::gfx::pipeline::*;
use crate::{Parameters, RingBuffer, gfx::queue::Queue, gfx::*, io, warning};

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

const FALLBACK_TEXTURE_ID: u32 = u32::MAX;

struct TestData {
    target_frame: Option<Arc<PixelBuffer>>,
    test_vertex_buffer: Arc<MemoryBuffer>,
    test_index_buffer: Arc<MemoryBuffer>,
    pending_uploads: VecDeque<PendingUpload>,
    linear_sampler: Arc<Sampler>,
    nearest_sampler: Arc<Sampler>,
    test_pipeline: Arc<Pipeline>,
    fallback_texture: Arc<PixelBuffer>,
}

struct PendingUpload {
    buffer: Arc<MemoryBuffer>,
    target: Arc<PixelBuffer>,
    image_id: u32,
}

#[repr(C)]
struct PerCommand {
    scale: Vector2f,
    is_sdf: bool,
    sdf_range: f32,
}

/// The core graphics subsystem, managing Vulkan initialization and device selection.
///
/// `Core` encapsulates a Vulkan instance and a selected physical device.
/// It is responsible for creating and maintaining the graphics pipeline.
/// `Core` is thread-safe and can be shared across threads.
pub struct Gfx {
    test_data: TestData,
    pending_images: Arc<Mutex<VecDeque<(u32, io::Image)>>>,
    pending_image_uploads: VecDeque<(u32, io::Image)>,
    render_commands: Arc<Mutex<VecDeque<RenderFrame>>>,
    cached_render_frame: Option<RenderFrame>,
    next_image_id: u32,
    pixel_buffers: HashMap<u32, Arc<PixelBuffer>>,
    released_images: Arc<Mutex<VecDeque<u32>>>,
    queue: Queue,
    allocator: Arc<allocator::Allocator>,
    device: Arc<Device>,
    instance: Instance,
    frame_n: u64,
    frame_cpu_times: RingBuffer<f64>,
    max_uploads_per_frame: usize,
    work_available: Arc<(Mutex<bool>, Condvar)>,
}

// SAFETY: Core manages a Vulkan instance which can be safely shared and accessed across threads.
// The Vulkan instance itself is thread-safe for most operations.
unsafe impl Send for Gfx {}
unsafe impl Sync for Gfx {}

impl Drop for Gfx {
    fn drop(&mut self) {
        warning!("Gfx::drop");
    }
}

impl Gfx {
    /// Signals that work is available, waking up the GFX thread if it's waiting
    /// in event-driven mode. This is a no-op in continuous mode.
    pub fn signal_work_available(&self) {
        let (work_flag, condvar) = &*self.work_available;
        if let Ok(mut flag) = work_flag.lock() {
            *flag = true;
            drop(flag);
            condvar.notify_one();
        }
    }

    pub fn run(
        gfx: Arc<Mutex<gfx::Gfx>>,
        exit_flag: &std::sync::atomic::AtomicBool,
        mode: LoopMode,
    ) {
        log::set_current_thread_name("gfx");

        info!("GFX START");

        const TARGET_FPS: f64 = 120.0;
        let target_frame_time = std::time::Duration::from_secs_f64(1.0 / TARGET_FPS);
        const SPIN_THRESHOLD: std::time::Duration = std::time::Duration::from_micros(500);

        let start_instant = std::time::Instant::now();
        let mut last_instant = std::time::Instant::now();
        let mut last_stat_instant = std::time::Instant::now();
        let mut frame_delta_times: RingBuffer<f64> = RingBuffer::new(120);

        loop {
            // Check for exit signal
            if exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            // Handle event-driven mode: wait for work notification
            if let LoopMode::Event = mode {
                // Clone notifier without holding the core (gfx) lock while waiting
                let work_notifier = {
                    let core = gfx.lock().unwrap();
                    std::sync::Arc::clone(&core.work_available)
                };

                let (work_flag, condvar) = &*work_notifier;
                let mut work_ready = work_flag.lock().unwrap();

                // Wait until work is available or exit flag is set
                while !*work_ready && !exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    work_ready = condvar.wait(work_ready).unwrap();
                }

                // Reset the work flag after waking up
                *work_ready = false;
                drop(work_ready);

                if exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
            }

            let frame_start = std::time::Instant::now();

            {
                let mut core = gfx.lock().unwrap();

                let now = std::time::Instant::now();
                let _t = now.duration_since(start_instant).as_secs_f64();
                let _dt = now.duration_since(last_instant).as_secs_f32();
                frame_delta_times.push(_dt as f64);
                core.render();
                last_instant = now;

                let stat_now = std::time::Instant::now();
                if stat_now.duration_since(last_stat_instant).as_secs_f64() >= 1.0 {
                    let frame_cpu_times = core.frame_cpu_times();
                    if let Some((min, max, mean)) = frame_cpu_times.min_max_mean() {
                        debug!(
                            "GFX frame CPU -> min: {}, max: {}, mean: {}",
                            utilities::format_duration(min),
                            utilities::format_duration(max),
                            utilities::format_duration(mean)
                        );
                    }
                    if let Some((min, max, mean)) = frame_delta_times.min_max_mean() {
                        debug!(
                            "GFX frame dt -> min: {}, max: {}, mean: {}",
                            utilities::format_duration(min as f64),
                            utilities::format_duration(max as f64),
                            utilities::format_duration(mean as f64)
                        );
                    }

                    let current = alloc::current_bytes();
                    let peak = alloc::peak_bytes();
                    let vram = gfx::Gfx::current_allocated_vram_bytes();
                    let rss = alloc::rss_bytes();
                    match rss {
                        Some(rss_b) => debug!(
                            "Memory -> {}, peak: {} | VRAM: {} | RAM: {}",
                            utilities::format_bytes_iec(current),
                            utilities::format_bytes_iec(peak),
                            utilities::format_bytes_iec(vram),
                            utilities::format_bytes_iec(rss_b)
                        ),
                        None => debug!(
                            "Memory -> {}, peak: {} | VRAM: {}",
                            utilities::format_bytes_iec(current),
                            utilities::format_bytes_iec(peak),
                            utilities::format_bytes_iec(vram),
                        ),
                    }
                    last_stat_instant = stat_now;
                }
            }

            // Frame rate limiting only in continuous mode
            if let LoopMode::Continuous = mode {
                let elapsed = frame_start.elapsed();
                if elapsed < target_frame_time {
                    let mut remaining = target_frame_time - elapsed;

                    // Sleep for bulk of remaining time
                    while remaining > SPIN_THRESHOLD {
                        std::thread::sleep(std::time::Duration::from_millis(1));
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
        device: Arc<Device>,
        allocator: Arc<allocator::Allocator>,
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

        let mut test_shader_program: ShaderProgram = ShaderProgram::new("test_shader_program");
        let test_shader_compiler = Compiler::new().unwrap();
        let test_shader_stage = test_shader_compiler
            .compile(Stage::VERTEX, OVERLAY_SHADER)
            .unwrap();
        test_shader_program.add_stage(test_shader_stage);
        let test_shader_stage = test_shader_compiler
            .compile(Stage::FRAGMENT, OVERLAY_SHADER)
            .unwrap();
        test_shader_program.add_stage(test_shader_stage);

        /*let test_pipeline_params = pipeline::Parameters {
            shader: test_shader_program,
            topology: Topology::Triangles,
            winding: Winding::CounterClockwise,
            culling: Culling::None,
            raster: Raster::Fill,
            samples: Samples::X1,
            depth_comparison: Comparison::Always,
            depth_write: false,
            depth_test: false,
            bindings: vec![],
            attributes: vec![],
            push_constants: vec![],
            descriptors: vec![
                Descriptor::new(0, DescriptorType::SampledImage, Stage::Fragment),
                Descriptor::new(1, DescriptorType::Sampler, Stage::Fragment),
            ],
            attachments: vec![Attachment::new(
                Format::B8G8R8A8_UNORM,
                AttachmentBlend::straight_alpha_blend(),
            )],
        };*/
        let test_pipeline_params = pipeline::Parameters {
            shader: test_shader_program,
            topology: Topology::Triangles,
            winding: Winding::CounterClockwise,
            culling: Culling::None,
            raster: Raster::Fill,
            samples: Samples::X1,
            depth_comparison: Comparison::Always,
            depth_write: false,
            depth_test: false,
            bindings: vec![Binding {
                binding: 0,
                stride: std::mem::size_of::<GpuVertex>() as u32,
                rate: Rate::VERTEX,
            }],
            attributes: vec![
                Attribute {
                    binding: 0,
                    location: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: std::mem::offset_of!(GpuVertex, position) as u32,
                },
                Attribute {
                    binding: 0,
                    location: 1,
                    format: Format::R32G32_SFLOAT,
                    offset: std::mem::offset_of!(GpuVertex, texture) as u32,
                },
                Attribute {
                    binding: 0,
                    location: 2,
                    format: Format::U32,
                    offset: std::mem::offset_of!(GpuVertex, color) as u32,
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
        let test_pipeline = Pipeline::new(device.clone(), &test_pipeline_params).unwrap();

        let test_vertex_buffer = allocator
            .create_memory_buffer(
                1024 * 1024, // 1MB for vertices
                buffer::MemoryUsage::VERTEX_BUFFER,
                buffer::MemoryAccess::READ_WRITE,
            )
            .unwrap();

        let test_index_buffer = allocator
            .create_memory_buffer(
                1024 * 1024, // 1MB for indices
                buffer::MemoryUsage::INDEX_BUFFER,
                buffer::MemoryAccess::READ_WRITE,
            )
            .unwrap();

        // Create fallback texture: 1x1 white pixel
        let fallback_pixel_data = [0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8]; // RGBA white
        let fallback_upload = allocator
            .create_memory_buffer(
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
        let fallback_texture = Arc::new(fallback_texture_buffer);

        // Set up fallback texture to be uploaded in first render
        let fallback_texture_clone = Arc::clone(&fallback_texture);
        let pending_fallback = PendingUpload {
            buffer: Arc::new(fallback_upload),
            target: fallback_texture_clone,
            image_id: FALLBACK_TEXTURE_ID,
        };

        let mut pending_uploads = VecDeque::new();
        pending_uploads.push_back(pending_fallback);

        let mut pixel_buffers = HashMap::new();
        pixel_buffers.insert(FALLBACK_TEXTURE_ID, Arc::clone(&fallback_texture));

        // Create all available font atlases
        for font_id in fonts::ALL_FONT_IDS {
            if let Some(font_data) = fonts::get_font_atlas(*font_id) {
                let font_upload = allocator
                    .create_memory_buffer(
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
                let font_texture = Arc::new(font_texture_buffer);

                pixel_buffers.insert(font_data.texture_id, Arc::clone(&font_texture));

                let font_texture_clone = Arc::clone(&font_texture);
                let pending_font = PendingUpload {
                    buffer: Arc::new(font_upload),
                    target: font_texture_clone,
                    image_id: font_data.texture_id,
                };
                pending_uploads.push_back(pending_font);
            }
        }

        let test_data = TestData {
            target_frame: None,
            linear_sampler: Arc::new(linear_sampler),
            nearest_sampler: Arc::new(nearest_sampler),
            test_pipeline: Arc::new(test_pipeline),
            test_vertex_buffer: Arc::new(test_vertex_buffer),
            test_index_buffer: Arc::new(test_index_buffer),
            pending_uploads,
            fallback_texture,
        };

        let command_queue = Queue::new(&device, 4);

        Gfx {
            instance,
            device: device,
            allocator,
            test_data,
            frame_n: 0,
            queue: command_queue,
            pixel_buffers,
            pending_images: Arc::new(Mutex::new(VecDeque::new())),
            pending_image_uploads: VecDeque::new(),
            render_commands: Arc::new(Mutex::new(VecDeque::new())),
            cached_render_frame: None,
            next_image_id: 0,
            released_images: Arc::new(Mutex::new(VecDeque::new())),
            frame_cpu_times: RingBuffer::new(120),
            max_uploads_per_frame: 4,
            work_available: Arc::new((Mutex::new(false), Condvar::new())),
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
        &self.device.physical_device()
    }

    pub fn vulkan_instance_handle(&self) -> vulkan::VkInstance {
        self.instance.handle()
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn allocator(&self) -> &Arc<allocator::Allocator> {
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
        self.test_data.target_frame = Some(Arc::new(target_frame));

        self.render();
    }

    pub fn create_pipeline(&self, parameters: pipeline::Parameters) -> Pipeline {
        Pipeline::new(self.device.clone(), &parameters).unwrap()
    }

    pub fn create_image(&mut self, io_image: io::Image) -> Image {
        let image_id = self.next_image_id;
        self.next_image_id += 1;

        let image = Image::new(
            image_id,
            io_image.width,
            io_image.height,
            Arc::downgrade(&self.released_images),
        );

        self.create_pixel_buffer_for(image_id, &io_image);
        self.pending_image_uploads.push_back((image_id, io_image));

        image
    }

    pub fn create_dynamic_image(&mut self) -> Image {
        let image_id = self.next_image_id;
        self.next_image_id += 1;

        Image::new(image_id, 0, 0, Arc::downgrade(&self.released_images))
    }

    pub fn image_mailbox(&self) -> Arc<Mutex<VecDeque<(u32, io::Image)>>> {
        Arc::clone(&self.pending_images)
    }

    pub fn submit_image(&self, image_id: u32, io_image: io::Image) {
        if let Ok(mut pending) = self.pending_images.lock() {
            pending.push_back((image_id, io_image));
        }
    }

    pub fn render_mailbox(&self) -> Arc<Mutex<VecDeque<RenderFrame>>> {
        Arc::clone(&self.render_commands)
    }

    pub fn submit_render_frame(&self, frame: RenderFrame) {
        if let Ok(mut pending) = self.render_commands.lock() {
            pending.push_back(frame);
        }
    }

    pub fn clear_render_commands(&mut self) {
        self.cached_render_frame = None;
        if let Ok(mut pending) = self.render_commands.lock() {
            pending.clear();
        }
    }

    fn acquire_upload_buffer(&mut self, required_size: usize) -> Arc<MemoryBuffer> {
        let buffer = self
            .allocator
            .create_memory_buffer(
                required_size,
                buffer::MemoryUsage::TRANSFER_SRC,
                buffer::MemoryAccess::WRITE,
            )
            .unwrap();
        Arc::new(buffer)
    }

    fn drain_released_images(&mut self) {
        if let Ok(mut released) = self.released_images.lock() {
            while let Some(image_id) = released.pop_front() {
                warning!("Releasing image: {}", image_id);

                self.pixel_buffers.remove(&image_id);
                // Also remove from pending uploads if not yet staged
                self.pending_image_uploads.retain(|(id, _)| *id != image_id);
            }
        }
    }

    fn stage_incoming_images(&mut self) {
        // Consume paired image submissions and stage uploads
        if let Some((image_id, io_image)) = {
            let mut pending = self.pending_images.lock().unwrap();
            pending.pop_front()
        } {
            self.create_pixel_buffer_for(image_id, &io_image);
            self.pending_image_uploads.push_back((image_id, io_image));
        }

        // Process pending uploads up to max_uploads_per_frame
        let mut uploads_processed = 0;
        while uploads_processed < self.max_uploads_per_frame {
            if let Some((image_id, io_image)) = self.pending_image_uploads.pop_front() {
                let upload_buffer = self.acquire_upload_buffer(io_image.pixels.len());
                upload_buffer.write(&io_image.pixels);

                if let Some(target_buffer) = self.pixel_buffers.get(&image_id) {
                    self.test_data.pending_uploads.push_back(PendingUpload {
                        buffer: upload_buffer,
                        target: Arc::clone(target_buffer),
                        image_id,
                    });
                }
                uploads_processed += 1;
            } else {
                break;
            }
        }
    }

    fn create_pixel_buffer_for(&mut self, image_id: u32, io_image: &io::Image) {
        // Check if we already have a pixel buffer with the same dimensions and format
        if self
            .pixel_buffers
            .iter()
            .find(|(id, pb)| {
                *id == &image_id
                    && pb.width() == io_image.width
                    && pb.height() == io_image.height
                    && pb.format() == io_image.format
            })
            .is_some()
        {
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

        let pixel_buffer = Arc::new(pixel_buffer);
        self.pixel_buffers.insert(image_id, pixel_buffer);
    }

    fn preprocess_render_frame(&self, frame: &mut RenderFrame) {
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
                    frame.vertices[offset + i].position = positions[i];
                    frame.vertices[offset + i].texture = uvs[i];
                    frame.vertices[offset + i].color = desc.color;
                }
            }
        }
    }

    fn render_empty(&mut self) {
        self.queue.enqueue_present(move |cmd, present_image| {
            cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::PRESENT_SRC_KHR);
        });
    }

    pub fn render(&mut self) {
        let frame_start = std::time::Instant::now();

        // Drain any released images before staging new ones
        self.drain_released_images();

        let target_frame = match self.test_data.target_frame.as_ref() {
            Some(frame) => Arc::clone(frame),
            None => {
                self.render_empty();
                return;
            }
        };

        self.stage_incoming_images();

        // Check for new render commands; if present, cache them and use; otherwise use cached frame
        {
            if let Ok(mut pending) = self.render_commands.lock() {
                if let Some(frame) = pending.pop_back() {
                    pending.clear();
                    self.cached_render_frame = Some(frame);
                }
            }
        }

        let mut frame = match self.cached_render_frame.clone() {
            Some(f) => f,
            None => {
                self.render_empty();
                return;
            }
        };

        // Preprocess: update vertices for images with dynamic fitting based on actual pixel buffer sizes
        self.preprocess_render_frame(&mut frame);

        // Update vertex and index buffers with pre-computed data from UI
        if !frame.vertices.is_empty() {
            self.test_data.test_vertex_buffer.write(&frame.vertices);
        }
        if !frame.indices.is_empty() {
            self.test_data.test_index_buffer.write(&frame.indices);
        }

        let pending_uploads = std::mem::take(&mut self.test_data.pending_uploads);
        let test_pipeline = Arc::clone(&self.test_data.test_pipeline);
        let test_vertex_buffer = Arc::clone(&self.test_data.test_vertex_buffer);
        let test_index_buffer = Arc::clone(&self.test_data.test_index_buffer);
        let linear_sampler = Arc::clone(&self.test_data.linear_sampler);
        let nearest_sampler = Arc::clone(&self.test_data.nearest_sampler);
        let cached_frame = Some(frame);
        let target_frame_clone = Arc::clone(&target_frame);
        let pixel_buffers = self.pixel_buffers.clone();

        self.queue.enqueue(move |cmd| {
            // Handle all pending uploads to their target buffers
            for upload in &pending_uploads {
                cmd.layout_barrier(&upload.target, Layout::TRANSFER_DST);
                cmd.copy_buffer_to_image(&upload.buffer, &upload.target);

                cmd.layout_barrier(&upload.target, Layout::SHADER_READ_ONLY);
            }

            cmd.layout_barrier(&target_frame, Layout::TRANSFER_DST);

            cmd.clear_pixel_buffer(&target_frame, &[0f32, 0f32, 0f32, 0f32]);

            cmd.layout_barrier(&target_frame, Layout::COLOR_ATTACHMENT);

            let render_area = Rectangle {
                x: 0.0,
                y: 0.0,
                w: target_frame.width() as f32,
                h: target_frame.height() as f32,
            };

            cmd.begin_rendering(&render_area, &[&target_frame]);
            cmd.bind_pipeline(&test_pipeline);
            cmd.bind_vertex_buffer(&test_vertex_buffer);
            cmd.bind_index_buffer(&test_index_buffer);
            cmd.set_viewport(&render_area);

            let push_constants = PerCommand {
                scale: Vector2f::new(
                    2f32 / target_frame.width() as f32,
                    2f32 / target_frame.height() as f32,
                ),
                sdf_range: 1f32,
                is_sdf: false,
            };
            cmd.push_constants(
                &test_pipeline,
                Stage::VERTEX | Stage::FRAGMENT,
                &push_constants,
            );

            // Execute draw batches if frame is present
            if let Some(frame) = &cached_frame {
                for batch in &frame.batches {
                    for draw_cmd in &batch.commands {
                        let scissor = draw_cmd
                            .scissor
                            .clone()
                            .unwrap_or_else(|| render_area.clone());
                        cmd.set_scissor(&scissor);

                        let mut sampler = linear_sampler.clone();

                        // In short, image_id being Some indicates intention that this is
                        // a textured draw. If no image with that id exists (or otherwise invalid),
                        // try the fallback in the same manner.
                        // Otherwise, use the fallback texture for a pure geometry draw.
                        let texture = if let Some(image_id) = draw_cmd.image_id {
                            pixel_buffers
                                .get(&image_id)
                                .and_then(|t| t.is_defined().then(|| t))
                                .or_else(|| {
                                    draw_cmd.image_fallback_id.and_then(|fallback_id| {
                                        pixel_buffers
                                            .get(&fallback_id)
                                            .and_then(|t| t.is_defined().then(|| t))
                                    })
                                })
                        } else {
                            sampler = nearest_sampler.clone();
                            pixel_buffers
                                .get(&FALLBACK_TEXTURE_ID)
                                .and_then(|t| t.is_defined().then(|| t))
                        };

                        if let Some(texture) = texture {
                            cmd.push_pixel_descriptor(&test_pipeline, 0, texture, 1, &sampler);
                            cmd.draw_indexed(draw_cmd.index_count, 1, draw_cmd.index_offset, 0, 0);
                        }
                    }
                }
            }

            cmd.end_rendering();

            cmd.layout_barrier(&target_frame, Layout::TRANSFER_SRC);
        });

        self.queue.enqueue_present(move |cmd, present_image| {
            cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::TRANSFER_DST);
            cmd.blit_to_present(&target_frame_clone, present_image, Filter::Linear);
            cmd.present_image_barrier(present_image, Layout::TRANSFER_DST, Layout::PRESENT_SRC_KHR);
            target_frame_clone.set_layout(Layout::PRESENT_SRC_KHR);
        });

        let frame_end = std::time::Instant::now();
        let frame_duration = frame_end.duration_since(frame_start).as_secs_f64();
        self.frame_cpu_times.push(frame_duration);

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
        let vk_device = Arc::new(Device::new(&self.params, selected_device)?);

        // 4. Create VMA
        let allocator = allocator::Allocator::new(&vk_instance, Arc::clone(&vk_device))?;

        Ok(Gfx::new(vk_instance, vk_device, allocator))
    }
}

static COMPOSITION_SHADER: &str = r#"
struct fragment_input
{
	float4 Position : SV_POSITION;
	float2 UV : TEXCOORD0;
};

[[vk::binding(0, 0)]] Texture2D commandTexture;
[[vk::binding(1, 0)]] SamplerState commandSampler;

[shader("vertex")]
fragment_input vertex(in uint vertexIndex : SV_VertexID)
{
    fragment_input output = (fragment_input)0;
    output.UV = float2((vertexIndex << 1) & 2, vertexIndex & 2);
    output.Position = float4(output.UV * 2.0f - 1.0f, 0.0f, 1.0f);
	return output;
}

[shader("pixel")]
float4 fragment(fragment_input input) : SV_TARGET
{
	return commandTexture.Sample(commandSampler, input.UV);
    //return float4(input.UV.x, 0.0, input.UV.y, 1.0);
}
"#;

static OVERLAY_SHADER: &str = r#"
struct PerCommand
{
	float2 Scale;
    float sdfRange;
    bool isSdf;
};

[[vk::push_constant]] PerCommand command;

[[vk::binding(0, 0)]] Texture2D commandTexture;
[[vk::binding(1, 0)]] SamplerState commandSampler;

struct vertex_input
{
	float2 Position : POSITION0;
	float2 UV : TEXCOORD0;
	uint Color : COLOR0;
};

struct fragment_input
{
	float4 Position : SV_POSITION;
	float2 UV : TEXCOORD0;
	float4 Color : COLOR0;
};

float4 UnpackColor(uint packed)
{
    float a = (float)(packed & 0xFF) / 255.0f;
    float b = (float)((packed >> 8) & 0xFF) / 255.0f;
    float g = (float)((packed >> 16) & 0xFF) / 255.0f;
    float r = (float)((packed >> 24) & 0xFF) / 255.0f;
    return float4(r, g, b, a);
}

[shader("vertex")]
fragment_input vertex(vertex_input input, in uint vertexIndex : SV_VertexID)
{
    fragment_input output = (fragment_input)0;
    output.Position = float4(input.Position * command.Scale + float2(-1, -1), 0.0, 1.0);
	output.UV = input.UV;
	output.Color = UnpackColor(input.Color);
	return output;
}

[shader("pixel")]
float4 fragment(fragment_input input) : SV_TARGET
{ 
	float4 geometryColor = input.Color;
	float4 textureColor = commandTexture.Sample(commandSampler, input.UV);
    float alpha = textureColor.a;

	//if (command.isSdf)
	//{ 
	//	float sdf = textureColor.a - 0.5;
	//	alpha = smoothstep(-command.sdfRange, +command.sdfRange, sdf);
	//}

	return float4(geometryColor.rgb * textureColor.rgb, alpha * geometryColor.a);
}
"#;
