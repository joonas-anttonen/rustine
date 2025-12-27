#![allow(dead_code)]

mod parameters;
pub use parameters::ApiParameters;
pub mod buffers;
mod core;
mod vma;
mod vma_ffi;
pub mod vulkan;
pub mod vulkan_ffi;
pub use core::Core;

use crate::version::Version;
use std::fmt;

use crate::warning;

pub const MINIMUM_VULKAN_API_VERSION: Version = Version::new(1, 4, 0);

pub struct PixelBuffer {
    image: vulkan_ffi::VkImage,
    image_view: vulkan_ffi::VkImageView,
    allocation: vma_ffi::VmaAllocation,
    allocator: std::sync::Arc<vma::Allocator>,
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        warning!("PixelBuffer::drop");
        unsafe {
            vulkan_ffi::vkDestroyImageView(
                self.allocator.device.handle(),
                self.image_view,
                std::ptr::null(),
            );
            vma_ffi::vmaDestroyImage(self.allocator.handle, self.image, self.allocation);
        }
    }
}

pub struct MemoryBuffer {
    pub handle: vma_ffi::VmaAllocation,
    pub allocator: std::sync::Arc<vma::Allocator>,
}

impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        warning!("MemoryBuffer::drop");
        unsafe {
            vma_ffi::vmaDestroyBuffer(self.allocator.handle, std::ptr::null_mut(), self.handle);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format(vulkan_ffi::VkFormat);
impl Format {
    pub const R8G8B8A8_UNORM: Self = Self(vulkan_ffi::VkFormat::R8G8B8A8_UNORM);
    pub const B8G8R8A8_UNORM: Self = Self(vulkan_ffi::VkFormat::B8G8R8A8_UNORM);
    pub const D32_SFLOAT: Self = Self(vulkan_ffi::VkFormat::D32_SFLOAT);
    pub const D24_UNORM_S8_UINT: Self = Self(vulkan_ffi::VkFormat::D24_UNORM_S8_UINT);
    pub const D32_SFLOAT_S8_UINT: Self = Self(vulkan_ffi::VkFormat::D32_SFLOAT_S8_UINT);

    pub fn to_vk(&self) -> vulkan_ffi::VkFormat {
        self.0
    }
    pub fn from_vk(format: vulkan_ffi::VkFormat) -> Self {
        Self(format)
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

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
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
pub struct ImageUsage(u32);
impl ImageUsage {
    pub const TRANSFER_SRC: Self = Self(vulkan_ffi::VkImageUsageFlags::TRANSFER_SRC_BIT as u32);
    pub const TRANSFER_DST: Self = Self(vulkan_ffi::VkImageUsageFlags::TRANSFER_DST_BIT as u32);
    pub const SAMPLED: Self = Self(vulkan_ffi::VkImageUsageFlags::SAMPLED_BIT as u32);
    pub const STORAGE: Self = Self(vulkan_ffi::VkImageUsageFlags::STORAGE_BIT as u32);
    pub const COLOR_ATTACHMENT: Self =
        Self(vulkan_ffi::VkImageUsageFlags::COLOR_ATTACHMENT_BIT as u32);
    pub const DEPTH_ATTACHMENT: Self =
        Self(vulkan_ffi::VkImageUsageFlags::DEPTH_STENCIL_ATTACHMENT_BIT as u32);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
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
pub enum Outcome {
    Success,
    NotImplemented,
    NotSupported,
    Unknown(i32),
}

impl std::error::Error for Outcome {}
impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Success => write!(f, "Success"),
            Outcome::NotSupported => write!(f, "Not supported"),
            Outcome::NotImplemented => write!(f, "Not implemented"),
            Outcome::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

impl Outcome {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Outcome::Success,
            -7 | -8 | -11 => Outcome::NotSupported, // Extension not present, feature not present, format not supported
            other => Outcome::Unknown(other),
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
pub type Result<T> = std::result::Result<T, Outcome>;
