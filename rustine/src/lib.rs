pub mod gfx;
pub mod gui;
pub mod scene;
pub mod io;
pub mod log;
pub use log::Log;
pub mod alloc;
mod ringbuffer;
pub use ringbuffer::RingBuffer;
pub mod version;
pub use rustinesc::*;
pub use version::Version;
mod numerics;
pub use numerics::*;
pub mod model;
pub mod lua;
pub mod lua_ui;

use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

/// Controls how the GUI and GFX run loops handle timing and synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Continuous loop with frame rate limiting.
    Continuous,
    /// Wait on a condition variable until work is signaled.
    /// Useful for on-demand rendering or reducing CPU usage.
    Event,
}

/// A thread-safe mailbox for sending and receiving data with optional capacity limit.
pub struct Mailbox<T> {
    queue: Mutex<VecDeque<T>>,
    event: Arc<AutoResetEvent>,
    capacity: usize,
}

impl<T> Mailbox<T> {
    /// Creates a new mailbox without a capacity limit.
    ///
    /// Use `with_capacity` to specify a capacity limit.
    pub fn new(event: Arc<AutoResetEvent>) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(std::collections::VecDeque::new()),
            event,
            capacity: 0,
        })
    }

    /// Creates a new mailbox with a specified capacity.
    ///
    /// If the mailbox reaches its capacity, the oldest item will be removed to make room for new ones.
    pub fn with_capacity(event: Arc<AutoResetEvent>, capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(std::collections::VecDeque::with_capacity(capacity)),
            event,
            capacity,
        })
    }

    /// Waits until the mailbox has at least one item.
    pub fn wait(&self) {
        self.event.wait();
    }

    /// Pushes an item into the mailbox queue and signals the waiter.
    pub fn push(&self, item: T) {
        if let Ok(mut pending) = self.queue.lock() {
            // If the mailbox has a capacity and is full, remove the oldest item.
            if self.capacity > 0 && pending.len() >= self.capacity {
                log::warning!("Mailbox::discard");
                pending.pop_front();
            }

            pending.push_back(item);
            self.event.set();
        }
    }

    /// Pops the latest item from the mailbox and discards any older items.
    pub fn pop_back_and_discard(&self) -> Option<T> {
        if let Ok(mut pending) = self.queue.lock() {
            let request = pending.pop_back();
            pending.clear();
            request
        } else {
            None
        }
    }

    /// Pops the latest item from the mailbox.
    pub fn pop_back(&self) -> Option<T> {
        if let Ok(mut pending) = self.queue.lock() {
            pending.pop_back()
        } else {
            None
        }
    }

    /// Pops a oldest item from the mailbox.
    pub fn pop_front(&self) -> Option<T> {
        if let Ok(mut pending) = self.queue.lock() {
            pending.pop_front()
        } else {
            None
        }
    }

    pub fn retain(&self, f: impl FnMut(&T) -> bool) {
        if let Ok(mut pending) = self.queue.lock() {
            pending.retain(f);
        }
    }
}

pub struct AutoResetEvent {
    mutex: Mutex<bool>,
    condvar: Condvar,
}

impl AutoResetEvent {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    pub fn wait(&self) {
        let mut signaled = self.mutex.lock().unwrap();
        while !*signaled {
            signaled = self.condvar.wait(signaled).unwrap();
        }
        *signaled = false;
    }

    pub fn set(&self) {
        let mut signaled = self.mutex.lock().unwrap();
        *signaled = true;
        self.condvar.notify_one();
    }
}

impl Default for AutoResetEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the target platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Wayland,
    X11,
    MacOS,
}

/// Parameters for initializing the library.
#[derive(Debug)]
pub(crate) struct Parameters {
    pub debugging: bool,
    pub platform: Platform,
    pub app_version: Version,
    pub app_name: String,
    pub device_selector: gfx::DeviceSelector,
}

pub mod utilities {
    pub fn format_duration(seconds: f64) -> String {
        if seconds >= 3600.0 {
            format!("{:.0} h", seconds / 3600.0)
        } else if seconds >= 60.0 {
            format!("{:.0} m", seconds / 60.0)
        } else if seconds >= 1.0 {
            format!("{:.0} s", seconds)
        } else if seconds >= 1e-3 {
            format!("{:.0} ms", seconds * 1e3)
        } else if seconds >= 1e-6 {
            format!("{:.0} us", seconds * 1e6)
        } else {
            format!("{:.0} ns", seconds * 1e9)
        }
    }

    pub fn format_bytes_iec(bytes: usize) -> String {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut value = bytes as f64;
        let mut idx = 0usize;
        while value >= 1024.0 && idx < UNITS.len() - 1 {
            value /= 1024.0;
            idx += 1;
        }
        if idx == 0 {
            format!("{:.0} {}", value, UNITS[idx])
        } else if value < 10.0 {
            format!("{:.1} {}", value, UNITS[idx])
        } else {
            format!("{:.0} {}", value, UNITS[idx])
        }
    }
}
