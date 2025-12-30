#![allow(dead_code)]

mod core;
mod vma;
mod vma_ffi;
pub mod vulkan;
pub mod vulkan_ffi;
pub use core::Core;
pub mod presentation;
pub mod queue;
pub use queue::Queue;
pub use queue::SubmitStatus;

use crate::version::Version;
use std::fmt;

use crate::{error, vk_call, warning};
use std::sync::Arc;

pub const MINIMUM_VULKAN_API_VERSION: Version = Version::new(1, 4, 0);

pub struct CommandPool {
    handle: vulkan_ffi::VkCommandPool,
    device: Arc<vulkan::Device>,
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        warning!("CommandPool::drop");
        unsafe {
            vulkan_ffi::vkDestroyCommandPool(self.device.handle(), self.handle, std::ptr::null());
        }
    }
}

impl CommandPool {
    pub fn new(handle: vulkan_ffi::VkCommandPool, device: Arc<vulkan::Device>) -> Arc<Self> {
        Arc::new(CommandPool { handle, device })
    }

    pub fn allocate_command_buffer(self: &Arc<Self>) -> Result<CommandBuffer> {
        let allocate_info = vulkan_ffi::VkCommandBufferAllocateInfo {
            sType: vulkan_ffi::VkStructureType::COMMAND_BUFFER_ALLOCATE_INFO as u32,
            pNext: std::ptr::null(),
            commandPool: self.handle,
            level: vulkan_ffi::VkCommandBufferLevel::PRIMARY,
            commandBufferCount: 1,
        };

        let mut command_buffer_handle = vulkan_ffi::VkCommandBuffer::default();
        vk_call!(vulkan_ffi::vkAllocateCommandBuffers(
            self.device.handle(),
            &allocate_info,
            &mut command_buffer_handle
        ))?;

        // Create fence
        let fence_info = vulkan_ffi::VkFenceCreateInfo {
            sType: vulkan_ffi::VkStructureType::FENCE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut fence_handle = vulkan_ffi::VkFence::default();
        vk_call!(vulkan_ffi::vkCreateFence(
            self.device.handle(),
            &fence_info,
            std::ptr::null(),
            &mut fence_handle
        ))?;

        let semaphore_info = vulkan_ffi::VkSemaphoreCreateInfo {
            sType: vulkan_ffi::VkStructureType::SEMAPHORE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut semaphore_handle = vulkan_ffi::VkSemaphore::default();
        vk_call!(vulkan_ffi::vkCreateSemaphore(
            self.device.handle(),
            &semaphore_info,
            std::ptr::null(),
            &mut semaphore_handle
        ))?;

        Ok(CommandBuffer::new(
            command_buffer_handle,
            fence_handle,
            semaphore_handle,
            Arc::clone(&self),
        ))
    }
}

/// Represents a command buffer used for recording graphics commands.
pub struct CommandBuffer {
    handle: vulkan_ffi::VkCommandBuffer,
    fence: vulkan_ffi::VkFence,
    semaphore: vulkan_ffi::VkSemaphore,
    pool: Arc<CommandPool>,
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        warning!("CommandBuffer::drop");
        unsafe {
            vulkan_ffi::vkDestroySemaphore(
                self.pool.device.handle(),
                self.semaphore,
                std::ptr::null(),
            );
            vulkan_ffi::vkDestroyFence(self.pool.device.handle(), self.fence, std::ptr::null());
            vulkan_ffi::vkFreeCommandBuffers(
                self.pool.device.handle(),
                self.pool.handle,
                1,
                &self.handle,
            );
        }
    }
}

impl CommandBuffer {
    /// Creates a new command buffer with the given handle, fence, semaphore, and pool.
    pub fn new(
        handle: vulkan_ffi::VkCommandBuffer,
        fence: vulkan_ffi::VkFence,
        semaphore: vulkan_ffi::VkSemaphore,
        pool: Arc<CommandPool>,
    ) -> Self {
        CommandBuffer {
            handle,
            fence,
            semaphore,
            pool,
        }
    }

    pub fn is_complete(&self) -> bool {
        let status = unsafe { vulkan_ffi::vkGetFenceStatus(self.pool.device.handle(), self.fence) };
        match status {
            vulkan_ffi::VkResult::VK_SUCCESS => true,
            vulkan_ffi::VkResult::VK_NOT_READY => false,
            _ => {
                error!("Failed to get fence status: {:?}", status);
                false
            }
        }
    }

