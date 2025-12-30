#![allow(dead_code)]

use crate::{error, warning};
use crate::{
    gfx::CommandBuffer, gfx::CommandPool, gfx::PixelBuffer, gfx::Queue, gfx::SubmitStatus,
};

use std::{collections::VecDeque, sync::Arc};

pub enum AcquireStatus<'a> {
    Success(&'a PixelBuffer),
    Timeout,
    OutOfDate,
    Error(crate::gfx::Status),
}

pub trait PresentationProvider {
    fn acquire(&self) -> AcquireStatus<'_>;
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
    fn acquire(&self) -> AcquireStatus<'_> {
        AcquireStatus::Success(&self.output_frame)
    }
}

pub struct SwapchainProvider {}

impl PresentationProvider for SwapchainProvider {
    fn acquire(&self) -> AcquireStatus<'_> {
        AcquireStatus::Timeout
    }
}

pub struct Presenter {
    queue: Arc<Queue>,
    command_pool: Arc<CommandPool>,
    available_commands: VecDeque<CommandBuffer>,
    recorded_commands: VecDeque<CommandBuffer>,
    queued_commands: VecDeque<CommandBuffer>,

    presentation_provider: Box<dyn PresentationProvider>,
}

impl Drop for Presenter {
    fn drop(&mut self) {
        warning!("Presenter::drop");

        self.queue.wait_idle();
    }
}

impl Presenter {
    pub fn new(
        queue: Arc<Queue>,
        presentation_provider: impl PresentationProvider + 'static,
    ) -> Self {
        let command_pool = queue.allocate_command_pool();

        let mut available_commands = VecDeque::new();

        for _ in 0..3 {
            available_commands.push_back(command_pool.allocate_command_buffer().unwrap());
        }

        Presenter {
            queue,
            command_pool,
            available_commands,
            recorded_commands: VecDeque::new(),
            queued_commands: VecDeque::new(),
            presentation_provider: Box::new(presentation_provider),
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

    fn ensure_available_command(&mut self) {
        if self.available_commands.is_empty() {
            if self.queued_commands.is_empty() {
                error!(
                    "Presenter::ensure_available_command: No available command buffers and no queued commands"
                );
            } else {
                warning!(
                    "Presenter::ensure_available_command: No available command buffers, waiting for the first queued command to complete"
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

        let command_buffer = self
            .available_commands
            .pop_front()
            .expect("No available command buffers");

        let output_frame = match self.presentation_provider.acquire() {
            AcquireStatus::Success(frame) => frame,
            AcquireStatus::Timeout => {
                warning!("Presenter::present: Acquire timed out");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::OutOfDate => {
                warning!("Presenter::present: Acquire out of date");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::Error(err) => {
                error!("Presenter::present: Acquire error: {:?}", err);
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
                //warning!("Presenter::present_frame: Submit timed out");
                commands.reset();
                self.available_commands.push_back(commands);
            }
            SubmitStatus::Error(err) => {
                error!("Presenter::present_frame: Submit error: {:?}", err);
                commands.reset();
                self.available_commands.push_back(commands);
            }
        }*/
    }

    pub fn enqueue_present_keyed_mutex(
        &mut self,
        command_recorder: impl FnOnce(&CommandBuffer, &PixelBuffer),
    ) {
        self.collect_completed_commands();
        self.ensure_available_command();

        let command_buffer = self
            .available_commands
            .pop_front()
            .expect("No available command buffers");

        let output_frame = match self.presentation_provider.acquire() {
            AcquireStatus::Success(frame) => frame,
            AcquireStatus::Timeout => {
                warning!("Presenter::present: Acquire timed out");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::OutOfDate => {
                warning!("Presenter::present: Acquire out of date");
                self.available_commands.push_back(command_buffer);
                return;
            }
            AcquireStatus::Error(err) => {
                error!("Presenter::present: Acquire error: {:?}", err);
                self.available_commands.push_back(command_buffer);
                return;
            }
        };

        command_buffer.begin();
        command_recorder(&command_buffer, &output_frame);
        command_buffer.end();

        let submit_status =
            self.queue
                .submit_with_keyed_mutex(&command_buffer, &output_frame, 1, 0);
        match submit_status {
            SubmitStatus::Success => {
                self.queued_commands.push_back(command_buffer);
            }
            SubmitStatus::Timeout => {
                //warning!("Presenter::present_frame: Submit timed out");
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
            SubmitStatus::Error(err) => {
                error!("Presenter::present_frame: Submit error: {:?}", err);
                command_buffer.reset();
                self.available_commands.push_back(command_buffer);
            }
        }
    }
}
