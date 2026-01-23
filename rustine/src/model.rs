#![allow(dead_code)]

use std::sync::Weak;

use crate::Mailbox;

pub const INVALID_MODEL_ID: u32 = 999_999_999;

pub struct ModelPool {
    element_byte_size: usize,
    total_byte_size: usize,
    id: u32,
    release_mailbox: Weak<Mailbox<u32>>,
}

impl ModelPool {
    pub fn new(
        element_byte_size: usize,
        total_byte_size: usize,
        id: u32,
        release_mailbox: Weak<Mailbox<u32>>,
    ) -> Self {
        Self {
            element_byte_size,
            total_byte_size,
            id,
            release_mailbox,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Drop for ModelPool {
    fn drop(&mut self) {
        if let Some(mailbox) = self.release_mailbox.upgrade() {
            mailbox.push(self.id);
        }
    }
}

impl Default for ModelPool {
    fn default() -> Self {
        Self {
            element_byte_size: 0,
            total_byte_size: 0,
            id: INVALID_MODEL_ID,
            release_mailbox: Weak::new(),
        }
    }
}

pub struct Model {
    pool_id: u32,
    id: u32,
    release_mailbox: Weak<Mailbox<u32>>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            pool_id: INVALID_MODEL_ID,
            id: INVALID_MODEL_ID,
            release_mailbox: Weak::new(),
        }
    }
}

impl Model {
    pub fn new(pool_id: u32, id: u32, release_mailbox: Weak<Mailbox<u32>>) -> Self {
        Self {
            pool_id,
            id,
            release_mailbox,
        }
    }
}