use crate::{error, vk_call, warning};
use crate::{gfx::PixelBuffer, gfx::vulkan as vk, gfx::*};

use std::sync::Arc;

pub enum AcquireStatus {
    Success(PresentationImage),
    Timeout,
    OutOfDate,
    Error(crate::gfx::Status),
}

pub enum Method {
    Headless,
    SharedImage,
    Swapchain,
}

/// Parameters for initializing a presentation provider for FFI and internal use.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Parameters {
    pub width: u32,
    pub height: u32,
    /// Opaque handle to the presentation surface. `VkSurfaceKHR`, `HANDLE` or similar depending on platform and usage.
    pub surface_handle: *const std::ffi::c_void,
    pub vertical_sync: u32,
}

#[derive(Copy, Clone)]
pub struct PresentationImage {
    pub width: u32,
    pub height: u32,
    pub image: vk::VkImage,
    pub image_view: vk::VkImageView,
    pub format: vk::VkFormat,
    pub index: u32,
    pub swapchain_handle: vk::VkSwapchainKHR,
    pub acquire_semaphore: vk::VkSemaphore,
}

pub trait PresentationProvider {
    fn method(&self) -> Method;
    fn image_count(&self) -> u32;
    fn acquire(&mut self) -> AcquireStatus;
}

pub struct SharedImageProvider {
    output_frame: PixelBuffer,
}
impl SharedImageProvider {
    pub fn new(allocator: &Arc<allocator::Allocator>, parameters: Parameters) -> Self {
        let output_frame = allocator
            .create_external_pixel_buffer(
                Format::B8G8R8A8_UNORM,
                parameters.width,
                parameters.height,
                ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                ImageAspect::COLOR,
                parameters.surface_handle,
            )
            .unwrap();
        SharedImageProvider { output_frame }
    }
}

impl PresentationProvider for SharedImageProvider {
    fn method(&self) -> Method {
        Method::SharedImage
    }

    fn image_count(&self) -> u32 {
        1
    }

    fn acquire(&mut self) -> AcquireStatus {
        let presentation_image = PresentationImage {
            width: self.output_frame.width(),
            height: self.output_frame.height(),
            image: self.output_frame.image(),
            image_view: self.output_frame.image_view(),
            format: vk::VkFormat::B8G8R8A8_UNORM,
            index: 0,
            swapchain_handle: vk::VkSwapchainKHR::default(),
            acquire_semaphore: vk::VkSemaphore::default(),
        };
        AcquireStatus::Success(presentation_image)
    }
}

pub struct SwapchainProvider {
    swapchain_handle: vk::VkSwapchainKHR,
    swapchain_images: Vec<PresentationImage>,
    current_acquire_index: u32,
    acquire_fence: vk::VkFence,
    acquire_semaphores: Vec<vk::VkSemaphore>,
    device: Arc<Device>,
}

impl Drop for SwapchainProvider {
    fn drop(&mut self) {
        unsafe {
            vk::vkDestroyFence(self.device.handle(), self.acquire_fence, std::ptr::null());
            for semaphore in &self.acquire_semaphores {
                vk::vkDestroySemaphore(self.device.handle(), *semaphore, std::ptr::null());
            }

            for presentation_image in &self.swapchain_images {
                if !presentation_image.image_view.is_null() {
                    vk::vkDestroyImageView(
                        self.device.handle(),
                        presentation_image.image_view,
                        std::ptr::null(),
                    );
                }
            }
            if !self.swapchain_handle.is_null() {
                vk::vkDestroySwapchainKHR(
                    self.device.handle(),
                    self.swapchain_handle,
                    std::ptr::null(),
                );
            }
        }
    }
}

impl PresentationProvider for SwapchainProvider {
    fn method(&self) -> Method {
        Method::Swapchain
    }

    fn image_count(&self) -> u32 {
        self.swapchain_images.len() as u32
    }

    fn acquire(&mut self) -> AcquireStatus {
        let previous_acquire_index = if self.current_acquire_index == 0 {
            (self.acquire_semaphores.len() - 1) as u32
        } else {
            self.current_acquire_index - 1
        };
        let acquire_semaphore = self.acquire_semaphores[previous_acquire_index as usize];

        let mut image_index: u32 = 0;
        let result = unsafe {
            vk::vkAcquireNextImageKHR(
                self.device.handle(),
                self.swapchain_handle,
                std::u64::MAX,
                acquire_semaphore,
                vk::VkFence::default(),
                &mut image_index,
            )
        };

        // Reset the acquire fence
        /*unsafe {
            vk::vkWaitForFences(
                self.device.handle(),
                1,
                &self.acquire_fence,
                vk::VK_TRUE,
                std::u64::MAX,
            );
            vk::vkResetFences(self.device.handle(), 1, &self.acquire_fence);
        }*/

        match result {
            vk::VkResult::SUCCESS | vk::VkResult::SUBOPTIMAL_KHR => {
                let mut presentation_image = self.swapchain_images[image_index as usize];
                self.current_acquire_index += 1;
                if self.current_acquire_index >= self.acquire_semaphores.len() as u32 {
                    self.current_acquire_index = 0;
                }
                presentation_image.acquire_semaphore = acquire_semaphore;

                AcquireStatus::Success(presentation_image)
            }
            vk::VkResult::TIMEOUT => AcquireStatus::Timeout,
            vk::VkResult::OUT_OF_DATE_KHR => AcquireStatus::OutOfDate,
            _ => AcquireStatus::Error(crate::gfx::Status::from_code(result.0)),
        }
    }
}

