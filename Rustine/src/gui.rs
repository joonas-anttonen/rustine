#![allow(dead_code)]

use std::sync::{Arc, Mutex};

pub struct Core {
    gfx: Arc<Mutex<crate::gfx::core::Core>>,
}

impl Drop for Core {
    fn drop(&mut self) {
        use super::log;
        log::Log::global().append(log::Severity::Warning, "", "gui::Core", "drop");
    }
}

impl Core {
    pub fn new(gfx: Arc<Mutex<crate::gfx::core::Core>>) -> Self {
        Core { gfx }
    }
}
