use crate::{gfx::*, warning};

use std::{result};

pub struct Core {
    instance: vulkan::Instance,
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
    pub fn new(params: &ApiParameters) -> std::result::Result<Self, Result> {
        let vk_instance = vulkan::vk_create_instance(&params)?;

        Ok(Self {
            instance: vk_instance,
        })
    }

    pub fn enumerate_physical_devices(&self) -> result::Result<Vec<PhysicalDevice>, Result> {
        let devices = vulkan::vk_enumerate_physical_devices(&self.instance)?;
        Ok(devices)
    }
}
