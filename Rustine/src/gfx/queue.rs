#![allow(dead_code)]

use crate::{error, gfx, gfx::vulkan_ffi as vk, warning};
use crate::{
    gfx::CommandBuffer, gfx::CommandPool, gfx::presentation::AcquireStatus,
    gfx::presentation::Method, gfx::presentation::PresentationImage,
    gfx::presentation::PresentationProvider, gfx::vulkan,
};

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub enum SubmitStatus {
    Success,
    Timeout,
    OutOfDate,
    Error(gfx::Status),
}

/*/// Represents a transfer operation type.
/// General form is some input data or resources, some output data or resources,
/// and an optional completion callback.
pub enum TransferOp {
    CreateBufferFromData {
        data: Vec<u8>,
        usage: vk::VkBufferUsageFlags,
        output_buffer: Arc<Mutex<MemoryBuffer>>,
        completion_callback: Option<Box<dyn FnOnce() + Send>>,
    },
}

pub struct TransferQueue {
    queue_handle: Arc<Mutex<vk::VkQueue>>,
    command_pool: Arc<CommandPool>,
    available_commands: VecDeque<CommandBuffer>,
    recorded_commands: VecDeque<CommandBuffer>,
    queued_commands: VecDeque<CommandBuffer>,
}

impl Drop for TransferQueue {
    fn drop(&mut self) {
        warning!("TransferQueue::drop");

        //self.wait_for_idle();

        while let Some(cmd) = self.recorded_commands.pop_front() {
            cmd.reset();
            self.available_commands.push_back(cmd);
        }

        while let Some(cmd) = self.queued_commands.pop_front() {
            cmd.reset();
            self.available_commands.push_back(cmd);
        }
    }
}

impl TransferQueue {
    pub fn new(device: &Arc<vulkan::Device>) -> Self {
        let family_index = device.transfer_queue_family_index();
        let queue_handle = device.transfer_queue();

        let command_pool = CommandPool::new(family_index, &device);

        let mut available_commands = VecDeque::new();

        for _ in 0..3 {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        TransferQueue {
            queue_handle,
            command_pool,
            available_commands,
            recorded_commands: VecDeque::new(),
            queued_commands: VecDeque::new(),
        }
    }
}*/

pub struct Queue {
    queue_handle: Arc<Mutex<vk::VkQueue>>,
    command_pool: Arc<CommandPool>,
    available_commands: VecDeque<CommandBuffer>,
    recorded_commands: VecDeque<CommandBuffer>,
    queued_commands: VecDeque<CommandBuffer>,

    presentation_provider: Box<dyn PresentationProvider>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        warning!("Queue::drop");

        self.drain();
    }
}

