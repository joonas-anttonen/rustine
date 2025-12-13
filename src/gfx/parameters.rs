#![allow(dead_code)]

use super::Version;

/// Parameters for initializing the graphics API.
#[derive(Debug)]
pub struct ApiParameters {
    pub enable_debugging: bool,
    pub required_api_version: Version,
    pub app_version: Version,
    pub app_engine_version: Version,
    pub app_name: String,
    pub app_engine_name: String,
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
}
