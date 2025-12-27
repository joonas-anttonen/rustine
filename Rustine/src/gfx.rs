#![allow(dead_code)]

mod parameters;
pub use parameters::ApiParameters;
mod core;
mod vma;
mod vma_ffi;
pub mod vulkan;
pub mod vulkan_ffi;
/// Re-export core types for easier access.
pub use core::Core;

use crate::version::Version;
use std::fmt;

/// Represents a rendering surface (e.g., a window surface) for graphics output.
#[derive(Debug)]
pub struct Surface {
    pub handle: u64,
    pub width: u32,
    pub height: u32,
}

/// Represents the target platform for graphics API initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Wayland,
    X11,
    MacOS,
    Unknown,
}

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

impl Result {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Result::Success,
            -7 | -8 | -11 => Result::NotSupported, // Extension not present, feature not present, format not supported
            other => Result::Unknown(other),
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
    pub driver: Version,
    pub api: Version,
    pub device_type: PhysicalDeviceType,
    pub id: u128,
    pub luid: u64,
    pub handle: u64,
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
