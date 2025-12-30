#![allow(dead_code)]

use crate::{error, gfx, gfx::vulkan, warning};
use std::sync::Arc;

pub struct Presenter {
    queue: Arc<vulkan::Queue>,
    command_pool: Arc<gfx::CommandPool>,
    available_commands: std::collections::VecDeque<gfx::CommandBuffer>,
    recorded_commands: std::collections::VecDeque<gfx::CommandBuffer>,
    queued_commands: std::collections::VecDeque<gfx::CommandBuffer>,

    output_frame: gfx::PixelBuffer,
}

impl Drop for Presenter {
    fn drop(&mut self) {
        warning!("Presenter::drop");

        self.queue.wait_idle();
    }
}

impl Presenter {
    pub fn new(queue: Arc<vulkan::Queue>, output_frame: gfx::PixelBuffer) -> Self {
        let command_pool = queue.allocate_command_pool();

        let mut available_commands = std::collections::VecDeque::new();

        for _ in 0..3 {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        Presenter {
            queue,
            command_pool,
            available_commands,
            recorded_commands: std::collections::VecDeque::new(),
            queued_commands: std::collections::VecDeque::new(),
            output_frame,
        }
    }

    // Takes a closure that records commands into a command buffer for the frame
    pub fn record_frame(
        &mut self,
        recorder: impl FnOnce(&gfx::CommandBuffer, &gfx::PixelBuffer),
    ) -> () {
        //warning!("Presenter::record_frame");

        // Check the front of the queued commands to see if any have completed
        while let Some(front) = self.queued_commands.front() {
            if front.is_complete() {
                let completed = self.queued_commands.pop_front().unwrap();
                completed.reset();
                self.available_commands.push_back(completed);
            } else {
                break;
            }
        }

        if self.available_commands.is_empty() {
            if self.queued_commands.is_empty() {
                error!(
                    "Presenter::record_frame: No available command buffers and no queued commands"
                );
            } else {
                warning!(
                    "Presenter::record_frame: No available command buffers, waiting for the first queued command to complete"
                );
                let first_queued = self.queued_commands.front().unwrap();
                first_queued.wait_for_completion(10_000_000).unwrap();
                let completed = self.queued_commands.pop_front().unwrap();
                completed.reset();
                self.available_commands.push_back(completed);
            }
        }

        // Implementation for recording a frame for presentation
        let command_buffer = self
            .available_commands
            .pop_front()
            .expect("No available command buffers");

        recorder(&command_buffer, &self.output_frame);

        self.recorded_commands.push_back(command_buffer);

        ()
    }

    pub fn present_frame(&mut self) {
        //warning!("Presenter::present_frame");

        let commands = self
            .recorded_commands
            .pop_front()
            .expect("No recorded command buffers to present");

        //self.queue.submit(&commands);
        //self.queued_commands.push_back(commands);

        let submit_status =
            self.queue
                .submit_with_keyed_mutex(&commands, &self.output_frame, 1, 0);
        match submit_status {
            vulkan::SubmitStatus::Success => {
                self.queued_commands.push_back(commands);
            }
            vulkan::SubmitStatus::Timeout => {
                //warning!("Presenter::present_frame: Submit timed out");
                commands.reset();
                self.available_commands.push_back(commands);
            }
            vulkan::SubmitStatus::Error(err) => {
                error!("Presenter::present_frame: Submit error: {:?}", err);
                commands.reset();
                self.available_commands.push_back(commands);
            }
        }
    }
}
