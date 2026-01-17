#![allow(dead_code)]

use crate::vk_call;
use crate::warning;
use crate::{gfx::allocator, gfx::vulkan as vk};

use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct PixelBuffer {
    width: u32,
    height: u32,
    format: crate::gfx::Format,
    image: vk::VkImage,
    image_view: vk::VkImageView,
    allocation: allocator::VmaAllocation,
    allocator: Rc<allocator::Allocator>,
    layout: AtomicU32,
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
            vk::vkDestroyImageView(
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
        format: crate::gfx::Format,
        image: vk::VkImage,
        image_view: vk::VkImageView,
        allocation: allocator::VmaAllocation,
        allocator: Rc<allocator::Allocator>,
    ) -> Self {
        Self {
            image,
            image_view,
            allocation,
            allocator,
            width,
            height,
            format,
            layout: AtomicU32::new(crate::gfx::Layout::UNDEFINED.to_raw_u32()),
        }
    }

    pub fn image(&self) -> vk::VkImage {
        self.image
    }

    pub fn image_view(&self) -> vk::VkImageView {
        self.image_view
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn size(&self) -> crate::Vector2i {
        crate::Vector2i { x: self.width as i32, y: self.height as i32 }
    }

    pub fn format(&self) -> crate::gfx::Format {
        self.format
    }

    pub fn is_defined(&self) -> bool {
        self.layout.load(Ordering::Acquire) != crate::gfx::Layout::UNDEFINED.to_raw_u32()
    }

    /// Returns the current layout of the image.
    pub fn layout(&self) -> crate::gfx::Layout {
        let raw = self.layout.load(Ordering::Acquire);
        crate::gfx::Layout::from_raw_u32(raw)
    }

    /// Sets the current layout of the image. Called after barrier operations.
    pub fn set_layout(&self, layout: crate::gfx::Layout) {
        self.layout.store(layout.to_raw_u32(), Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct MemoryUsage(vk::VkBufferUsageFlags);
impl std::ops::BitOr for MemoryUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl MemoryUsage {
    pub const VERTEX_BUFFER: Self = Self(vk::VkBufferUsageFlags::VERTEX_BUFFER_BIT);
    pub const INDEX_BUFFER: Self = Self(vk::VkBufferUsageFlags::INDEX_BUFFER_BIT);
    pub const UNIFORM_BUFFER: Self = Self(vk::VkBufferUsageFlags::UNIFORM_BUFFER_BIT);
    pub const TRANSFER_SRC: Self = Self(vk::VkBufferUsageFlags::TRANSFER_SRC_BIT);
    pub const TRANSFER_DST: Self = Self(vk::VkBufferUsageFlags::TRANSFER_DST_BIT);
    pub const INDIRECT_BUFFER: Self = Self(vk::VkBufferUsageFlags::INDIRECT_BUFFER_BIT);
    pub const STORAGE_BUFFER: Self = Self(vk::VkBufferUsageFlags::STORAGE_BUFFER_BIT);
    pub fn to_vk(&self) -> vk::VkBufferUsageFlags {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum MemoryAccess {
    NONE,
    READ,
    WRITE,
    READ_WRITE,
}

pub struct MemoryBuffer {
    pub usage: MemoryUsage,
    pub access: MemoryAccess,
    handle: vk::VkBuffer,
    allocation: allocator::VmaAllocation,
    allocator: std::rc::Rc<allocator::Allocator>,
}

impl Eq for MemoryBuffer {}
impl PartialEq for MemoryBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}
impl std::hash::Hash for MemoryBuffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl MemoryBuffer {
    pub fn handle(&self) -> vk::VkBuffer {
        self.handle
    }

    pub fn allocation(&self) -> allocator::VmaAllocation {
        self.allocation
    }

    pub fn new(
        usage: MemoryUsage,
        access: MemoryAccess,
        handle: vk::VkBuffer,
        allocation: allocator::VmaAllocation,
        allocator: std::rc::Rc<allocator::Allocator>,
    ) -> Self {
        Self {
            usage,
            access,
            handle,
            allocation,
            allocator,
        }
    }

    /// Writes the specified data to the memory buffer.
    ///
    /// This is only valid if the memory buffer has write access.
    pub fn write<T>(&self, data: &[T]) {
        self.write_at(0, data);
    }

    /// Writes the specified data to the memory buffer starting at the given memory offset.
    ///
    /// This is only valid if the memory buffer has write access.
    pub fn write_at<T>(&self, memory_offset: usize, data: &[T]) {
        let data_ptr = data.as_ptr() as *const std::ffi::c_void;
        let data_size = std::mem::size_of_val(data);

        if self.access == MemoryAccess::NONE || self.access == MemoryAccess::READ {
            panic!("No write access to memory buffer");
        }

        let mut allocation_info: allocator::VmaAllocationInfo = unsafe { std::mem::zeroed() };
        unsafe {
            allocator::vmaGetAllocationInfo(
                self.allocator.handle(),
                self.allocation,
                &mut allocation_info,
            )
        };

        // Ensure size makes sense
        assert!(memory_offset + data_size <= allocation_info.size.0 as usize);

        vk_call!(allocator::vmaCopyMemoryToAllocation(
            self.allocator.handle(),
            data_ptr,
            self.allocation,
            vk::VkDeviceSize(memory_offset as u64),
            vk::VkDeviceSize(data_size as u64),
        ))
        .unwrap();
    }
}

impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        warning!("MemoryBuffer::drop");
        unsafe {
            allocator::vmaDestroyBuffer(self.allocator.handle(), self.handle, self.allocation);
        }
    }
}
