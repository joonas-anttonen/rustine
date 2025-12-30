use crate::{error, vk_call, warning};
use crate::{gfx::PixelBuffer, gfx::vulkan, gfx::vulkan_ffi, lib_ffi::PresentationParameters};

use std::sync::Arc;

pub enum AcquireStatus {
    Success(PresentationImage),
    Timeout,
    OutOfDate,
    Error(crate::gfx::Status),
}

pub enum PresentationMethod {
    Headless,
    SharedImage,
    Swapchain,
}

#[derive(Copy, Clone)]
pub struct PresentationImage {
    pub memory: vulkan_ffi::VkDeviceMemory,
    pub image: vulkan_ffi::VkImage,
    pub image_view: vulkan_ffi::VkImageView,
    pub format: vulkan_ffi::VkFormat,
    pub index: u32,
    pub swapchain_handle: vulkan_ffi::VkSwapchainKHR,
}

pub trait PresentationProvider {
    fn acquire(&self) -> AcquireStatus;
}

pub struct SharedImageProvider {
    output_frame: PixelBuffer,
}
impl SharedImageProvider {
    pub fn new(output_frame: PixelBuffer) -> Self {
        SharedImageProvider { output_frame }
    }
}

impl PresentationProvider for SharedImageProvider {
    fn acquire(&self) -> AcquireStatus {
        let presentation_image = PresentationImage {
            memory: self.output_frame.device_memory(),
            image: self.output_frame.image(),
            image_view: self.output_frame.image_view(),
            format: vulkan_ffi::VkFormat::B8G8R8A8_UNORM,
            index: 0,
            swapchain_handle: std::ptr::null_mut(),
        };
        AcquireStatus::Success(presentation_image)
    }
}

pub struct SwapchainProvider {
    swapchain_handle: vulkan_ffi::VkSwapchainKHR,
    swapchain_images: Vec<PresentationImage>,
    acquire_fence: vulkan_ffi::VkFence,
    device: Arc<vulkan::Device>,
}

impl Drop for SwapchainProvider {
    fn drop(&mut self) {
        unsafe {
            vulkan_ffi::vkDestroyFence(self.device.handle(), self.acquire_fence, std::ptr::null());

            for presentation_image in &self.swapchain_images {
                if !presentation_image.image_view.is_null() {
                    vulkan_ffi::vkDestroyImageView(
                        self.device.handle(),
                        presentation_image.image_view,
                        std::ptr::null(),
                    );
                }
            }
            if !self.swapchain_handle.is_null() {
                vulkan_ffi::vkDestroySwapchainKHR(
                    self.device.handle(),
                    self.swapchain_handle,
                    std::ptr::null(),
                );
            }
        }
    }
}

impl PresentationProvider for SwapchainProvider {
    fn acquire(&self) -> AcquireStatus {
        let mut image_index: u32 = 0;
        let result = unsafe {
            vulkan_ffi::vkAcquireNextImageKHR(
                self.device.handle(),
                self.swapchain_handle,
                std::u64::MAX,
                std::ptr::null_mut(),
                self.acquire_fence,
                &mut image_index,
            )
        };

        // Reset the acquire fence
        unsafe {
            vulkan_ffi::vkWaitForFences(
                self.device.handle(),
                1,
                &self.acquire_fence,
                vulkan_ffi::VK_TRUE,
                std::u64::MAX,
            );
            vulkan_ffi::vkResetFences(self.device.handle(), 1, &self.acquire_fence);
        }

        match result {
            vulkan_ffi::VkResult::SUCCESS | vulkan_ffi::VkResult::SUBOPTIMAL_KHR => {
                let presentation_image = self.swapchain_images[image_index as usize];
                AcquireStatus::Success(presentation_image)
            }
            vulkan_ffi::VkResult::TIMEOUT => AcquireStatus::Timeout,
            vulkan_ffi::VkResult::OUT_OF_DATE_KHR => AcquireStatus::OutOfDate,
            _ => AcquireStatus::Error(crate::gfx::Status::from_code(result.0)),
        }
    }
}

impl SwapchainProvider {
    pub fn new(device: &Arc<vulkan::Device>, params: PresentationParameters) -> Self {
        let physical_device_handle = device.physical_device_handle();
        let surface_handle = params.surface_handle as vulkan_ffi::VkSurfaceKHR;

        let mut surface_capabilities: vulkan_ffi::VkSurfaceCapabilitiesKHR =
            unsafe { std::mem::zeroed() };
        vk_call!(vulkan_ffi::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
            physical_device_handle,
            surface_handle,
            &mut surface_capabilities
        ))
        .unwrap();

        let surface_formats = vulkan::enumerate_physical_device_surface_formats(
            physical_device_handle,
            surface_handle,
        )
        .unwrap();
        let surface_present_modes = vulkan::enumerate_physical_device_surface_present_modes(
            physical_device_handle,
            surface_handle,
        )
        .unwrap();

        let chosen_extent = match surface_capabilities.currentExtent.width {
            std::u32::MAX => vulkan_ffi::VkExtent2D {
                width: params.width,
                height: params.height,
            },
            _ => surface_capabilities.currentExtent,
        };

