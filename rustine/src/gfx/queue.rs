#![allow(dead_code)]

use crate::{error, gfx::vulkan as vk, gfx::*, warning};
use crate::{
    gfx::CommandBuffer, gfx::CommandPool, gfx::presentation::AcquireStatus,
    gfx::presentation::Method, gfx::presentation::PresentationImage,
    gfx::presentation::PresentationProvider,
};

use std::{collections::VecDeque, rc::Rc};

pub enum SubmitStatus {
    Success,
    Timeout,
    OutOfDate,
    Error(crate::gfx::Status),
}

pub struct Queue {
    command_pool: Rc<CommandPool>,
    available_commands: VecDeque<CommandBuffer>,
    queued_commands: VecDeque<CommandBuffer>,

    presentation_provider: Option<presentation::SwapchainProvider>,

    device: Rc<Device>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        warning!("Queue::drop");

        self.drain();
    }
}

impl Queue {
    pub fn drop_presenter(&mut self) {
        self.drain();
        self.presentation_provider = None;
    }

    pub fn swap_presenter(&mut self, parameters: presentation::Parameters) {
        // The idea here is to retrieve VkSwapchainKHR from old_presenter,
        // while still keeping old_presenter alive until the new presenter is created
        let old_presenter = self.presentation_provider.take();
        let old_swapchain = if let Some(old_presenter) = &old_presenter {
            old_presenter.handle()
        } else {
            vk::VkSwapchainKHR::default()
        };

        let presentation_provider =
            presentation::SwapchainProvider::new(&self.device, parameters, old_swapchain);
        self.presentation_provider = Some(presentation_provider);

        self.drain();
    }

    pub fn new(device: &Rc<Device>, concurrent_commands: u32) -> Self {
        let command_pool = CommandPool::new(device.general_queue_family_index(), device);

        let mut available_commands = VecDeque::new();

        for _ in 0..concurrent_commands {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        Queue {
            command_pool,
            available_commands,
            queued_commands: VecDeque::new(),
            presentation_provider: None,
            device: Rc::clone(device),
        }
    }

    pub fn wait_for_idle(&self) {
        let result = unsafe { vk::vkQueueWaitIdle(*self.device.general_queue()) };
        match result {
            vk::VkResult::SUCCESS => {}
            _ => error!("Queue::wait_for_idle: {:?}", Status::from_code(result.0)),
        }
    }

    pub fn drain(&mut self) {
        //warning!("Queue::drain");
        self.wait_for_idle();

        while let Some(mut cmd) = self.queued_commands.pop_front() {
            cmd.reset();
            self.available_commands.push_back(cmd);
        }
    }

    fn collect_completed_commands(&mut self) {
        while let Some(front) = self.queued_commands.front() {
            if front.is_complete() {
                let mut completed = self.queued_commands.pop_front().unwrap();
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
                    let mut completed = self.queued_commands.pop_front().unwrap();
                    completed.reset();
                    self.available_commands.push_back(completed);
                } else {
                    //warning!("Queue::ensure_available_command: Timeout");
                    return false;
                }
            }
        }
        true
    }

    pub fn enqueue(&mut self, command_recorder: impl FnOnce(&mut CommandBuffer)) {
        self.collect_completed_commands();
        if !self.ensure_available_command() {
            //warning!("Queue::enqueue: No available command buffers");
            return;
        }

        let command_buffer = self.available_commands.pop_front();
        if command_buffer.is_none() {
            //error!("Queue::enqueue: No available command buffer!");
            return;
        }

        let mut command_buffer = command_buffer.unwrap();
        command_buffer.use_for(command::CommandPurpose::Graphics);
        command_buffer.begin();
        command_recorder(&mut command_buffer);
        command_buffer.end();

        // Find the most recent queued command that is not a Present.
        /*let previous_command_buffer = self
            .queued_commands
            .iter()
            .rev()
            .find(|cmd| cmd.purpose() != command::CommandPurpose::Present);

        if previous_command_buffer.is_some() {
            warning!("Queue::enqueue: Found previous command buffer that is not a Present");
        }*/

        let submit_status = self.submit(&command_buffer, None);
        match submit_status {
            SubmitStatus::Success => {
                self.queued_commands.push_back(command_buffer);
            }
            _ => {
                warning!("Queue::enqueue: Submit failed");
                self.drain();
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
        }
    }

    pub fn enqueue_present(
        &mut self,
        command_recorder: impl FnOnce(&mut CommandBuffer, &PresentationImage),
    ) {
        self.collect_completed_commands();
        if !self.ensure_available_command() {
            //warning!("Queue::enqueue_present: No available command buffers");
            return;
        }

        if self.presentation_provider.is_none() {
            warning!("Queue::enqueue_present: No presentation provider");
            return;
        }

        let presentation_provider = &mut self.presentation_provider.as_mut().unwrap();

        let output_frame = match presentation_provider.acquire() {
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
        let mut command_buffer = command_buffer.unwrap();
        command_buffer.use_for(command::CommandPurpose::Present);
        command_buffer.begin();
        command_recorder(&mut command_buffer, &output_frame);
        command_buffer.end();

        let submit_status = match presentation_provider.method() {
            Method::Headless => {
                warning!(
                    "Queue::enqueue_present: Headless presentation method does not support presenting"
                );
                return;
            }
            Method::SharedImage => self.submit(&command_buffer, None),
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
            sType: vk::VkStructureType::SUBMIT_INFO,
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
            vk::vkQueueSubmit(
                *self.device.general_queue(),
                1,
                &submit_info,
                command_buffer.fence(),
            )
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
            sType: vk::VkStructureType::PRESENT_INFO_KHR,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 1,
            pWaitSemaphores: &image.acquire_semaphore,
            swapchainCount: 1,
            pSwapchains: &image.swapchain_handle,
            pImageIndices: &image.index,
            pResults: std::ptr::null_mut(),
        };

        let result = unsafe { vk::vkQueuePresentKHR(*self.device.general_queue(), &present_info) };
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
}
