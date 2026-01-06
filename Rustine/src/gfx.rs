#![allow(dead_code)]

mod core;
mod instance;
mod vma;
pub use instance::Instance;
mod device;
pub use device::*;
pub mod vulkan;
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
pub mod compiler;
pub use compiler::*;
pub mod pipeline;
pub use pipeline::*;

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

use vulkan as vk;

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    R8G8B8A8_UNORM,
    B8G8R8A8_UNORM,
    D32_SFLOAT,
    D24_UNORM_S8_UINT,
    D32_SFLOAT_S8_UINT,
}
impl Format {
    pub fn to_vk(&self) -> vk::VkFormat {
        match self {
            Format::R8G8B8A8_UNORM => vk::VkFormat::R8G8B8A8_UNORM,
            Format::B8G8R8A8_UNORM => vk::VkFormat::B8G8R8A8_UNORM,
            Format::D32_SFLOAT => vk::VkFormat::D32_SFLOAT,
            Format::D24_UNORM_S8_UINT => vk::VkFormat::D24_UNORM_S8_UINT,
            Format::D32_SFLOAT_S8_UINT => vk::VkFormat::D32_SFLOAT_S8_UINT,
        }
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
pub struct Samples(u32);
impl Samples {
    pub const X1: Self = Self(vulkan::VkSampleCountFlags::X1_BIT as u32);
    pub const X2: Self = Self(vulkan::VkSampleCountFlags::X2_BIT as u32);
    pub const X4: Self = Self(vulkan::VkSampleCountFlags::X4_BIT as u32);
    pub const X8: Self = Self(vulkan::VkSampleCountFlags::X8_BIT as u32);
    pub const X16: Self = Self(vulkan::VkSampleCountFlags::X16_BIT as u32);
    pub const X32: Self = Self(vulkan::VkSampleCountFlags::X32_BIT as u32);
    pub const X64: Self = Self(vulkan::VkSampleCountFlags::X64_BIT as u32);
}
impl std::ops::BitOr for Samples {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the aspect flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAspect(u32);
impl ImageAspect {
    pub const COLOR: Self = Self(vulkan::VkImageAspectFlags::COLOR_BIT as u32);
    pub const DEPTH: Self = Self(vulkan::VkImageAspectFlags::DEPTH_BIT as u32);
    pub const STENCIL: Self = Self(vulkan::VkImageAspectFlags::STENCIL_BIT as u32);

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
pub struct ImageUsage(vulkan::VkImageUsageFlags);
impl ImageUsage {
    pub const TRANSFER_SRC: Self = Self(vulkan::VkImageUsageFlags::TRANSFER_SRC_BIT);
    pub const TRANSFER_DST: Self = Self(vulkan::VkImageUsageFlags::TRANSFER_DST_BIT);
    pub const SAMPLED: Self = Self(vulkan::VkImageUsageFlags::SAMPLED_BIT);
    pub const STORAGE: Self = Self(vulkan::VkImageUsageFlags::STORAGE_BIT);
    pub const COLOR_ATTACHMENT: Self = Self(vulkan::VkImageUsageFlags::COLOR_ATTACHMENT_BIT);
    pub const DEPTH_ATTACHMENT: Self =
        Self(vulkan::VkImageUsageFlags::DEPTH_STENCIL_ATTACHMENT_BIT);
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
    pub const TRANSFER_SRC: Self = Self(vulkan::VkBufferUsageFlags::TRANSFER_SRC_BIT as u32);
    pub const TRANSFER_DST: Self = Self(vulkan::VkBufferUsageFlags::TRANSFER_DST_BIT as u32);
    pub const UNIFORM: Self = Self(vulkan::VkBufferUsageFlags::UNIFORM_BUFFER_BIT as u32);
    pub const STORAGE: Self = Self(vulkan::VkBufferUsageFlags::STORAGE_BUFFER_BIT as u32);
    pub const INDEX: Self = Self(vulkan::VkBufferUsageFlags::INDEX_BUFFER_BIT as u32);
    pub const VERTEX: Self = Self(vulkan::VkBufferUsageFlags::VERTEX_BUFFER_BIT as u32);
    pub const INDIRECT: Self = Self(vulkan::VkBufferUsageFlags::INDIRECT_BUFFER_BIT as u32);

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

/// Represents the result of a graphics operation.
pub type Result<T> = std::result::Result<T, Status>;
