#![allow(dead_code)]

use crate::{error, vk_call, warning};
use crate::{
    gfx::CommandBuffer, gfx::CommandPool, gfx::PixelBuffer, gfx::presentation::AcquireStatus,
    gfx::presentation::PresentationMethod, gfx::presentation::PresentationProvider, gfx::vulkan,
    gfx::vulkan_ffi,
};

use std::{collections::VecDeque, sync::Arc};

pub enum SubmitStatus {
    Success,
    Timeout,
    Error(crate::gfx::Status),
}

pub struct Queue {
    queue_handle: vulkan_ffi::VkQueue,
    command_pool: Arc<CommandPool>,
    available_commands: VecDeque<CommandBuffer>,
    recorded_commands: VecDeque<CommandBuffer>,
    queued_commands: VecDeque<CommandBuffer>,

    presentation_method: PresentationMethod,
    presentation_provider: Box<dyn PresentationProvider>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        warning!("Queue::drop");

        self.wait_for_idle();
    }
}

impl Queue {
    pub fn new(
        device: &Arc<vulkan::Device>,
        presentation_method: PresentationMethod,
        presentation_provider: impl PresentationProvider + 'static,
    ) -> Self {
        let family_index = device.general_queue_family_index();
        let queue_handle = device.create_general_queue();

        let command_pool = CommandPool::new(family_index, &device);

        let mut available_commands = VecDeque::new();

        for _ in 0..3 {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        Queue {
            queue_handle,
            command_pool,
            available_commands,
            recorded_commands: VecDeque::new(),
            queued_commands: VecDeque::new(),
            presentation_method,
            presentation_provider: Box::new(presentation_provider),
        }
    }

    pub fn wait_for_idle(&self) {
        vk_call!(vulkan_ffi::vkQueueWaitIdle(self.queue_handle)).expect("vkQueueWaitIdle");
    }

    fn collect_completed_commands(&mut self) {
        while let Some(front) = self.queued_commands.front() {
            if front.is_complete() {
                let completed = self.queued_commands.pop_front().unwrap();
                completed.reset();
                self.available_commands.push_back(completed);
            } else {
                break;
            }
        }
    }

    fn ensure_available_command(&mut self) {
        if self.available_commands.is_empty() {
            if self.queued_commands.is_empty() {
                error!(
                    "Queue::ensure_available_command: No available command buffers and no queued commands"
                );
            } else {
                warning!(
                    "Queue::ensure_available_command: No available command buffers, waiting for the first queued command to complete"
                );
                let first_queued = self.queued_commands.front().unwrap();
                first_queued.wait_for_completion(10_000_000).unwrap();
                let completed = self.queued_commands.pop_front().unwrap();
                completed.reset();
                self.available_commands.push_back(completed);
            }
        }
    }

    pub fn enqueue_present(&mut self, command_recorder: impl FnOnce(&CommandBuffer, &PixelBuffer)) {
        self.collect_completed_commands();
        self.ensure_available_command();

        match self.presentation_method {
            PresentationMethod::Headless => {
                warning!(
                    "Queue::enqueue_present: Headless presentation method does not support presenting"
                );
            }
            PresentationMethod::SharedImage => self.enqueue_present_keyed_mutex(command_recorder),
            PresentationMethod::Swapchain => {
                let command_buffer = self
                    .available_commands
                    .pop_front()
                    .expect("No available command buffers");

                let output_frame = match self.presentation_provider.acquire() {
                    AcquireStatus::Success(frame) => frame,
                    AcquireStatus::Timeout => {
                        warning!("Queue::present: Acquire timed out");
                        self.available_commands.push_back(command_buffer);
                        return;
                    }
                    AcquireStatus::OutOfDate => {
                        warning!("Queue::present: Acquire out of date");
                        self.available_commands.push_back(command_buffer);
                        return;
                    }
                    AcquireStatus::Error(err) => {
                        error!("Queue::present: Acquire error: {:?}", err);
                        self.available_commands.push_back(command_buffer);
                        return;
                    }
                };

                command_buffer.begin();
                command_recorder(&command_buffer, &output_frame);
                command_buffer.end();

                self.recorded_commands.push_back(command_buffer);

                /*let commands = self
                    .recorded_commands
                    .pop_front()
                    .expect("No recorded command buffers to present");

                let submit_status = self
                    .queue
                    .submit_present(&commands, _, _);
                match submit_status {
                    SubmitStatus::Success => {
                        self.queued_commands.push_back(commands);
                    }
                    SubmitStatus::Timeout => {
                        //warning!("Queue::present_frame: Submit timed out");
                        commands.reset();
                        self.available_commands.push_back(commands);
                    }
                    SubmitStatus::Error(err) => {
                        error!("Queue::present_frame: Submit error: {:?}", err);
                        commands.reset();
                        self.available_commands.push_back(commands);
                    }
                }*/
            }
        }
    }

    fn enqueue_present_keyed_mutex(
        &mut self,
        command_recorder: impl FnOnce(&CommandBuffer, &PixelBuffer),
    ) {
        let command_buffer = self
            .available_commands
            .pop_front()
            .expect("No available command buffers");

        let output_frame = match self.presentation_provider.acquire() {
            AcquireStatus::Success(frame) => frame,
            AcquireStatus::Timeout => {
                warning!("Queue::present: Acquire timed out");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::OutOfDate => {
                warning!("Queue::present: Acquire out of date");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::Error(err) => {
                error!("Queue::present: Acquire error: {:?}", err);
                self.available_commands.push_back(command_buffer);
                return;
            }
        };

        command_buffer.begin();
        command_recorder(&command_buffer, &output_frame);
        command_buffer.end();

        let submit_status = self.submit_with_keyed_mutex(&command_buffer, &output_frame, 1, 0);
        match submit_status {
            SubmitStatus::Success => {
                self.queued_commands.push_back(command_buffer);
            }
            SubmitStatus::Timeout => {
                //warning!("Queue::present_frame: Submit timed out");
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
            SubmitStatus::Error(err) => {
                error!("Queue::present_frame: Submit error: {:?}", err);
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
        }
    }

    pub fn submit(&self, commands: &CommandBuffer) {
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
            self.queue_handle,
            1,
            &submit_info,
            commands.fence,
        ))
        .expect("vkQueueSubmit failures should be handled");
    }

    pub fn submit_present(
        &self,
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

        let result = vk_call!(vulkan_ffi::vkQueuePresentKHR(
            self.queue_handle,
            &present_info
        ));
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
        &self,
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
            self.queue_handle,
            1,
            &submit_info,
            command_buffer.fence,
        ));
        self.wait_for_idle();

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