    pub fn wait_for_completion(&self, timeout_ns: u64) -> Result<()> {
        vk_call!(vulkan_ffi::vkWaitForFences(
            self.pool.device.handle(),
            1,
            &self.fence,
            vulkan_ffi::VK_TRUE,
            timeout_ns,
        ))
    }

    pub fn reset(&self) {
        vk_call!(vulkan_ffi::vkResetFences(
            self.pool.device.handle(),
            1,
            &self.fence
        ))
        .unwrap_or_else(|r| {
            error!("Failed to reset fence: {:?}", r);
        });
        vk_call!(vulkan_ffi::vkResetCommandBuffer(self.handle, 0)).unwrap_or_else(|r| {
            error!("Failed to reset command buffer: {:?}", r);
        });
    }

    /// Begins recording commands into the command buffer.
    pub fn begin(&self) {
        let begin_info = vulkan_ffi::VkCommandBufferBeginInfo {
            sType: vulkan_ffi::VkStructureType::COMMAND_BUFFER_BEGIN_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            pInheritanceInfo: std::ptr::null(),
        };
        vk_call!(vulkan_ffi::vkBeginCommandBuffer(self.handle, &begin_info)).unwrap_or_else(|r| {
            error!("Failed to begin command buffer: {:?}", r);
        });
    }

    /// Ends recording commands into the command buffer.
    pub fn end(&self) {
        vk_call!(vulkan_ffi::vkEndCommandBuffer(self.handle)).unwrap_or_else(|r| {
            error!("Failed to end command buffer: {:?}", r);
        });
    }

