use crate::{RingBuffer, gfx::queue::Queue, gfx::*, io, warning};

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

struct TestData {
    test_pixel_buffer: Arc<PixelBuffer>,
    test_pixel_format: Format,
    target_frame: Option<Arc<PixelBuffer>>,
    test_vertex_buffer: Arc<MemoryBuffer>,
    test_index_buffer: Arc<MemoryBuffer>,
    upload_pool: Vec<UploadBuffer>,
    upload_cursor: usize,
    pending_upload: Option<PendingUpload>,
    test_sampler: Arc<Sampler>,
    test_pipeline: Arc<Pipeline>,
    test_vertex_data: Vec<GpuVertex>,
    test_index_data: Vec<u32>,
}

struct UploadBuffer {
    buffer: Arc<MemoryBuffer>,
    capacity: usize,
}

#[repr(C)]
struct GpuVertex {
    position: Vector2f,
    texture: Vector2f,
    color: u32,
}

#[repr(C)]
struct PerCommand {
    scale: Vector2f,
    is_sdf: bool,
    sdf_range: f32,
}

struct PendingUpload {
    buffer: Arc<MemoryBuffer>,
    width: u32,
    height: u32,
    format: Format,
}

/// The core graphics subsystem, managing Vulkan initialization and device selection.
///
/// `Core` encapsulates a Vulkan instance and a selected physical device.
/// It is responsible for creating and maintaining the graphics pipeline.
/// `Core` is thread-safe and can be shared across threads.
pub struct Core {
    test_data: TestData,
    pending_images: Arc<Mutex<VecDeque<io::Image>>>,
    render_commands: Arc<Mutex<VecDeque<RenderFrame>>>,
    cached_render_frame: Option<RenderFrame>,
    next_image_id: u32,
    pixel_buffers: HashMap<u32, Option<PixelBuffer>>,
    queue: Queue,
    allocator: Arc<allocator::Allocator>,
    device: Arc<Device>,
    instance: Instance,
    frame_n: u64,
    frame_cpu_times: RingBuffer<f64>,
}

// SAFETY: Core manages a Vulkan instance which can be safely shared and accessed across threads.
// The Vulkan instance itself is thread-safe for most operations.
unsafe impl Send for Core {}
unsafe impl Sync for Core {}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");
    }
}

