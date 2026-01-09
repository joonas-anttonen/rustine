use crate::{gfx::presentation::PresentationProvider, gfx::queue::Queue, gfx::*, io, warning};

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;

struct TestData {
    test_pixel_buffer: Arc<PixelBuffer>,
    test_pixel_buffer_2: Arc<PixelBuffer>,
    test_vertex_buffer: Arc<MemoryBuffer>,
    test_index_buffer: Arc<MemoryBuffer>,
    test_image_transfer_buffer: Option<Arc<MemoryBuffer>>,
    test_sampler: Arc<Sampler>,
    test_pipeline: Arc<Pipeline>,
    test_vertex_data: Vec<GpuVertex>,
    test_index_data: Vec<u32>,
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

/// The core graphics subsystem, managing Vulkan initialization and device selection.
///
/// `Core` encapsulates a Vulkan instance and a selected physical device.
/// It is responsible for creating and maintaining the graphics pipeline.
/// `Core` is thread-safe and can be shared across threads.
pub struct Core {
    test_data: TestData,
    pending_images: VecDeque<io::Image>,
    next_image_id: u32,
    pixel_buffers: HashMap<u32, Option<PixelBuffer>>,
    queue: Option<Queue>,
    allocator: Arc<allocator::Allocator>,
    device: Arc<Device>,
    instance: Instance,
    frame_n: u64,
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

        let test_image_transfer_buffer = allocator
            .create_memory_buffer(
                webp_frame_data.len(),
                buffer::MemoryUsage::TRANSFER_SRC,
                buffer::MemoryAccess::WRITE,
            )
            .unwrap();
        test_image_transfer_buffer.write(&webp_frame_data);

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

