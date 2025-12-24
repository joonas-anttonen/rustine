use super::vulkan;

use std::{fmt, result};

/// Represents the result of a graphics operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Result {
    Success,
    NotSupported,
    Unknown(i32),
}

impl std::error::Error for Result {}
impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Result::Success => write!(f, "Success"),
            Result::NotSupported => write!(f, "Not supported"),
            Result::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

/// Represents the type of a physical graphics device.
#[derive(Debug)]
pub enum PhysicalDeviceType {
    Discrete,
    Integrated,
    Virtual,
    Cpu,
    Other,
}

/// Represents a physical graphics device (GPU) in the system.
#[derive(Debug)]
pub struct PhysicalDevice {
    pub name: String,
    pub driver: super::Version,
    pub api: super::Version,
    pub device_type: PhysicalDeviceType,
    pub id: u128,
}

impl std::fmt::Display for PhysicalDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (API: {}, Driver: {}, Type: {:?}, Id: {:?})",
            self.name, self.api, self.driver, self.device_type, self.id
        )
    }
}

pub struct Core {
    instance: vulkan::Instance,
}

impl Drop for Core {
    fn drop(&mut self) {
        // Vulkan instance will be automatically dropped
    }
}

impl Core {
    pub fn new(params: &super::ApiParameters) -> std::result::Result<Self, Result> {
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
