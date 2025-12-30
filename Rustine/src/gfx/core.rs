use crate::{
    gfx::presentation::PresentationMethod, gfx::presentation::PresentationProvider,
    gfx::queue::Queue, gfx::*, warning,
};

use std::sync::Arc;

/// The core graphics subsystem, managing Vulkan initialization and device selection.
///
/// `Core` encapsulates a Vulkan instance and a selected physical device.
/// It is responsible for creating and maintaining the graphics pipeline.
/// `Core` is thread-safe and can be shared across threads.
pub struct Core {
    test_pixel_buffer: PixelBuffer,
    queue: Option<Queue>,
    allocator: Arc<vma::Allocator>,
    physical_device: PhysicalDevice,
    device: Arc<vulkan::Device>,
    instance: vulkan::Instance,
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
        instance: vulkan::Instance,
        physical_device: PhysicalDevice,
        device: Arc<vulkan::Device>,
        allocator: Arc<vma::Allocator>,
        test_pixel_buffer: PixelBuffer,
    ) -> Self {
        Core {
            instance,
            physical_device,
            device,
            allocator,
            test_pixel_buffer,
            frame_n: 0,
            queue: None,
        }
    }

    /// Enumerates all available physical devices (GPUs) accessible via the Vulkan instance.
    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let devices = vulkan::enumerate_physical_devices(&self.instance)?;
        Ok(devices)
    }

    /// Creates a new `CoreBuilder` to configure and build a `Core` instance.
    pub fn builder(params: StartupParameters) -> CoreBuilder {
        CoreBuilder::new(params)
    }

    /// Returns a reference to the selected physical device.
    pub fn selected_physical_device(&self) -> &PhysicalDevice {
        &self.physical_device
    }

    pub fn vulkan_instance_handle(&self) -> vulkan_ffi::VkInstance {
        self.instance.handle()
    }

    pub fn device(&self) -> &Arc<vulkan::Device> {
        &self.device
    }

    pub fn allocator(&self) -> &Arc<vma::Allocator> {
        &self.allocator
    }

    pub fn next_frame(&mut self) -> u64 {
        self.frame_n += 1;
        self.frame_n
    }

    pub fn initialize_queue(
        &mut self,
        presentation_method: PresentationMethod,
        presentation_provider: impl PresentationProvider + 'static,
    ) {
        let queue = Queue::new(&self.device, presentation_method, presentation_provider);

        self.queue = Some(queue);
    }
    pub fn render(&mut self, t: f64, _dt: f32) {
        self.next_frame();

        if let Some(queue) = &mut self.queue {
            queue.enqueue_present(|cmd, pixel_buffer| {
                cmd.pixel_buffer_barrier(pixel_buffer, ImageLayout::GENERAL, ImageLayout::GENERAL);
                cmd.clear_pixel_buffer(
                    pixel_buffer,
                    [0.5, 0.5, ((t * 1.0).sin() * 0.5 + 0.5) as f32, 1.0],
                );
                cmd.pixel_buffer_barrier(pixel_buffer, ImageLayout::GENERAL, ImageLayout::GENERAL);
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
        let vk_instance = vulkan::create_instance(&self.params)?;

        // 2. Select physical device
        let selected_device = {
            let devices = vulkan::enumerate_physical_devices(&vk_instance)?;

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
        let vk_device = Arc::new(vulkan::create_device(
            &self.params,
            selected_device.handle as vulkan_ffi::VkPhysicalDevice,
        )?);

        // 4. Create VMA
        let allocator = vma::Allocator::new(&vk_instance, Arc::clone(&vk_device))?;

        let test_pixel_buffer = allocator.create_pixel_buffer(
            Format::R8G8B8A8_UNORM,
            256,
            256,
            ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
            ImageAspect::COLOR,
            ImageSamples::X1,
        )?;

        Ok(Core::new(
            vk_instance,
            selected_device,
            vk_device,
            allocator,
            test_pixel_buffer,
        ))
    }
}