impl Queue {
    pub fn new(
        device: &Arc<vulkan::Device>,
        presentation_provider: impl PresentationProvider + 'static,
    ) -> Self {
        let family_index = device.general_queue_family_index();
        let queue_handle = device.general_queue();

        let command_pool = CommandPool::new(family_index, &device);

        let mut available_commands = VecDeque::new();

        let command_buffer_count = u32::max(1, presentation_provider.image_count() - 1);
        for _ in 0..command_buffer_count {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        Queue {
            queue_handle,
            command_pool,
            available_commands,
            recorded_commands: VecDeque::new(),
            queued_commands: VecDeque::new(),
            presentation_provider: Box::new(presentation_provider),
        }
    }

    pub fn wait_for_idle(&self) {
        let locked_queue = self.queue_handle.lock().unwrap();
        let result = unsafe { vk::vkQueueWaitIdle(*locked_queue) };
        match result {
            vk::VkResult::SUCCESS => {}
            _ => error!(
                "Queue::wait_for_idle: {:?}",
                gfx::Status::from_code(result.0)
            ),
        }
    }

    pub fn drain(&mut self) {
        self.wait_for_idle();

        while let Some(cmd) = self.recorded_commands.pop_front() {
            cmd.reset();
            self.available_commands.push_back(cmd);
        }

        while let Some(cmd) = self.queued_commands.pop_front() {
            cmd.reset();
            self.available_commands.push_back(cmd);
        }
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

    fn ensure_available_command(&mut self) -> bool {
        if self.available_commands.is_empty() {
            if self.queued_commands.is_empty() {
                panic!(
                    "Queue::ensure_available_command: No available command buffers and no queued commands"
                );
            } else {
                let first_queued = self.queued_commands.front().unwrap();
                let completed = first_queued.wait_for_completion(1_000_000);
                if completed {
                    let completed = self.queued_commands.pop_front().unwrap();
                    completed.reset();
                    self.available_commands.push_back(completed);
                } else {
                    warning!("Queue::ensure_available_command: Timeout");
                    return false;
                }
            }
        }
        true
    }

    pub fn enqueue_present(
        &mut self,
        command_recorder: impl FnOnce(&CommandBuffer, &PresentationImage),
    ) {
        self.collect_completed_commands();
        if !self.ensure_available_command() {
            warning!("Queue::enqueue_present: No available command buffers");
            return;
        }

        let output_frame = match self.presentation_provider.acquire() {
            AcquireStatus::Success(frame) => frame,
            AcquireStatus::Timeout => {
                warning!("Queue::enqueue_present: Acquire Timeout");
                return;
            }
            AcquireStatus::OutOfDate => {
                warning!("Queue::enqueue_present: Acquire OutOfDate");
                return;
            }
            AcquireStatus::Error(err) => {
                error!("Queue::enqueue_present: Acquire {:?}", err);
                return;
            }
        };

        let command_buffer = self.available_commands.pop_front();
        if command_buffer.is_none() {
            error!("Queue::enqueue_present: No available command buffer!");
            return;
        }
        let command_buffer = command_buffer.unwrap();
        command_buffer.begin();
        command_recorder(&command_buffer, &output_frame);
        command_buffer.end();

        let submit_status = match self.presentation_provider.method() {
            Method::Headless => {
                warning!(
                    "Queue::enqueue_present: Headless presentation method does not support presenting"
                );
                return;
            }
            Method::SharedImage => {
                self.submit_with_keyed_mutex(&command_buffer, &output_frame, 1, 0)
            }
            Method::Swapchain => self.submit_present(&command_buffer, None, &output_frame),
        };
        match submit_status {
            SubmitStatus::Success => {
                self.queued_commands.push_back(command_buffer);
            }
            SubmitStatus::Timeout => {
                self.drain();
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
            SubmitStatus::OutOfDate => {
                warning!("Queue::enqueue_present: Submit OutOfDate");
                self.drain();
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
            SubmitStatus::Error(err) => {
                error!("Queue::enqueue_present: Submit {:?}", err);
                self.drain();
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
        }
    }

    fn submit(
        &self,
        command_buffer: &CommandBuffer,
        previous_command_buffer: Option<&CommandBuffer>,
    ) -> SubmitStatus {
        let wait_dst_stage = vk::VkPipelineStageFlags::TOP_OF_PIPE_BIT;
        let (wait_semaphore_count, wait_semaphores, wait_dst_stage_mask) =
            if let Some(prev) = previous_command_buffer {
                (
                    1,
                    &prev.semaphore() as *const vk::VkSemaphore,
                    &wait_dst_stage as *const vk::VkPipelineStageFlags,
                )
            } else {
                (0, std::ptr::null(), std::ptr::null())
            };

        let submit_info = vk::VkSubmitInfo {
            sType: vk::VkStructureType::SUBMIT_INFO as u32,
            pNext: std::ptr::null(),
            waitSemaphoreCount: wait_semaphore_count,
            pWaitSemaphores: wait_semaphores,
            pWaitDstStageMask: wait_dst_stage_mask,
            commandBufferCount: 1,
            pCommandBuffers: &command_buffer.handle(),
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };

        let result = unsafe {
            let locked_queue = self.queue_handle.lock().unwrap();
            vk::vkQueueSubmit(*locked_queue, 1, &submit_info, command_buffer.fence())
        };
        match result {
            vk::VkResult::SUCCESS => SubmitStatus::Success,
            _ => SubmitStatus::Error(crate::gfx::Status::from_code(result.0)),
        }
    }

    fn submit_present(
        &self,
        command_buffer: &CommandBuffer,
        previous_command_buffer: Option<&CommandBuffer>,
        image: &PresentationImage,
    ) -> SubmitStatus {
        match self.submit(command_buffer, previous_command_buffer) {
            SubmitStatus::Success => {}
            other => return other,
        };

        let present_info = vk::VkPresentInfoKHR {
            sType: vk::VkStructureType::PRESENT_INFO_KHR as u32,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 1,
            pWaitSemaphores: &image.acquire_semaphore,
            swapchainCount: 1,
            pSwapchains: &image.swapchain_handle,
            pImageIndices: &image.index,
            pResults: std::ptr::null_mut(),
        };

        let result = unsafe {
            let locked_queue = self.queue_handle.lock().unwrap();
            vk::vkQueuePresentKHR(*locked_queue, &present_info)
        };
        match result {
            vk::VkResult::SUCCESS => SubmitStatus::Success,
            vk::VkResult::SUBOPTIMAL_KHR => {
                warning!("Queue::submit_present: Present suboptimal");
                SubmitStatus::Success
            }
            vk::VkResult::OUT_OF_DATE_KHR => SubmitStatus::OutOfDate,
            vk::VkResult::TIMEOUT => SubmitStatus::Timeout,
            _ => SubmitStatus::Error(crate::gfx::Status::from_code(result.0)),
        }
    }

    fn submit_with_keyed_mutex(
        &self,
        command_buffer: &CommandBuffer,
        shared_pixel_buffer: &PresentationImage,
        acquire_key: u64,
        release_key: u64,
    ) -> SubmitStatus {
        let timeout = 10u32;

        let keyed_mutex_acquire_release_info = vk::VkWin32KeyedMutexAcquireReleaseInfoKHR {
            sType: vk::VkStructureType::WIN32_KEYED_MUTEX_ACQUIRE_RELEASE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            acquireCount: 1,
            pAcquireSyncs: &shared_pixel_buffer.memory,
            pAcquireKeys: &acquire_key,
            pAcquireTimeouts: &timeout,
            releaseCount: 1,
            pReleaseSyncs: &shared_pixel_buffer.memory,
            pReleaseKeys: &release_key,
        };
        let submit_info = vk::VkSubmitInfo {
            sType: vk::VkStructureType::SUBMIT_INFO as u32,
            pNext: &keyed_mutex_acquire_release_info as *const _ as *const _,
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            pWaitDstStageMask: std::ptr::null(),
            commandBufferCount: 1,
            pCommandBuffers: &command_buffer.handle(),
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };
        let result = unsafe {
            let locked_queue = self.queue_handle.lock().unwrap();
            vk::vkQueueSubmit(*locked_queue, 1, &submit_info, command_buffer.fence())
        };
        match result {
            vk::VkResult::SUCCESS => SubmitStatus::Success,
            vk::VkResult::TIMEOUT => SubmitStatus::Timeout,
            _ => SubmitStatus::Error(gfx::Status::from_code(result.0)),
        }
    }
}
