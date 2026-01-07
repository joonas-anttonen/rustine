#![allow(dead_code)]

use crate::warning;
use crate::{gfx::vma, gfx::vulkan};

use std::sync::Arc;

pub struct PixelBuffer {
    width: u32,
    height: u32,
    image: vulkan::VkImage,
    image_view: vulkan::VkImageView,
    allocation: vma::VmaAllocation,
    allocation_info: vma::VmaAllocationInfo, // TODO: Don't store this, retrieve on demand
    allocator: Arc<vma::Allocator>,
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        warning!("PixelBuffer::drop");
        unsafe {
            vulkan::vkDestroyImageView(
                self.allocator.device.handle(),
                self.image_view,
                std::ptr::null(),
            );
            vma::vmaDestroyImage(self.allocator.handle, self.image, self.allocation);
        }
    }
}

impl PixelBuffer {
    pub fn new (
        width: u32,
        height: u32,
        image: vulkan::VkImage,
        image_view: vulkan::VkImageView,
        allocation: vma::VmaAllocation,
        allocation_info: vma::VmaAllocationInfo,
        allocator: Arc<vma::Allocator>,
    ) -> Self {
        Self {
            image,
            image_view,
            allocation,
            allocation_info,
            allocator,
            width,
            height,
        }
    }

    pub fn device_memory(&self) -> vulkan::VkDeviceMemory {
        self.allocation_info.deviceMemory
    }

    pub fn image(&self) -> vulkan::VkImage {
        self.image
    }

    pub fn image_view(&self) -> vulkan::VkImageView {
        self.image_view
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

pub struct MemoryBuffer {
    pub handle: vma::VmaAllocation,
    pub allocator: std::sync::Arc<vma::Allocator>,
}

impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        warning!("MemoryBuffer::drop");
        unsafe {
            vma::vmaDestroyBuffer(self.allocator.handle, std::ptr::null_mut(), self.handle);
        }
    }
}
