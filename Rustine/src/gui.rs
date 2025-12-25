#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use crate::warning;

pub struct Core {
    gfx: Arc<Mutex<crate::gfx::core::Core>>,
}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");
    }
}

impl Core {
    pub fn new(gfx: Arc<Mutex<crate::gfx::core::Core>>) -> Self {
        Core { gfx }
    }
}