    fn barrier_stage_mask(layout: vulkan_ffi::VkImageLayout) -> vulkan_ffi::VkPipelineStageFlags2 {
        match layout {
            vulkan_ffi::VkImageLayout::UNDEFINED => {
                vulkan_ffi::VkPipelineStageFlags2::TOP_OF_PIPE_BIT
            }
            vulkan_ffi::VkImageLayout::TRANSFER_DST_OPTIMAL => {
                vulkan_ffi::VkPipelineStageFlags2::ALL_TRANSFER_BIT
            }
            vulkan_ffi::VkImageLayout::TRANSFER_SRC_OPTIMAL => {
                vulkan_ffi::VkPipelineStageFlags2::ALL_TRANSFER_BIT
            }
            vulkan_ffi::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                vulkan_ffi::VkPipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_BIT
            }
            vulkan_ffi::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                vulkan_ffi::VkPipelineStageFlags2::EARLY_FRAGMENT_TESTS_BIT
                    | vulkan_ffi::VkPipelineStageFlags2::LATE_FRAGMENT_TESTS_BIT
            }
            vulkan_ffi::VkImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                vulkan_ffi::VkPipelineStageFlags2::FRAGMENT_SHADER_BIT
            }
            vulkan_ffi::VkImageLayout::PRESENT_SRC_KHR => {
                vulkan_ffi::VkPipelineStageFlags2::BOTTOM_OF_PIPE_BIT
            }
            // Something else: you get everything
            _ => vulkan_ffi::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
        }
    }

    fn barrier_access_mask(layout: vulkan_ffi::VkImageLayout) -> vulkan_ffi::VkAccessFlags2 {
        match layout {
            vulkan_ffi::VkImageLayout::UNDEFINED => vulkan_ffi::VkAccessFlags2::NONE,
            vulkan_ffi::VkImageLayout::PRESENT_SRC_KHR => vulkan_ffi::VkAccessFlags2::NONE,
            vulkan_ffi::VkImageLayout::TRANSFER_DST_OPTIMAL => {
                vulkan_ffi::VkAccessFlags2::TRANSFER_WRITE_BIT
            }
            vulkan_ffi::VkImageLayout::TRANSFER_SRC_OPTIMAL => {
                vulkan_ffi::VkAccessFlags2::TRANSFER_READ_BIT
            }
            vulkan_ffi::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                vulkan_ffi::VkAccessFlags2::COLOR_ATTACHMENT_WRITE_BIT
            }
            vulkan_ffi::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                vulkan_ffi::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
            }
            vulkan_ffi::VkImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                vulkan_ffi::VkAccessFlags2::SHADER_READ_BIT
            }
            // Something else: you get everything
            _ => {
                vulkan_ffi::VkAccessFlags2::MEMORY_READ_BIT
                    | vulkan_ffi::VkAccessFlags2::MEMORY_WRITE_BIT
                    | vulkan_ffi::VkAccessFlags2::TRANSFER_READ_BIT
                    | vulkan_ffi::VkAccessFlags2::TRANSFER_WRITE_BIT
                    | vulkan_ffi::VkAccessFlags2::COLOR_ATTACHMENT_READ_BIT
                    | vulkan_ffi::VkAccessFlags2::COLOR_ATTACHMENT_WRITE_BIT
                    | vulkan_ffi::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ_BIT
                    | vulkan_ffi::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
                    | vulkan_ffi::VkAccessFlags2::SHADER_READ_BIT
                    | vulkan_ffi::VkAccessFlags2::SHADER_WRITE_BIT
            }
        }
    }

    pub fn full_barrier(&self) {
        let memory_barrier = vulkan_ffi::VkMemoryBarrier2 {
            sType: vulkan_ffi::VkStructureType::MEMORY_BARRIER_2 as u32,
            pNext: std::ptr::null(),
            srcStageMask: vulkan_ffi::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
            srcAccessMask: vulkan_ffi::VkAccessFlags2::NONE,
            dstStageMask: vulkan_ffi::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
            dstAccessMask: vulkan_ffi::VkAccessFlags2::NONE,
        };
        let dependency_info = vulkan_ffi::VkDependencyInfo {
            sType: vulkan_ffi::VkStructureType::DEPENDENCY_INFO as u32,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 1,
            pMemoryBarriers: &memory_barrier,
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 0,
            pImageMemoryBarriers: std::ptr::null(),
        };
        unsafe {
            vulkan_ffi::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    pub fn transfer_barrier(&self) {
        let memory_barrier = vulkan_ffi::VkMemoryBarrier2 {
            sType: vulkan_ffi::VkStructureType::MEMORY_BARRIER_2 as u32,
            pNext: std::ptr::null(),
            srcStageMask: vulkan_ffi::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            srcAccessMask: vulkan_ffi::VkAccessFlags2::TRANSFER_WRITE_BIT,
            dstStageMask: vulkan_ffi::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            dstAccessMask: vulkan_ffi::VkAccessFlags2::TRANSFER_READ_BIT,
        };
        let dependency_info = vulkan_ffi::VkDependencyInfo {
            sType: vulkan_ffi::VkStructureType::DEPENDENCY_INFO as u32,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 1,
            pMemoryBarriers: &memory_barrier,
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 0,
            pImageMemoryBarriers: std::ptr::null(),
        };
        unsafe {
            vulkan_ffi::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    pub fn pixel_buffer_barrier(
        &self,
        buffer: &PixelBuffer,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    ) {
        let image_memory_barrier = vulkan_ffi::VkImageMemoryBarrier2 {
            sType: vulkan_ffi::VkStructureType::IMAGE_MEMORY_BARRIER_2 as u32,
            pNext: std::ptr::null(),
            srcStageMask: Self::barrier_stage_mask(old_layout.to_vk()),
            srcAccessMask: Self::barrier_access_mask(old_layout.to_vk()),
            dstStageMask: Self::barrier_stage_mask(new_layout.to_vk()),
            dstAccessMask: Self::barrier_access_mask(new_layout.to_vk()),
            oldLayout: old_layout.to_vk(),
            newLayout: new_layout.to_vk(),
            srcQueueFamilyIndex: vulkan_ffi::VK_QUEUE_FAMILY_IGNORED,
            dstQueueFamilyIndex: vulkan_ffi::VK_QUEUE_FAMILY_IGNORED,
            image: buffer.image,
            subresourceRange: vulkan_ffi::VkImageSubresourceRange {
                // TODO: Support more aspects
                aspectMask: vulkan_ffi::VkImageAspectFlags::COLOR_BIT as u32,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };
        let dependency_info = vulkan_ffi::VkDependencyInfo {
            sType: vulkan_ffi::VkStructureType::DEPENDENCY_INFO as u32,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 0,
            pMemoryBarriers: std::ptr::null(),
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 1,
            pImageMemoryBarriers: &image_memory_barrier,
        };

        unsafe {
            vulkan_ffi::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    /// Clears the given pixel buffer to the specified color.
    /// Current layout of `buffer` must be `SHARED_PRESENT_KHR`, `GENERAL` or `TRANSFER_DST_OPTIMAL`.
    pub fn clear_pixel_buffer(&self, buffer: &PixelBuffer, color: [f32; 4]) {
        let clear_color = vulkan_ffi::VkClearColorValue { float32: color };
        let image_subresource_range = vulkan_ffi::VkImageSubresourceRange {
            aspectMask: vulkan_ffi::VkImageAspectFlags::COLOR_BIT as u32,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: 0,
            layerCount: 1,
        };
        unsafe {
            vulkan_ffi::vkCmdClearColorImage(
                self.handle,
                buffer.image,
                vulkan_ffi::VkImageLayout::GENERAL,
                &clear_color,
                1,
                &image_subresource_range,
            );
        }
    }
}

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

pub struct PixelBuffer {
    image: vulkan_ffi::VkImage,
    image_view: vulkan_ffi::VkImageView,
    allocation: vma_ffi::VmaAllocation,
    allocation_info: vma_ffi::VmaAllocationInfo, // TODO: Don't store this, retrieve on demand
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

impl PixelBuffer {
    pub fn device_memory(&self) -> vulkan_ffi::VkDeviceMemory {
        self.allocation_info.deviceMemory
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

pub struct ImageLayout(vulkan_ffi::VkImageLayout);
impl ImageLayout {
    pub const UNDEFINED: Self = Self(vulkan_ffi::VkImageLayout::UNDEFINED);
    pub const GENERAL: Self = Self(vulkan_ffi::VkImageLayout::GENERAL);
    pub const COLOR_ATTACHMENT_OPTIMAL: Self =
        Self(vulkan_ffi::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    pub const DEPTH_STENCIL_ATTACHMENT_OPTIMAL: Self =
        Self(vulkan_ffi::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    pub const TRANSFER_SRC: Self = Self(vulkan_ffi::VkImageLayout::TRANSFER_SRC_OPTIMAL);
    pub const TRANSFER_DST: Self = Self(vulkan_ffi::VkImageLayout::TRANSFER_DST_OPTIMAL);
    pub const SHADER_READ_ONLY_OPTIMAL: Self =
        Self(vulkan_ffi::VkImageLayout::SHADER_READ_ONLY_OPTIMAL);
    pub const PRESENT_SRC_KHR: Self = Self(vulkan_ffi::VkImageLayout::PRESENT_SRC_KHR);
    pub fn to_vk(&self) -> vulkan_ffi::VkImageLayout {
        self.0
    }
    pub fn from_vk(layout: vulkan_ffi::VkImageLayout) -> Self {
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
pub enum Outcome {
    Success,
    NotImplemented(i32),
    InvalidOperation(i32),
    NotSupported(i32),
    Timeout(i32),
    Unknown(i32),
}

impl std::error::Error for Outcome {}
impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Success => write!(f, "Success"),
            Outcome::InvalidOperation(code) => write!(f, "Invalid operation: {}", code),
            Outcome::NotSupported(code) => write!(f, "Not supported: {}", code),
            Outcome::NotImplemented(code) => write!(f, "Not implemented: {}", code),
            Outcome::Timeout(code) => write!(f, "Timeout: {}", code),
            Outcome::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

impl Outcome {
    pub fn to_code(&self) -> i32 {
        match self {
            Outcome::Success => 0,
            Outcome::InvalidOperation(code) => *code,
            Outcome::NotImplemented(code) => *code,
            Outcome::NotSupported(code) => *code,
            Outcome::Timeout(code) => *code,
            Outcome::Unknown(code) => *code,
        }
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Outcome::Success,
            -7 | -8 | -11 => Outcome::NotSupported(code), // Extension not present, feature not present, format not supported
            -4 => Outcome::InvalidOperation(code),        // Invalid operation
            -1 => Outcome::NotImplemented(code),          // Not implemented
            2 => Outcome::Timeout(code),                  // Timeout
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
