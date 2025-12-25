#![allow(dead_code)]

pub mod parameters;
pub use parameters::ApiParameters;
pub mod vulkan;
pub mod core;

use crate::{version::Version};
use std::{fmt};

/// Re-export core types for easier access.
pub use core::Core;

/// Represents the target platform for graphics API initialization.
#[derive(Debug)]
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