impl Core {
    pub fn new(
        instance: Instance,
        device: Arc<Device>,
        allocator: Arc<allocator::Allocator>,
    ) -> Self {
        // Load data from /home/jant/pictures/hmm/0yzhnmy0.webp
        let webp_raw_data = std::fs::read("/home/jant/pictures/hmm/0yzhnmy0.webp")
            .expect("Failed to load WebP file");
        let mut webp_decoder =
            crate::io::webp::WebPDecoder::new(&webp_raw_data).expect("Failed to decode WebP file");
        let webp_width = webp_decoder.width();
        let webp_height = webp_decoder.height();
        let mut webp_frame_data = vec![0u8; (webp_width * webp_height * 4) as usize];
        let _ = webp_decoder
            .next_frame(&mut webp_frame_data)
            .expect("Failed to decode WebP frame");

        const UPLOAD_POOL_SIZE: usize = 3;
        let mut upload_pool = Vec::with_capacity(UPLOAD_POOL_SIZE);
        for _ in 0..UPLOAD_POOL_SIZE {
            let buffer = allocator
                .create_memory_buffer(
                    webp_frame_data.len(),
                    buffer::MemoryUsage::TRANSFER_SRC,
                    buffer::MemoryAccess::WRITE,
                )
                .unwrap();
            upload_pool.push(UploadBuffer {
                capacity: webp_frame_data.len(),
                buffer: Arc::new(buffer),
            });
        }
        upload_pool[0].buffer.write(&webp_frame_data);

        let test_pixel_buffer = allocator
            .create_pixel_buffer(
                Format::R8G8B8A8_UNORM,
                webp_width,
                webp_height,
                ImageUsage::SAMPLED
                    | ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_DST
                    | ImageUsage::TRANSFER_SRC,
                ImageAspect::COLOR,
                Samples::X1,
            )
            .unwrap();

        let test_sampler = Sampler::new(
            device.clone(),
            Filter::Linear,
            SamplerAddressMode::Repeat,
            SamplerBorderColor::FloatOpaqueBlack,
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
                Format::B8G8R8A8_UNORM,
                AttachmentBlend::straight_alpha_blend(),
            )],
        };
        let test_pipeline = Pipeline::new(device.clone(), &test_pipeline_params).unwrap();

        let test_vertex_buffer = allocator
            .create_memory_buffer(
                1024,
                buffer::MemoryUsage::VERTEX_BUFFER,
                buffer::MemoryAccess::READ_WRITE,
            )
            .unwrap();

        let test_index_buffer = allocator
            .create_memory_buffer(
                1024,
                buffer::MemoryUsage::INDEX_BUFFER,
                buffer::MemoryAccess::READ_WRITE,
            )
            .unwrap();

        let pending_upload = Some(PendingUpload {
            buffer: Arc::clone(&upload_pool[0].buffer),
            width: webp_width,
            height: webp_height,
            format: Format::R8G8B8A8_UNORM,
        });

        let test_data = TestData {
            test_pixel_buffer: Arc::new(test_pixel_buffer),
            test_pixel_format: Format::R8G8B8A8_UNORM,
            target_frame: None,
            test_sampler: Arc::new(test_sampler),
            test_pipeline: Arc::new(test_pipeline),
            test_vertex_buffer: Arc::new(test_vertex_buffer),
            test_index_buffer: Arc::new(test_index_buffer),
            upload_pool,
            upload_cursor: 1,
            pending_upload,
            test_vertex_data: Vec::with_capacity(4096),
            test_index_data: Vec::with_capacity(4096),
        };

        let command_queue = Queue::new(&device, 4);

        Core {
            instance,
            device: device,
            allocator,
            test_data,
            frame_n: 0,
            queue: command_queue,
            pixel_buffers: HashMap::new(),
            pending_images: Arc::new(Mutex::new(VecDeque::new())),
            render_commands: Arc::new(Mutex::new(VecDeque::new())),
            cached_render_frame: None,
            next_image_id: 0,
            frame_cpu_times: RingBuffer::new(120),
        }
    }

    /// Enumerates all available physical devices (GPUs) accessible via the Vulkan instance.
    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let devices = self.instance.enumerate_physical_devices()?;
        Ok(devices)
    }

    /// Creates a new `CoreBuilder` to configure and build a `Core` instance.
    pub fn builder(params: StartupParameters) -> CoreBuilder {
        CoreBuilder::new(params)
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

    pub fn image_mailbox(&self) -> Arc<Mutex<VecDeque<io::Image>>> {
        Arc::clone(&self.pending_images)
    }

    pub fn submit_image(&self, io_image: io::Image) {
        if let Ok(mut pending) = self.pending_images.lock() {
            pending.push_back(io_image);
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
        if self.test_data.upload_pool.is_empty() {
            let buffer = self
                .allocator
                .create_memory_buffer(
                    required_size,
                    buffer::MemoryUsage::TRANSFER_SRC,
                    buffer::MemoryAccess::WRITE,
                )
                .unwrap();
            self.test_data.upload_pool.push(UploadBuffer {
                buffer: Arc::new(buffer),
                capacity: required_size,
            });
            self.test_data.upload_cursor = 0;
        }

        let index = self.test_data.upload_cursor % self.test_data.upload_pool.len();
        if self.test_data.upload_pool[index].capacity < required_size {
            let buffer = self
                .allocator
                .create_memory_buffer(
                    required_size,
                    buffer::MemoryUsage::TRANSFER_SRC,
                    buffer::MemoryAccess::WRITE,
                )
                .unwrap();
            self.test_data.upload_pool[index] = UploadBuffer {
                buffer: Arc::new(buffer),
                capacity: required_size,
            };
        }

        let upload_buffer = Arc::clone(&self.test_data.upload_pool[index].buffer);
        self.test_data.upload_cursor = (self.test_data.upload_cursor + 1) % self.test_data.upload_pool.len();
        upload_buffer
    }

    fn ensure_test_pixel_buffer(&mut self, width: u32, height: u32, format: Format) {
        let needs_recreate =
            self.test_data.test_pixel_buffer.width() != width
                || self.test_data.test_pixel_buffer.height() != height
                || self.test_data.test_pixel_format != format;

        if needs_recreate {
            let pixel_buffer = self
                .allocator
                .create_pixel_buffer(
                    format,
                    width,
                    height,
                    ImageUsage::SAMPLED
                        | ImageUsage::COLOR_ATTACHMENT
                        | ImageUsage::TRANSFER_DST
                        | ImageUsage::TRANSFER_SRC,
                    ImageAspect::COLOR,
                    Samples::X1,
                )
                .unwrap();
            self.test_data.test_pixel_buffer = Arc::new(pixel_buffer);
            self.test_data.test_pixel_format = format;
        }
    }

    fn stage_incoming_images(&mut self) {
        let next_image = {
            let mut pending = self.pending_images.lock().unwrap();
            let image = pending.pop_back();
            pending.clear();
            image
        };

        if let Some(io_image) = next_image {
            let upload_buffer = self.acquire_upload_buffer(io_image.data.len());
            upload_buffer.write(&io_image.data);

            self.ensure_test_pixel_buffer(io_image.width, io_image.height, io_image.format);

            self.test_data.pending_upload = Some(PendingUpload {
                buffer: upload_buffer,
                width: io_image.width,
                height: io_image.height,
                format: io_image.format,
            });
        }
    }

    pub fn create_image(&mut self, io_image: io::Image) -> Image {
        let image = Image {
            width: io_image.width,
            height: io_image.height,
            id: self.next_image_id,
        };
        self.next_image_id += 1;
        if let Ok(mut pending) = self.pending_images.lock() {
            pending.push_back(io_image);
        }
        self.pixel_buffers.insert(image.id, None);
        image
    }

    pub fn initialize_swapchain(&mut self, parameters: presentation::Parameters) {
        let swapchain_size = Vector2u::new(parameters.width, parameters.height);

        self.queue.swap_presenter(parameters);

        let target_frame = self
            .allocator
            .create_pixel_buffer(
                Format::B8G8R8A8_UNORM,
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
    }

    pub fn create_pipeline(&self, parameters: pipeline::Parameters) -> Pipeline {
        Pipeline::new(self.device.clone(), &parameters).unwrap()
    }

    fn push_image(&mut self, image: Image, rect: Rectangle, fit: Fit, color: u32) {
        let image_extent = Vector2f::new(image.width as f32, image.height as f32);
        let mut uv0 = Vector2f::new(0.0, 0.0);
        let mut uv1 = Vector2f::new(1.0, 1.0);

        let final_image_position;
        let mut final_image_extent;

        match fit {
            Fit::NONE => {
                final_image_position = rect.position();
                final_image_extent = rect.extent();

                uv0.x = rect.x / image_extent.x;
                uv0.y = rect.y / image_extent.y;
                uv1.x = (rect.x + rect.w) / image_extent.x;
                uv1.y = (rect.y + rect.h) / image_extent.y;
            }
            Fit::FILL => {
                final_image_position = rect.position();
                final_image_extent = rect.extent();
            }
            Fit::FILL_KEEP_ASPECT_RATIO => {
                let horizontal = image_extent.x > image_extent.y;
                let scale = if horizontal {
                    rect.h / image_extent.y
                } else {
                    rect.w / image_extent.x
                };
                final_image_extent = image_extent * scale;
                
                // Calculate UV coordinates to crop the oversized dimension
                let overflow = Vector2f::new(
                    (final_image_extent.x - rect.extent().x) / final_image_extent.x,
                    (final_image_extent.y - rect.extent().y) / final_image_extent.y,
                );
                uv0 = overflow * 0.5;
                uv1 = Vector2f::new(1.0, 1.0) - overflow * 0.5;
                
                final_image_extent = rect.extent();
                final_image_position = rect.position();
            }
            Fit::FIT_KEEP_ASPECT_RATIO => {
                let horizontal = image_extent.x > image_extent.y;
                let scale = if horizontal {
                    rect.w / image_extent.x
                } else {
                    rect.h / image_extent.y
                };
                final_image_extent = image_extent * scale;
                let offset = (rect.extent() - final_image_extent) * 0.5;
                final_image_position = rect.position() + offset;
            }
            Fit::CENTER => {
                let offset = (rect.extent() - image_extent) * 0.5;
                final_image_position = rect.position() + offset;
                final_image_extent = image_extent;
            }
        }

        self.push_quad(
            final_image_position,
            final_image_position + final_image_extent,
            uv0,
            uv1,
            color,
        );
    }

    fn push_quad(&mut self, a: Vector2f, c: Vector2f, a_uv: Vector2f, c_uv: Vector2f, color: u32) {
        let vertices = &mut self.test_data.test_vertex_data;
        let indices = &mut self.test_data.test_index_data;

        let start_index = vertices.len() as u32;
        indices.push(start_index + 0);
        indices.push(start_index + 1);
        indices.push(start_index + 2);
        indices.push(start_index + 0);
        indices.push(start_index + 2);
        indices.push(start_index + 3);

        let b = Vector2f::new(c.x, a.y);
        let d = Vector2f::new(a.x, c.y);
        let b_uv = Vector2f::new(c_uv.x, a_uv.y);
        let d_uv = Vector2f::new(a_uv.x, c_uv.y);

        vertices.push(GpuVertex {
            position: a,
            texture: a_uv,
            color,
        });
        vertices.push(GpuVertex {
            position: b,
            texture: b_uv,
            color,
        });
        vertices.push(GpuVertex {
            position: c,
            texture: c_uv,
            color,
        });
        vertices.push(GpuVertex {
            position: d,
            texture: d_uv,
            color,
        });
    }

    pub fn render(&mut self, _t: f64, _dt: f32) {
        let frame_start = std::time::Instant::now();

        self.next_frame();
        self.test_data.test_vertex_data.clear();
        self.test_data.test_index_data.clear();

        let target_frame = match self.test_data.target_frame.as_ref() {
            Some(frame) => Arc::clone(frame),
            None => return,
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

        if let Some(frame) = &self.cached_render_frame {
            // Update vertex and index buffers with pre-computed data from UI
            if !frame.vertices.is_empty() {
                self.test_data.test_vertex_buffer.write(&frame.vertices);
            }
            if !frame.indices.is_empty() {
                self.test_data.test_index_buffer.write(&frame.indices);
            }
        } else {
            // Fallback: render the dynamic image (compute vertices on the fly)
            self.test_data.test_vertex_data.clear();
            self.test_data.test_index_data.clear();
            self.push_image(
                Image {
                    width: self.test_data.test_pixel_buffer.width(),
                    height: self.test_data.test_pixel_buffer.height(),
                    id: 0,
                },
                Rectangle {
                    x: 0.0,
                    y: 0.0,
                    w: target_frame.width() as f32,
                    h: target_frame.height() as f32,
                },
                Fit::FIT_KEEP_ASPECT_RATIO,
                0xFFFFFFFFu32,
            );
            self.test_data
                .test_vertex_buffer
                .write(&self.test_data.test_vertex_data);
            self.test_data
                .test_index_buffer
                .write(&self.test_data.test_index_data);
        }

        let test_pixel_buffer = Arc::clone(&self.test_data.test_pixel_buffer);
        let pending_upload = self.test_data.pending_upload.take();
        let test_pipeline = Arc::clone(&self.test_data.test_pipeline);
        let test_vertex_buffer = Arc::clone(&self.test_data.test_vertex_buffer);
        let test_index_buffer = Arc::clone(&self.test_data.test_index_buffer);
        let test_sampler = Arc::clone(&self.test_data.test_sampler);
        let cached_frame = self.cached_render_frame.clone();
        let target_frame_clone = Arc::clone(&target_frame);

        self.queue.enqueue(move |cmd| {
            if let Some(upload) = &pending_upload {
                cmd.layout_barrier(&test_pixel_buffer, Layout::UNDEFINED, Layout::TRANSFER_DST);
                cmd.copy_buffer_to_image(&upload.buffer, &test_pixel_buffer);

                cmd.layout_barrier(
                    &test_pixel_buffer,
                    Layout::TRANSFER_DST,
                    Layout::SHADER_READ_ONLY,
                );
            } else {
                cmd.layout_barrier(
                    &test_pixel_buffer,
                    Layout::SHADER_READ_ONLY,
                    Layout::SHADER_READ_ONLY,
                );
            }

            cmd.layout_barrier(&target_frame, Layout::UNDEFINED, Layout::TRANSFER_DST);

            cmd.clear_pixel_buffer(&target_frame, &[0f32, 0f32, 0f32, 0f32]);

            cmd.layout_barrier(
                &target_frame,
                Layout::TRANSFER_DST,
                Layout::COLOR_ATTACHMENT,
            );

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
                        let scissor = draw_cmd.scissor.clone().unwrap_or_else(|| render_area.clone());
                        cmd.set_scissor(&scissor);
                        
                        cmd.push_pixel_descriptor(&test_pipeline, 0, &test_pixel_buffer, 1, &test_sampler);
                        cmd.draw_indexed(draw_cmd.index_count, 1, draw_cmd.index_offset, 0, 0);
                    }
                }
            } else {
                // Fallback: no batches (shouldn't happen with current logic, but safe)
                cmd.set_scissor(&render_area);
            }

            cmd.end_rendering();

            cmd.layout_barrier(
                &target_frame,
                Layout::COLOR_ATTACHMENT,
                Layout::TRANSFER_SRC,
            );
        });

        self.queue.enqueue_present(move |cmd, present_image| {
            cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::TRANSFER_DST);
            cmd.blit_to_present(&target_frame_clone, present_image, Filter::Linear);
            cmd.present_image_barrier(present_image, Layout::TRANSFER_DST, Layout::PRESENT_SRC_KHR);
        });

        let frame_end = std::time::Instant::now();
        let frame_duration = frame_end.duration_since(frame_start).as_secs_f64();
        self.frame_cpu_times.push(frame_duration);
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

/// Builder for creating and configuring a `Core` graphics instance.
///
/// `CoreBuilder` allows fine-grained control over graphics initialization, including
/// API parameter configuration and physical device selection strategy.
#[derive(Debug)]
pub struct CoreBuilder {
    params: StartupParameters,
    selector: DeviceSelector,
}

impl CoreBuilder {
    /// Creates a new `CoreBuilder` with the given API parameters.
    pub fn new(params: StartupParameters) -> Self {
        Self {
            params,
            selector: DeviceSelector::Optimal,
        }
    }

    /// Selects the optimal physical device based on device type and API/driver versions.
    ///
    /// Discrete GPUs are preferred over integrated, virtual, and CPU devices.
    /// Ties are broken by higher API and driver versions.
    pub fn select_optimal_device(mut self) -> Self {
        self.selector = DeviceSelector::Optimal;
        self
    }

    /// Selects a specific device by its index in the enumerated list.
    pub fn select_device_by_index(mut self, index: usize) -> Self {
        self.selector = DeviceSelector::ByIndex(index);
        self
    }

    /// Selects a specific device by its UUID.
    pub fn select_device_by_id(mut self, id: u128) -> Self {
        self.selector = DeviceSelector::ById(id);
        self
    }

    /// Selects a specific device by its LUID (Windows-specific identifier).
    pub fn select_device_by_luid(mut self, luid: u64) -> Self {
        self.selector = DeviceSelector::ByLuid(luid);
        self
    }

    /// Selects a specific device by its name.
    pub fn select_device_by_name<S: Into<String>>(mut self, name: S) -> Self {
        self.selector = DeviceSelector::ByName(name.into());
        self
    }

    /// Builds the `Core` instance.
    pub fn build(self) -> Result<Core> {
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

            let selected = match self.selector {
                DeviceSelector::Optimal => devices
                    .into_iter()
                    .max_by_key(|d| (pick_type_score(&d.device_type), d.api, d.driver)),
                DeviceSelector::ByIndex(i) => devices.into_iter().nth(i),
                DeviceSelector::ById(id) => devices.into_iter().find(|d| d.id == id),
                DeviceSelector::ByLuid(luid) => devices.into_iter().find(|d| d.luid == luid),
                DeviceSelector::ByName(name) => devices.into_iter().find(|d| d.name == name),
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

        Ok(Core::new(vk_instance, vk_device, allocator))
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
    float r = (float)(packed & 0xFF) / 255.0f;
    float g = (float)((packed >> 8) & 0xFF) / 255.0f;
    float b = (float)((packed >> 16) & 0xFF) / 255.0f;
    float a = (float)((packed >> 24) & 0xFF) / 255.0f;
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
