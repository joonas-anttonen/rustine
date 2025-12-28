#![allow(dead_code)]

use std::sync::Arc;

use crate::{gfx, gfx::vma_ffi, gfx::vulkan, gfx::vulkan_ffi};
use crate::{vk_call, warning};

pub struct Allocator {
    pub handle: vma_ffi::VmaAllocator,
    pub device: Arc<vulkan::Device>,
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
    pub fn new(instance: &vulkan::Instance, device: Arc<vulkan::Device>) -> gfx::Result<Arc<Self>> {
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
            vulkanApiVersion: gfx::MINIMUM_VULKAN_API_VERSION.to_vk_version(),
            pTypeExternalMemoryHandleTypes: std::ptr::null(),
        };

        let mut allocator_handle: vma_ffi::VmaAllocator = std::ptr::null_mut();

        vk_call!(vma_ffi::vmaCreateAllocator(
            &create_info,
            &mut allocator_handle
        ))?;

        Ok(Arc::new(Allocator {
            handle: allocator_handle,
            device: Arc::clone(&device),
        }))
    }

    pub fn create_pixel_buffer(
        self: &Arc<Self>,
        format: gfx::Format,
        width: u32,
        height: u32,
        usage: gfx::ImageUsage,
        aspect: gfx::ImageAspect,
        samples: gfx::ImageSamples,
    ) -> gfx::Result<gfx::PixelBuffer> {
        let image_create_info = vulkan_ffi::VkImageCreateInfo {
            sType: vulkan_ffi::VkStructureType::IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: vulkan_ffi::VkImageType::X2D,
            format: format.to_vk(),
            extent: vulkan_ffi::VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            samples: samples.0,
            tiling: vulkan_ffi::VkImageTiling::OPTIMAL,
            usage: usage.0,
            sharingMode: vulkan_ffi::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: vulkan_ffi::VkImageLayout::UNDEFINED,
        };
        let mut image_view_create_info = vulkan_ffi::VkImageViewCreateInfo {
            sType: vulkan_ffi::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            image: std::ptr::null_mut(), // NOTE: image not available yet, will be set in allocate_image
            viewType: vulkan_ffi::VkImageViewType::X2D,
            format: format.to_vk(),
            components: vulkan_ffi::VkComponentMapping {
                r: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                g: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                b: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                a: vulkan_ffi::VkComponentSwizzle::IDENTITY,
            },
            subresourceRange: vulkan_ffi::VkImageSubresourceRange {
                aspectMask: aspect.0,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };

        self.allocate_image(&image_create_info, &mut image_view_create_info)
    }

    fn allocate_image(
        self: &Arc<Self>,
        image_create_info: &vulkan_ffi::VkImageCreateInfo,
        image_view_create_info: &mut vulkan_ffi::VkImageViewCreateInfo,
    ) -> gfx::Result<gfx::PixelBuffer> {
        let mut image: vulkan_ffi::VkImage = std::ptr::null_mut();
        let mut image_view: vulkan_ffi::VkImageView = std::ptr::null_mut();
        let mut allocation: vma_ffi::VmaAllocation = std::ptr::null_mut();
        let mut allocation_info: vma_ffi::VmaAllocationInfo = unsafe { std::mem::zeroed() };

        let allocation_create_info = vma_ffi::VmaAllocationCreateInfo {
            flags: 0,
            usage: vma_ffi::VmaMemoryUsage::AUTO,
            requiredFlags: 0,
            preferredFlags: 0,
            memoryTypeBits: 0,
            pool: std::ptr::null_mut(),
            pUserData: std::ptr::null_mut(),
            priority: 0.0,
        };

        vk_call!(vma_ffi::vmaCreateImage(
            self.handle,
            image_create_info,
            &allocation_create_info,
            &mut image,
            &mut allocation,
            &mut allocation_info
        ))?;

        image_view_create_info.image = image; // Set the image now that it's created

        vk_call!(vulkan_ffi::vkCreateImageView(
            self.device.handle(),
            image_view_create_info,
            std::ptr::null(),
            &mut image_view
        ))
        .or_else(|err| {
            // If creating the image view fails, clean up the previously
            // created VMA image and allocation to avoid leaking resources.
            unsafe {
                vma_ffi::vmaDestroyImage(self.handle, image, allocation);
            }
            Err(err)
        })?;

        Ok(gfx::PixelBuffer {
            image,
            image_view,
            allocation,
            allocator: Arc::clone(&self),
        })
    }

    fn allocate_external_image(
        self: &Arc<Self>,
        handle: *const std::ffi::c_void,
        image_create_info: &mut vulkan_ffi::VkImageCreateInfo,
        image_view_create_info: &mut vulkan_ffi::VkImageViewCreateInfo,
    ) -> gfx::Result<gfx::PixelBuffer> {
        let external_image_create_info = vulkan_ffi::VkExternalMemoryImageCreateInfo {
            sType: vulkan_ffi::VkStructureType::EXTERNAL_MEMORY_IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            handleTypes: vulkan_ffi::VkExternalMemoryHandleTypeFlags::D3D11_TEXTURE_BIT as u32,
        };
        image_create_info.pNext = &external_image_create_info
            as *const vulkan_ffi::VkExternalMemoryImageCreateInfo
            as *const std::ffi::c_void;

        let import_memory_win32_info = vulkan_ffi::VkImportMemoryWin32HandleInfoKHR {
            sType: vulkan_ffi::VkStructureType::IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            handleType: vulkan_ffi::VkExternalMemoryHandleTypeFlags::D3D11_TEXTURE_BIT as u32,
            handle: handle as *mut std::ffi::c_void,
            name: std::ptr::null(),
        };

        let mut image: vulkan_ffi::VkImage = std::ptr::null_mut();
        let mut image_view: vulkan_ffi::VkImageView = std::ptr::null_mut();
        let mut allocation: vma_ffi::VmaAllocation = std::ptr::null_mut();
        let mut allocation_info: vma_ffi::VmaAllocationInfo = unsafe { std::mem::zeroed() };

        let allocation_create_info = vma_ffi::VmaAllocationCreateInfo {
            flags: 0,
            usage: vma_ffi::VmaMemoryUsage::AUTO,
            requiredFlags: 0,
            preferredFlags: 0,
            memoryTypeBits: 0,
            pool: std::ptr::null_mut(),
            pUserData: std::ptr::null_mut(),
            priority: 0.0,
        };

        vk_call!(vma_ffi::vmaCreateDedicatedImage(
            self.handle,
            image_create_info,
            &allocation_create_info,
            &import_memory_win32_info as *const vulkan_ffi::VkImportMemoryWin32HandleInfoKHR
                as *const std::ffi::c_void,
            &mut image,
            &mut allocation,
            &mut allocation_info
        ))?;

        image_view_create_info.image = image; // Set the image now that it's created

        vk_call!(vulkan_ffi::vkCreateImageView(
            self.device.handle(),
            image_view_create_info,
            std::ptr::null(),
            &mut image_view
        ))
        .or_else(|err| {
            // If creating the image view fails, clean up the previously
            // created VMA image and allocation to avoid leaking resources.
            unsafe {
                vma_ffi::vmaDestroyImage(self.handle, image, allocation);
            }
            Err(err)
        })?;

        Ok(gfx::PixelBuffer {
            image,
            image_view,
            allocation,
            allocator: Arc::clone(&self),
        })
    }

    pub fn create_external_pixel_buffer(
        self: &Arc<Self>,
        format: gfx::Format,
        width: u32,
        height: u32,
        usage: gfx::ImageUsage,
        aspect: gfx::ImageAspect,
        handle: *const std::ffi::c_void,
    ) -> gfx::Result<gfx::PixelBuffer> {
        let mut image_create_info = vulkan_ffi::VkImageCreateInfo {
            sType: vulkan_ffi::VkStructureType::IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: vulkan_ffi::VkImageType::X2D,
            format: format.to_vk(),
            extent: vulkan_ffi::VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            samples: gfx::ImageSamples::X1.0,
            tiling: vulkan_ffi::VkImageTiling::OPTIMAL,
            usage: usage.0,
            sharingMode: vulkan_ffi::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: vulkan_ffi::VkImageLayout::UNDEFINED,
        };
        let mut image_view_create_info = vulkan_ffi::VkImageViewCreateInfo {
            sType: vulkan_ffi::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            image: std::ptr::null_mut(), // NOTE: image not available yet, will be set in allocate_image
            viewType: vulkan_ffi::VkImageViewType::X2D,
            format: format.to_vk(),
            components: vulkan_ffi::VkComponentMapping {
                r: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                g: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                b: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                a: vulkan_ffi::VkComponentSwizzle::IDENTITY,
            },
            subresourceRange: vulkan_ffi::VkImageSubresourceRange {
                aspectMask: aspect.0,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };

        self.allocate_external_image(handle, &mut image_create_info, &mut image_view_create_info)
    }
}
