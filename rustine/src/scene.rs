#![allow(dead_code)]

use crate::{
    AutoResetEvent, Mailbox, RunMode,
    gfx::{self},
    io, log,
};

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum Command {
    Add(io::Model),
    Nop,
}

pub struct Scene {
    gfx: Arc<Mutex<gfx::Gfx>>,
    command_queue: Arc<Mailbox<Command>>,
}

pub fn run(am_scene: Arc<Mutex<Scene>>, mode: RunMode) {
    Scene::run(am_scene, mode);
}

impl Scene {
    pub fn run(_am_scene: Arc<Mutex<Scene>>, _mode: RunMode) {
        log::set_current_thread_name("scene");
        log::debug!("Scene::run");

        let (cmd_queue, _mesh_queue) = {
            let scene = _am_scene.lock().unwrap();
            let gfx = scene.gfx.lock().unwrap();
            (Arc::clone(&scene.command_queue), gfx.buffer_mailbox())
        };

        loop {
            if crate::should_exit() {
                break;
            }

            cmd_queue.wait();

            if crate::should_exit() {
                break;
            }

            while let Some(cmd) = cmd_queue.pop_front() {
                match cmd {
                    Command::Add(model) => {
                        let scene = _am_scene.lock().unwrap();
                        let mut gfx = scene.gfx.lock().unwrap();
                        gfx.create_buffer(model.calculate_memory_size());

                        for _mesh in &model.meshes {}
                    }
                    _ => {
                        log::warning!("Ignoring command: {:?}", cmd);
                    }
                }
            }
        }

        log::debug!("Scene::run: EXIT");
    }

    pub fn new(gfx: Arc<Mutex<gfx::Gfx>>) -> Self {
        Self {
            gfx,
            command_queue: Mailbox::new(Arc::new(AutoResetEvent::new())),
        }
    }

    pub fn wake_up(&self) {
        self.command_queue.push(Command::Nop);
    }

    pub fn mailbox(&self) -> Arc<Mailbox<Command>> {
        Arc::clone(&self.command_queue)
    }
}
