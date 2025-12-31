#![allow(dead_code)]

mod core;
mod vma;
mod vma_ffi;
pub mod vulkan;
pub mod vulkan_ffi;
pub use core::Core;
pub mod presentation;
pub mod queue;
pub use presentation::AcquireStatus;
pub use queue::Queue;
pub use queue::SubmitStatus;
pub mod command;
pub use command::CommandBuffer;
pub use command::CommandPool;
pub mod buffer;
pub use buffer::MemoryBuffer;
pub use buffer::PixelBuffer;

use crate::version::Version;

pub const MINIMUM_VULKAN_API_VERSION: Version = Version::new(1, 4, 0);

#[derive(Debug)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

/// Parameters for initializing the graphics API.
#[derive(Debug)]
pub struct StartupParameters {
    pub enable_debugging: bool,
    pub host_platform: Platform,
    pub host_version: Version,
    pub host_name: String,
}

use vulkan_ffi as vk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format(vk::VkFormat);
impl Format {
    pub const R8G8B8A8_UNORM: Self = Self(vk::VkFormat::R8G8B8A8_UNORM);
    pub const B8G8R8A8_UNORM: Self = Self(vk::VkFormat::B8G8R8A8_UNORM);
    pub const D32_SFLOAT: Self = Self(vk::VkFormat::D32_SFLOAT);
    pub const D24_UNORM_S8_UINT: Self = Self(vk::VkFormat::D24_UNORM_S8_UINT);
    pub const D32_SFLOAT_S8_UINT: Self = Self(vk::VkFormat::D32_SFLOAT_S8_UINT);

    pub fn to_vk(&self) -> vk::VkFormat {
        self.0
    }
    pub fn from_vk(format: vk::VkFormat) -> Self {
        Self(format)
    }
}

