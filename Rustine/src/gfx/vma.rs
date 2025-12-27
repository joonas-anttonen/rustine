#![allow(dead_code)]

use std::sync::Arc;

use crate::gfx;
use crate::gfx::vma_ffi;
use crate::gfx::vulkan_ffi;
use crate::version::Version;
use crate::vk_call;
use crate::warning;

pub struct Allocator {
    pub handle: vma_ffi::VmaAllocator,
}

impl Drop for Allocator {
    fn drop(&mut self) {
        warning!("Allocator::drop");
        unsafe {
            vma_ffi::vmaDestroyAllocator(self.handle);
        }
    }
}

impl Allocator {
    pub fn new(
        instance: &crate::gfx::vulkan::Instance,
        device: &crate::gfx::vulkan::Device,
    ) -> std::result::Result<Arc<Self>, gfx::Result> {
        let mut flags = vma_ffi::VmaAllocatorCreateFlags::NONE as u32;
        // If Windows platform, enable external memory handle types
        if cfg!(target_os = "windows") {
            flags = flags | vma_ffi::VmaAllocatorCreateFlags::KHR_EXTERNAL_MEMORY_WIN32_BIT as u32;
        }

        let create_info = vma_ffi::VmaAllocatorCreateInfo {
            flags: flags,
            physicalDevice: device.physical_device_handle(),
            device: device.handle(),
            preferredLargeHeapBlockSize: 0,
            pAllocationCallbacks: std::ptr::null(),
            pDeviceMemoryCallbacks: std::ptr::null(),
            pHeapSizeLimit: std::ptr::null(),
            pVulkanFunctions: std::ptr::null(),
            instance: instance.handle(),
            vulkanApiVersion: Version::new(1, 4, 0).to_vk_version(),
            pTypeExternalMemoryHandleTypes: std::ptr::null(),
        };

        let mut allocator_handle: vma_ffi::VmaAllocator = std::ptr::null_mut();

        vk_call!(vma_ffi::vmaCreateAllocator(
            &create_info,
            &mut allocator_handle
        ))?;

        Ok(Arc::new(Allocator {
            handle: allocator_handle,
        }))
    }

    pub fn allocate_image(
        self: &Arc<Self>,
        image_create_info: &vulkan_ffi::VkImageCreateInfo,
        allocation_create_info: &vma_ffi::VmaAllocationCreateInfo,
    ) -> std::result::Result<ImageAllocation, gfx::Result> {
        let mut image: vulkan_ffi::VkImage = std::ptr::null_mut();
        let mut allocation: vma_ffi::VmaAllocation = std::ptr::null_mut();
        let mut allocation_info: vma_ffi::VmaAllocationInfo = unsafe { std::mem::zeroed() };

        vk_call!(vma_ffi::vmaCreateImage(
            self.handle,
            image_create_info,
            allocation_create_info,
            &mut image,
            &mut allocation,
            &mut allocation_info
        ))?;

        Ok(ImageAllocation {
            handle: allocation,
            allocator: Arc::clone(&self),
        })
    }
}

pub struct ImageAllocation {
    pub handle: vma_ffi::VmaAllocation,
    pub allocator: Arc<Allocator>,
}

impl Drop for ImageAllocation {
    fn drop(&mut self) {
        warning!("ImageAllocation::drop");
        unsafe {
            vma_ffi::vmaDestroyImage(self.allocator.handle, std::ptr::null_mut(), self.handle);
        }
    }
}

pub struct BufferAllocation {
    pub handle: vma_ffi::VmaAllocation,
    pub allocator: Arc<Allocator>,
}

impl Drop for BufferAllocation {
    fn drop(&mut self) {
        warning!("BufferAllocation::drop");
        unsafe {
            vma_ffi::vmaDestroyBuffer(self.allocator.handle, std::ptr::null_mut(), self.handle);
        }
    }
}
