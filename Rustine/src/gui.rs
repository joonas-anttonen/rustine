#![allow(dead_code)]

use crate::{gfx, warning};
use std::sync::{Arc, Mutex};

pub struct Core {
    gfx: Arc<Mutex<gfx::Core>>,
}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");
    }
}

impl Core {
    pub fn new(gfx: Arc<Mutex<gfx::Core>>) -> Self {
        Core { gfx }
    }
}