impl SwapchainProvider {
    pub fn new(device: &Arc<Device>, params: Parameters) -> Self {
        let physical_device = device.physical_device();
        let surface_handle = vk::VkSurfaceKHR(params.surface_handle);

        let surface_capabilities = physical_device
            .get_surface_capabilities(surface_handle)
            .unwrap();

        let surface_formats = physical_device.get_surface_formats(surface_handle).unwrap();
        let surface_present_modes = physical_device
            .get_surface_present_modes(surface_handle)
            .unwrap();

        let chosen_extent = match surface_capabilities.currentExtent.width {
            std::u32::MAX => vk::VkExtent2D {
                width: params.width,
                height: params.height,
            },
            _ => surface_capabilities.currentExtent,
        };

        // Find B8G8R8A8_UNORM format
        let mut chosen_format = None;
        for format in &surface_formats {
            if format.format == vk::VkFormat::B8G8R8A8_UNORM
                && format.colorSpace == vk::VkColorSpaceKHR::SRGB_NONLINEAR_KHR
            {
                chosen_format = Some(*format);
                break;
            }
        }
        let chosen_format = match chosen_format {
            Some(format) => format,
            None => surface_formats[0],
        };

        let mut chosen_present_mode = vk::VkPresentModeKHR::FIFO;
        if params.vertical_sync == 0 {
            for &present_mode in &surface_present_modes {
                if present_mode == vk::VkPresentModeKHR::MAILBOX {
                    chosen_present_mode = present_mode;
                    break;
                } else if present_mode == vk::VkPresentModeKHR::IMMEDIATE {
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
        let chosen_image_usage =
            vk::VkImageUsageFlags::COLOR_ATTACHMENT_BIT | vk::VkImageUsageFlags::TRANSFER_DST_BIT;
        if (surface_capabilities.supportedUsageFlags & chosen_image_usage) != chosen_image_usage {
            error!("Required image usage flags not supported by surface");
        }

        let swapchain_create_info = vk::VkSwapchainCreateInfoKHR {
            sType: vk::VkStructureType::SWAPCHAIN_CREATE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            flags: 0,
            surface: surface_handle,
            minImageCount: chosen_image_count,
            imageFormat: chosen_format.format,
            imageColorSpace: chosen_format.colorSpace,
            imageExtent: chosen_extent,
            imageArrayLayers: 1,
            imageUsage: chosen_image_usage,
            imageSharingMode: vk::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            preTransform: vk::VkSurfaceTransformFlagsKHR::IDENTITY_BIT_KHR,
            compositeAlpha: vk::VkCompositeAlphaFlagsKHR::OPAQUE_BIT_KHR,
            presentMode: chosen_present_mode,
            clipped: 1,
            oldSwapchain: vk::VkSwapchainKHR::default(),
        };

        let mut swapchain_handle: vk::VkSwapchainKHR = vk::VkSwapchainKHR::default();
        vk_call!(vk::vkCreateSwapchainKHR(
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

        let mut acquire_semaphores = Vec::new();
        let swapchain_images: Vec<PresentationImage> = device
            .get_swapchain_images(swapchain_handle)
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(index, img_handle)| {
                // Create image view for each swapchain image
                let image_view_create_info = vk::VkImageViewCreateInfo {
                    sType: vk::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
                    pNext: std::ptr::null(),
                    flags: 0,
                    image: img_handle,
                    viewType: vk::VkImageViewType::X2D,
                    format: chosen_format.format,
                    components: vk::VkComponentMapping {
                        r: vk::VkComponentSwizzle::IDENTITY,
                        g: vk::VkComponentSwizzle::IDENTITY,
                        b: vk::VkComponentSwizzle::IDENTITY,
                        a: vk::VkComponentSwizzle::IDENTITY,
                    },
                    subresourceRange: vk::VkImageSubresourceRange {
                        aspectMask: vk::VkImageAspectFlags::COLOR_BIT as u32,
                        baseMipLevel: 0,
                        levelCount: 1,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                };

                let mut image_view_handle: vk::VkImageView = vk::VkImageView::default();
                vk_call!(vk::vkCreateImageView(
                    device.handle(),
                    &image_view_create_info,
                    std::ptr::null(),
                    &mut image_view_handle
                ))
                .unwrap();

                let mut acquire_semaphore: vk::VkSemaphore = vk::VkSemaphore::default();
                let semaphore_create_info = vk::VkSemaphoreCreateInfo {
                    sType: vk::VkStructureType::SEMAPHORE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                };
                vk_call!(vk::vkCreateSemaphore(
                    device.handle(),
                    &semaphore_create_info,
                    std::ptr::null(),
                    &mut acquire_semaphore
                ))
                .unwrap();

                acquire_semaphores.push(acquire_semaphore);

                PresentationImage {
                    width: chosen_extent.width,
                    height: chosen_extent.height,
                    image: img_handle,
                    image_view: image_view_handle,
                    format: chosen_format.format,
                    index: index as u32,
                    swapchain_handle,
                    acquire_semaphore: vk::VkSemaphore::default(),
                }
            })
            .collect();

        let fence_create_info = vk::VkFenceCreateInfo {
            sType: vk::VkStructureType::FENCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut acquire_fence: vk::VkFence = vk::VkFence::default();
        vk_call!(vk::vkCreateFence(
            device.handle(),
            &fence_create_info,
            std::ptr::null(),
            &mut acquire_fence
        ))
        .unwrap();

        // DEBUG: Log swapchain information
        warning!(
            "{}x{}x{} {:?} {:?}",
            chosen_extent.width,
            chosen_extent.height,
            swapchain_images.len(),
            chosen_format.format,
            chosen_present_mode,
        );

        SwapchainProvider {
            swapchain_handle,
            swapchain_images,
            current_acquire_index: 0,
            acquire_fence,
            acquire_semaphores,
            device: Arc::clone(&device),
        }
    }
}
