#![allow(dead_code)]

use crate::{error, vk_call, warning};
use crate::{gfx::vulkan, gfx::vulkan_ffi, gfx::*};

use std::sync::Arc;

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
    pub fn new(family_index: u32, device: &Arc<vulkan::Device>) -> Arc<Self> {
        let command_pool_create_info = vulkan_ffi::VkCommandPoolCreateInfo {
            sType: vulkan_ffi::VkStructureType::COMMAND_POOL_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: vulkan_ffi::VkCommandPoolCreateFlags::TRANSIENT_BIT
                | vulkan_ffi::VkCommandPoolCreateFlags::RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: family_index,
        };

        let mut command_pool_handle: vulkan_ffi::VkCommandPool = std::ptr::null_mut();
        vk_call!(vulkan_ffi::vkCreateCommandPool(
            device.handle(),
            &command_pool_create_info,
            std::ptr::null(),
            &mut command_pool_handle,
        ))
        .map_err(|err| error!("vkCreateCommandPool {:?}", err))
        .unwrap();

        Arc::new(CommandPool {
            handle: command_pool_handle,
            device: Arc::clone(&device),
        })
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

    pub fn handle(&self) -> vulkan_ffi::VkCommandBuffer {
        self.handle
    }

    pub fn semaphore(&self) -> vulkan_ffi::VkSemaphore {
        self.semaphore
    }

    pub fn fence(&self) -> vulkan_ffi::VkFence {
        self.fence
    }

    pub fn is_complete(&self) -> bool {
        let status = unsafe { vulkan_ffi::vkGetFenceStatus(self.pool.device.handle(), self.fence) };
        match status {
            vulkan_ffi::VkResult::SUCCESS => true,
            vulkan_ffi::VkResult::NOT_READY => false,
            _ => {
                error!("Failed to get fence status: {:?}", status);
                false
            }
        }
    }

    pub fn wait_for_completion(&self, timeout_ns: u64) -> bool {
        let result = unsafe {
            vulkan_ffi::vkWaitForFences(
                self.pool.device.handle(),
                1,
                &self.fence,
                vulkan_ffi::VK_TRUE,
                timeout_ns,
            )
        };
        match result {
            vulkan_ffi::VkResult::SUCCESS => true,
            vulkan_ffi::VkResult::TIMEOUT => false,
            _ => {
                error!("Failed to wait for fence: {:?}", result);
                false
            }
        }
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

    pub fn present_image_barrier(
        &self,
        image: &presentation::PresentationImage,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    ) {
        self.raw_image_barrier(image.image, old_layout, new_layout);
    }

    pub fn image_barrier(
        &self,
        buffer: &PixelBuffer,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    ) {
        self.raw_image_barrier(buffer.image(), old_layout, new_layout);
    }

    fn raw_image_barrier(
        &self,
        image: vulkan_ffi::VkImage,
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
            image: image,
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

    pub fn clear_present_image(&self, image: &presentation::PresentationImage, color: [f32; 4]) {
        self.raw_clear_pixel_buffer(image.image, color);
    }

    /// Clears the given pixel buffer to the specified color.
    /// Current layout of `buffer` must be `SHARED_PRESENT_KHR`, `GENERAL` or `TRANSFER_DST_OPTIMAL`.
    pub fn clear_pixel_buffer(&self, buffer: &PixelBuffer, color: [f32; 4]) {
        self.raw_clear_pixel_buffer(buffer.image(), color);
    }

    pub fn raw_clear_pixel_buffer(&self, image: vulkan_ffi::VkImage, color: [f32; 4]) {
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
                image,
                vulkan_ffi::VkImageLayout::GENERAL,
                &clear_color,
                1,
                &image_subresource_range,
            );
        }
    }
}