pub struct ImageLayout(vk::VkImageLayout);
impl ImageLayout {
    pub const UNDEFINED: Self = Self(vk::VkImageLayout::UNDEFINED);
    pub const GENERAL: Self = Self(vk::VkImageLayout::GENERAL);
    pub const COLOR_ATTACHMENT: Self = Self(vk::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    pub const DEPTH_ATTACHMENT: Self = Self(vk::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    pub const TRANSFER_SRC: Self = Self(vk::VkImageLayout::TRANSFER_SRC_OPTIMAL);
    pub const TRANSFER_DST: Self = Self(vk::VkImageLayout::TRANSFER_DST_OPTIMAL);
    pub const SHADER_READ_ONLY: Self = Self(vk::VkImageLayout::SHADER_READ_ONLY_OPTIMAL);
    pub const PRESENT_SRC_KHR: Self = Self(vk::VkImageLayout::PRESENT_SRC_KHR);

    pub fn to_vk(&self) -> vk::VkImageLayout {
        self.0
    }
    pub fn from_vk(layout: vk::VkImageLayout) -> Self {
        Self(layout)
    }
}

/// Represents the sample count flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSamples(u32);
impl ImageSamples {
    pub const X1: Self = Self(vulkan_ffi::VkSampleCountFlags::X1_BIT as u32);
    pub const X2: Self = Self(vulkan_ffi::VkSampleCountFlags::X2_BIT as u32);
    pub const X4: Self = Self(vulkan_ffi::VkSampleCountFlags::X4_BIT as u32);
    pub const X8: Self = Self(vulkan_ffi::VkSampleCountFlags::X8_BIT as u32);
    pub const X16: Self = Self(vulkan_ffi::VkSampleCountFlags::X16_BIT as u32);
    pub const X32: Self = Self(vulkan_ffi::VkSampleCountFlags::X32_BIT as u32);
    pub const X64: Self = Self(vulkan_ffi::VkSampleCountFlags::X64_BIT as u32);
}
impl std::ops::BitOr for ImageSamples {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the aspect flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAspect(u32);
impl ImageAspect {
    pub const COLOR: Self = Self(vulkan_ffi::VkImageAspectFlags::COLOR_BIT as u32);
    pub const DEPTH: Self = Self(vulkan_ffi::VkImageAspectFlags::DEPTH_BIT as u32);
    pub const STENCIL: Self = Self(vulkan_ffi::VkImageAspectFlags::STENCIL_BIT as u32);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}
impl std::ops::BitOr for ImageAspect {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the usage flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageUsage(vulkan_ffi::VkImageUsageFlags);
impl ImageUsage {
    pub const TRANSFER_SRC: Self = Self(vulkan_ffi::VkImageUsageFlags::TRANSFER_SRC_BIT);
    pub const TRANSFER_DST: Self = Self(vulkan_ffi::VkImageUsageFlags::TRANSFER_DST_BIT);
    pub const SAMPLED: Self = Self(vulkan_ffi::VkImageUsageFlags::SAMPLED_BIT);
    pub const STORAGE: Self = Self(vulkan_ffi::VkImageUsageFlags::STORAGE_BIT);
    pub const COLOR_ATTACHMENT: Self = Self(vulkan_ffi::VkImageUsageFlags::COLOR_ATTACHMENT_BIT);
    pub const DEPTH_ATTACHMENT: Self =
        Self(vulkan_ffi::VkImageUsageFlags::DEPTH_STENCIL_ATTACHMENT_BIT);
}
impl std::ops::BitOr for ImageUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the usage flags for a memory buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferUsage(u32);
impl BufferUsage {
    pub const TRANSFER_SRC: Self = Self(vulkan_ffi::VkBufferUsageFlags::TRANSFER_SRC_BIT as u32);
    pub const TRANSFER_DST: Self = Self(vulkan_ffi::VkBufferUsageFlags::TRANSFER_DST_BIT as u32);
    pub const UNIFORM: Self = Self(vulkan_ffi::VkBufferUsageFlags::UNIFORM_BUFFER_BIT as u32);
    pub const STORAGE: Self = Self(vulkan_ffi::VkBufferUsageFlags::STORAGE_BUFFER_BIT as u32);
    pub const INDEX: Self = Self(vulkan_ffi::VkBufferUsageFlags::INDEX_BUFFER_BIT as u32);
    pub const VERTEX: Self = Self(vulkan_ffi::VkBufferUsageFlags::VERTEX_BUFFER_BIT as u32);
    pub const INDIRECT: Self = Self(vulkan_ffi::VkBufferUsageFlags::INDIRECT_BUFFER_BIT as u32);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}
impl std::ops::BitOr for BufferUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the target platform for graphics API initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Wayland,
    X11,
    MacOS,
}

/// Represents the result of a graphics operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    NotImplemented(i32),
    InvalidOperation(i32),
    NotSupported(i32),
    Timeout(i32),
    OutOfDate(i32),
    Unknown(i32),
}

impl std::error::Error for Status {}
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Success => write!(f, "Success"),
            Status::InvalidOperation(code) => write!(f, "Invalid operation: {}", code),
            Status::NotSupported(code) => write!(f, "Not supported: {}", code),
            Status::NotImplemented(code) => write!(f, "Not implemented: {}", code),
            Status::Timeout(code) => write!(f, "Timeout: {}", code),
            Status::OutOfDate(code) => write!(f, "Out of date: {}", code),
            Status::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

impl Status {
    pub fn to_code(&self) -> i32 {
        match self {
            Status::Success => 0,
            Status::InvalidOperation(code) => *code,
            Status::NotImplemented(code) => *code,
            Status::NotSupported(code) => *code,
            Status::Timeout(code) => *code,
            Status::OutOfDate(code) => *code,
            Status::Unknown(code) => *code,
        }
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Status::Success,
            -7 | -8 | -11 => Status::NotSupported(code), // Extension not present, feature not present, format not supported
            -4 => Status::InvalidOperation(code),        // Invalid operation
            -1 => Status::NotImplemented(code),          // Not implemented
            2 => Status::Timeout(code),                  // Timeout
            -1000001004 => Status::OutOfDate(code),      // Out of date
            other => Status::Unknown(other),
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

/// Represents the result of a graphics operation.
pub type Result<T> = std::result::Result<T, Status>;