        let test_pixel_buffer_2 = allocator
            .create_pixel_buffer(
                Format::B8G8R8A8_UNORM,
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

        let test_data = TestData {
            test_pixel_buffer: Arc::new(test_pixel_buffer),
            test_pixel_buffer_2: Arc::new(test_pixel_buffer_2),
            test_sampler: Arc::new(test_sampler),
            test_pipeline: Arc::new(test_pipeline),
            test_vertex_buffer: Arc::new(test_vertex_buffer),
            test_index_buffer: Arc::new(test_index_buffer),
            test_image_transfer_buffer: Some(Arc::new(test_image_transfer_buffer)),
            test_vertex_data: Vec::with_capacity(4096),
            test_index_data: Vec::with_capacity(4096),
        };

        Core {
            instance,
            device: device,
            allocator,
            test_data,
            frame_n: 0,
            queue: None,
            pixel_buffers: HashMap::new(),
            pending_images: VecDeque::new(),
            next_image_id: 0,
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

    pub fn drop_queue(&mut self) {
        self.queue = None;
    }

    pub fn create_image(&mut self, io_image: io::Image) -> Image {
        let image = Image {
            width: io_image.width,
            height: io_image.height,
            id: self.next_image_id,
        };
        self.next_image_id += 1;
        self.pending_images.push_back(io_image);
        self.pixel_buffers.insert(image.id, None);
        image
    }

    pub fn initialize_swapchain_queue(
        &mut self,
        presentation_provider: impl PresentationProvider + 'static,
    ) {
        self.queue = Some(Queue::new(&self.device, presentation_provider));
    }

    pub fn initialize_shared_image_queue(
        &mut self,
        presentation_provider: impl PresentationProvider + 'static,
    ) {
        self.queue = Some(Queue::new(&self.device, presentation_provider));
    }

    pub fn create_pipeline(&self, parameters: pipeline::Parameters) -> Pipeline {
        Pipeline::new(self.device.clone(), &parameters).unwrap()
    }

    pub fn initialize_queue(&mut self, presentation_provider: impl PresentationProvider + 'static) {
        self.queue = Some(Queue::new(&self.device, presentation_provider));
    }

    fn push_image(&mut self, image: Image, rect: Rectangle, fit: Fit, color: u32) {
        let image_extent = Vector2f::new(image.width as f32, image.height as f32);
        let mut uv0 = Vector2f::new(0.0, 0.0);
        let mut uv1 = Vector2f::new(1.0, 1.0);

        let final_image_position;
        let final_image_extent;

        match fit {
            Fit::NONE => {
                final_image_position = rect.position();
                final_image_extent = rect.extent();

                uv0.x = rect.x / image_extent.x;
                uv0.y = rect.y / image_extent.y;
                uv1.x = (rect.x + rect.w) / image_extent.x;
                uv1.y = (rect.y + rect.h) / image_extent.y;
            }
            Fit::STRETCH => {
                final_image_position = rect.position();
                final_image_extent = rect.extent();
            }
            Fit::FILL => {
                final_image_position = rect.position();
                final_image_extent = rect.extent();
            }
            Fit::FILL_KEEP_ASPECT_RATIO => {
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
        self.next_frame();
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
                w: self.test_data.test_pixel_buffer_2.width() as f32,
                h: self.test_data.test_pixel_buffer_2.height() as f32,
            },
            Fit::FILL_KEEP_ASPECT_RATIO,
            0xFFFFFFFFu32,
        );

        self.test_data
            .test_vertex_buffer
            .write(&self.test_data.test_vertex_data);
        self.test_data
            .test_index_buffer
            .write(&self.test_data.test_index_data);

        if let Some(queue) = &mut self.queue {
            queue.enqueue(|cmd| {
                if let Some(test_image_transfer_buffer) = &self.test_data.test_image_transfer_buffer
                {
                    cmd.layout_barrier(
                        &self.test_data.test_pixel_buffer,
                        Layout::UNDEFINED,
                        Layout::TRANSFER_DST,
                    );
                    cmd.copy_buffer_to_image(
                        test_image_transfer_buffer,
                        &self.test_data.test_pixel_buffer,
                    );

                    self.test_data.test_image_transfer_buffer = None;

                    cmd.layout_barrier(
                        &self.test_data.test_pixel_buffer,
                        Layout::TRANSFER_DST,
                        Layout::SHADER_READ_ONLY,
                    );
                } else {
                    cmd.layout_barrier(
                        &self.test_data.test_pixel_buffer,
                        Layout::SHADER_READ_ONLY,
                        Layout::SHADER_READ_ONLY,
                    );
                }

                cmd.layout_barrier(
                    &self.test_data.test_pixel_buffer_2,
                    Layout::UNDEFINED,
                    Layout::TRANSFER_DST,
                );

                cmd.clear_pixel_buffer(
                    &self.test_data.test_pixel_buffer_2,
                    &[0f32, 0f32, 1f32, 1f32],
                );

                cmd.layout_barrier(
                    &self.test_data.test_pixel_buffer_2,
                    Layout::TRANSFER_DST,
                    Layout::COLOR_ATTACHMENT,
                );

                let render_area = Rectangle {
                    x: 0.0,
                    y: 0.0,
                    w: self.test_data.test_pixel_buffer_2.width() as f32,
                    h: self.test_data.test_pixel_buffer_2.height() as f32,
                };

                cmd.begin_rendering(&render_area, &[&self.test_data.test_pixel_buffer_2]);
                cmd.bind_pipeline(&self.test_data.test_pipeline);
                cmd.bind_vertex_buffer(&self.test_data.test_vertex_buffer);
                cmd.bind_index_buffer(&self.test_data.test_index_buffer);
                cmd.set_viewport(&render_area);
                cmd.set_scissor(&render_area);

                let push_constants = PerCommand {
                    scale: Vector2f::new(
                        2f32 / self.test_data.test_pixel_buffer_2.width() as f32,
                        2f32 / self.test_data.test_pixel_buffer_2.height() as f32,
                    ),
                    sdf_range: 1f32,
                    is_sdf: false,
                };
                cmd.push_constants(
                    &self.test_data.test_pipeline,
                    Stage::VERTEX | Stage::FRAGMENT,
                    &push_constants,
                );
                cmd.push_pixel_descriptor(
                    &self.test_data.test_pipeline,
                    0,
                    &self.test_data.test_pixel_buffer,
                    1,
                    &self.test_data.test_sampler,
                );
                cmd.draw_indexed(self.test_data.test_index_data.len() as u32, 1, 0, 0, 0);
                cmd.end_rendering();

                cmd.layout_barrier(
                    &self.test_data.test_pixel_buffer_2,
                    Layout::COLOR_ATTACHMENT,
                    Layout::TRANSFER_SRC,
                );
            });

            queue.enqueue_present(|cmd, present_image| {
                cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::TRANSFER_DST);
                cmd.blit_to_present(
                    &self.test_data.test_pixel_buffer_2,
                    present_image,
                    Filter::Linear,
                );
                cmd.present_image_barrier(present_image, Layout::TRANSFER_DST, Layout::PRESENT_SRC_KHR);

                /*cmd.present_image_barrier(present_image, Layout::UNDEFINED, Layout::GENERAL);
                
                cmd.clear_present_image(
                    present_image,
                    [
                        ((t * 1.0).sin() * 0.25 + 0.5) as f32,
                        ((t * 5.0).sin() * 0.25 + 0.5) as f32,
                        ((t * 10.0).sin() * 0.25 + 0.5) as f32,
                        1.0,
                    ],
                );
                cmd.present_image_barrier(present_image, Layout::GENERAL, Layout::PRESENT_SRC_KHR);*/
            });
        }
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
    return textureColor;
    float alpha = textureColor.a;

	//if (command.isSdf)
	//{ 
	//	float sdf = textureColor.a - 0.5;
	//	alpha = smoothstep(-command.sdfRange, +command.sdfRange, sdf);
	//}

	return float4(geometryColor.rgb * textureColor.rgb, alpha * geometryColor.a);
}
"#;
