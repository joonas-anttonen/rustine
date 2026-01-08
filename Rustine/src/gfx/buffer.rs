#![allow(dead_code)]

use crate::warning;
use crate::{gfx::allocator, gfx::vulkan};

use std::sync::Arc;

pub struct PixelBuffer {
    width: u32,
    height: u32,
    image: vulkan::VkImage,
    image_view: vulkan::VkImageView,
    allocation: allocator::VmaAllocation,
    allocator: Arc<allocator::Allocator>,
}

impl Eq for PixelBuffer {}
impl PartialEq for PixelBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.image == other.image
    }
}
impl std::hash::Hash for PixelBuffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.image.hash(state);
    }
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        warning!("PixelBuffer::drop");
        unsafe {
            vulkan::vkDestroyImageView(
                self.allocator.device().handle(),
                self.image_view,
                std::ptr::null(),
            );
            allocator::vmaDestroyImage(self.allocator.handle(), self.image, self.allocation);
        }
    }
}

impl PixelBuffer {
    pub fn new(
        width: u32,
        height: u32,
        image: vulkan::VkImage,
        image_view: vulkan::VkImageView,
        allocation: allocator::VmaAllocation,
        allocator: Arc<allocator::Allocator>,
    ) -> Self {
        Self {
            image,
            image_view,
            allocation,
            allocator,
            width,
            height,
        }
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
    pub handle: allocator::VmaAllocation,
    pub allocator: std::sync::Arc<allocator::Allocator>,
}

impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        warning!("MemoryBuffer::drop");
        unsafe {
            allocator::vmaDestroyBuffer(self.allocator.handle(), std::ptr::null_mut(), self.handle);
        }
    }
}