        // Find B8G8R8A8_UNORM format
        let mut chosen_format = None;
        for format in &surface_formats {
            if format.format == vulkan_ffi::VkFormat::B8G8R8A8_UNORM
                && format.colorSpace == vulkan_ffi::VkColorSpaceKHR::SRGB_NONLINEAR_KHR
            {
                chosen_format = Some(*format);
                break;
            }
        }
        let chosen_format = match chosen_format {
            Some(format) => format,
            None => surface_formats[0],
        };

        let mut chosen_present_mode = vulkan_ffi::VkPresentModeKHR::FIFO_KHR;
        if params.vertical_sync == 0 {
            for &present_mode in &surface_present_modes {
                if present_mode == vulkan_ffi::VkPresentModeKHR::MAILBOX_KHR {
                    chosen_present_mode = present_mode;
                    break;
                } else if present_mode == vulkan_ffi::VkPresentModeKHR::IMMEDIATE_KHR {
                    chosen_present_mode = present_mode;
                }
            }
        }

        let chosen_image_count = if surface_capabilities.maxImageCount > 0 {
            std::cmp::min(
                surface_capabilities.minImageCount + 1,
                surface_capabilities.maxImageCount,
            )
        } else {
            surface_capabilities.minImageCount + 1
        };

        // At least COLOR_ATTACHMENT and TRANSFER_DST usage must be supported
        let chosen_image_usage = vulkan_ffi::VkImageUsageFlags::COLOR_ATTACHMENT_BIT
            | vulkan_ffi::VkImageUsageFlags::TRANSFER_DST_BIT;
        if (surface_capabilities.supportedUsageFlags & chosen_image_usage) != chosen_image_usage {
            error!("Required image usage flags not supported by surface");
        }

        let swapchain_create_info = vulkan_ffi::VkSwapchainCreateInfoKHR {
            sType: vulkan_ffi::VkStructureType::SWAPCHAIN_CREATE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            flags: 0,
            surface: surface_handle,
            minImageCount: chosen_image_count,
            imageFormat: chosen_format.format,
            imageColorSpace: chosen_format.colorSpace,
            imageExtent: chosen_extent,
            imageArrayLayers: 1,
            imageUsage: chosen_image_usage,
            imageSharingMode: vulkan_ffi::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            preTransform: vulkan_ffi::VkSurfaceTransformFlagsKHR::IDENTITY_BIT_KHR,
            compositeAlpha: vulkan_ffi::VkCompositeAlphaFlagsKHR::OPAQUE_BIT_KHR,
            presentMode: chosen_present_mode,
            clipped: 1,
            oldSwapchain: std::ptr::null_mut(),
        };

        let mut swapchain_handle: vulkan_ffi::VkSwapchainKHR = std::ptr::null_mut();
        vk_call!(vulkan_ffi::vkCreateSwapchainKHR(
            device.handle(),
            &swapchain_create_info,
            std::ptr::null(),
            &mut swapchain_handle
        ))
        .map_err(|err| {
            error!("Failed to create swapchain: {:?}", err);
            err
        })
        .unwrap();

        let swapchain_images: Vec<PresentationImage> =
            vulkan::enumerate_swapchain_images(device.handle(), swapchain_handle)
                .unwrap()
                .into_iter()
                .enumerate()
                .map(|(index, img_handle)| {
                    // Create image view for each swapchain image
                    let image_view_create_info = vulkan_ffi::VkImageViewCreateInfo {
                        sType: vulkan_ffi::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
                        pNext: std::ptr::null(),
                        flags: 0,
                        image: img_handle,
                        viewType: vulkan_ffi::VkImageViewType::X2D,
                        format: chosen_format.format,
                        components: vulkan_ffi::VkComponentMapping {
                            r: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                            g: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                            b: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                            a: vulkan_ffi::VkComponentSwizzle::IDENTITY,
                        },
                        subresourceRange: vulkan_ffi::VkImageSubresourceRange {
                            aspectMask: vulkan_ffi::VkImageAspectFlags::COLOR_BIT as u32,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    };

                    let mut image_view_handle: vulkan_ffi::VkImageView = std::ptr::null_mut();
                    vk_call!(vulkan_ffi::vkCreateImageView(
                        device.handle(),
                        &image_view_create_info,
                        std::ptr::null(),
                        &mut image_view_handle
                    ))
                    .unwrap();

                    PresentationImage {
                        memory: std::ptr::null_mut(),
                        image: img_handle,
                        image_view: image_view_handle,
                        format: chosen_format.format,
                        index: index as u32,
                        swapchain_handle,
                    }
                })
                .collect();

        let fence_create_info = vulkan_ffi::VkFenceCreateInfo {
            sType: vulkan_ffi::VkStructureType::FENCE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut acquire_fence: vulkan_ffi::VkFence = std::ptr::null_mut();
        vk_call!(vulkan_ffi::vkCreateFence(
            device.handle(),
            &fence_create_info,
            std::ptr::null(),
            &mut acquire_fence
        ))
        .unwrap();

        // DEBUG: Log swapchain information
        warning!(
            "Created swapchain: {} images, format: {:?}, present mode: {:?}",
            swapchain_images.len(),
            chosen_format.format,
            chosen_present_mode,
        );

        SwapchainProvider {
            swapchain_handle,
            swapchain_images,
            acquire_fence,
            device: Arc::clone(&device),
        }
    }
}
