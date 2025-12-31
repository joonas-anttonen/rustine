#![allow(dead_code)]

use crate::warning;
use crate::{gfx::vma, gfx::vma_ffi, gfx::vulkan_ffi};

use std::sync::Arc;

pub struct PixelBuffer {
    image: vulkan_ffi::VkImage,
    image_view: vulkan_ffi::VkImageView,
    allocation: vma_ffi::VmaAllocation,
    allocation_info: vma_ffi::VmaAllocationInfo, // TODO: Don't store this, retrieve on demand
    allocator: Arc<vma::Allocator>,
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
    pub fn new (
        image: vulkan_ffi::VkImage,
        image_view: vulkan_ffi::VkImageView,
        allocation: vma_ffi::VmaAllocation,
        allocation_info: vma_ffi::VmaAllocationInfo,
        allocator: Arc<vma::Allocator>,
    ) -> Self {
        Self {
            image,
            image_view,
            allocation,
            allocation_info,
            allocator,
        }
    }

    pub fn device_memory(&self) -> vulkan_ffi::VkDeviceMemory {
        self.allocation_info.deviceMemory
    }

    pub fn image(&self) -> vulkan_ffi::VkImage {
        self.image
    }

    pub fn image_view(&self) -> vulkan_ffi::VkImageView {
        self.image_view
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
