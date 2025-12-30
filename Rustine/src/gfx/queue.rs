#![allow(dead_code)]

use crate::{error, gfx::vulkan, gfx::vulkan_ffi, vk_call, warning};
use std::sync::Arc;

use crate::gfx::{CommandBuffer, CommandPool, PixelBuffer};

pub enum SubmitStatus {
    Success,
    Timeout,
    Error(crate::gfx::Status),
}

pub struct Queue {
    handle: vulkan_ffi::VkQueue,
    family_index: u32,
    device: Arc<vulkan::Device>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        warning!("Queue::drop");
    }
}

impl Queue {
    pub fn new(
        handle: vulkan_ffi::VkQueue,
        family_index: u32,
        device: Arc<vulkan::Device>,
    ) -> Arc<Self> {
        Arc::new(Queue {
            handle,
            family_index,
            device,
        })
    }

    pub fn allocate_command_pool(self: &Arc<Self>) -> Arc<CommandPool> {
        let command_pool_create_info = vulkan_ffi::VkCommandPoolCreateInfo {
            sType: vulkan_ffi::VkStructureType::COMMAND_POOL_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: vulkan_ffi::VkCommandPoolCreateFlags::TRANSIENT_BIT
                | vulkan_ffi::VkCommandPoolCreateFlags::RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: self.family_index,
        };

        let mut command_pool_handle: vulkan_ffi::VkCommandPool = std::ptr::null_mut();
        vk_call!(vulkan_ffi::vkCreateCommandPool(
            self.device.handle(),
            &command_pool_create_info,
            std::ptr::null(),
            &mut command_pool_handle,
        ))
        .map_err(|err| error!("vkCreateCommandPool {:?}", err))
        .unwrap();

        Arc::new(CommandPool {
            handle: command_pool_handle,
            device: Arc::clone(&self.device),
        })
    }

    pub fn wait_idle(self: &Arc<Self>) {
        vk_call!(vulkan_ffi::vkQueueWaitIdle(self.handle)).expect("vkQueueWaitIdle");
    }

    pub fn submit(self: &Arc<Self>, commands: &CommandBuffer) {
        let submit_info = vulkan_ffi::VkSubmitInfo {
            sType: vulkan_ffi::VkStructureType::SUBMIT_INFO as u32,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            pWaitDstStageMask: std::ptr::null(),
            commandBufferCount: 1,
            pCommandBuffers: &commands.handle,
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };

        vk_call!(vulkan_ffi::vkQueueSubmit(
            self.handle,
            1,
            &submit_info,
            commands.fence,
        ))
        .expect("vkQueueSubmit failures should be handled");
    }

    pub fn submit_present(
        self: &Arc<Self>,
        swapchain: vulkan_ffi::VkSwapchainKHR,
        image_index: u32,
    ) -> SubmitStatus {
        let present_info = vulkan_ffi::VkPresentInfoKHR {
            sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_PRESENT_INFO_KHR as u32,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            swapchainCount: 1,
            pSwapchains: &swapchain,
            pImageIndices: &image_index,
            pResults: std::ptr::null_mut(),
        };

        let result = vk_call!(vulkan_ffi::vkQueuePresentKHR(self.handle, &present_info));
        match result {
            Ok(()) => SubmitStatus::Success,
            Err(err) => {
                if let crate::gfx::Status::Timeout(_) = err {
                    SubmitStatus::Timeout
                } else {
                    SubmitStatus::Error(err)
                }
            }
        }
    }

    pub fn submit_with_keyed_mutex(
        self: &Arc<Self>,
        command_buffer: &CommandBuffer,
        shared_pixel_buffer: &PixelBuffer,
        acquire_key: u64,
        release_key: u64,
    ) -> SubmitStatus {
        let timeout = 10u32;

        let keyed_mutex_acquire_release_info = vulkan_ffi::VkWin32KeyedMutexAcquireReleaseInfoKHR {
            sType: vulkan_ffi::VkStructureType::WIN32_KEYED_MUTEX_ACQUIRE_RELEASE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            acquireCount: 1,
            pAcquireSyncs: &shared_pixel_buffer.device_memory(),
            pAcquireKeys: &acquire_key,
            pAcquireTimeouts: &timeout,
            releaseCount: 1,
            pReleaseSyncs: &shared_pixel_buffer.device_memory(),
            pReleaseKeys: &release_key,
        };
        let submit_info = vulkan_ffi::VkSubmitInfo {
            sType: vulkan_ffi::VkStructureType::SUBMIT_INFO as u32,
            pNext: &keyed_mutex_acquire_release_info as *const _ as *const _,
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            pWaitDstStageMask: std::ptr::null(),
            commandBufferCount: 1,
            pCommandBuffers: &command_buffer.handle,
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };
        let result = vk_call!(vulkan_ffi::vkQueueSubmit(
            self.handle,
            1,
            &submit_info,
            command_buffer.fence,
        ));
        self.wait_idle();

        match result {
            Ok(()) => SubmitStatus::Success,
            Err(err) => {
                if let crate::gfx::Status::Timeout(_) = err {
                    SubmitStatus::Timeout
                } else {
                    SubmitStatus::Error(err)
                }
            }
        }
    }
}
