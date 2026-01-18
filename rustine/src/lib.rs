pub mod gfx;
pub mod gui;
pub mod io;
pub mod log;
pub use log::Log;
pub mod alloc;
mod ringbuffer;
pub use ringbuffer::RingBuffer;
pub mod version;
pub use rustinesc::*;
pub use version::Version;

use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

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

    /// Pops a oldest item from the mailbox.
    pub fn pop_front(&self) -> Option<T> {
        if let Ok(mut pending) = self.queue.lock() {
            pending.pop_front()
        } else {
            None
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

pub type Vector2i = Vector2<i32>;
pub type Vector2u = Vector2<u32>;
pub type Vector2f = Vector2<f32>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Default for Vector2<u32> {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl Default for Vector2<f32> {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl std::ops::Add for Vector2<f32> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vector2<f32> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vector2<f32> {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl std::ops::Mul<Vector2<f32>> for f32 {
    type Output = Vector2<f32>;
    fn mul(self, vec: Vector2<f32>) -> Vector2<f32> {
        Vector2 {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl std::ops::Neg for Vector2<f32> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Vector2<f32> {
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self { x: 0.0, y: 0.0 }
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Computes the Euclidean distance between this vector and another.
    pub fn distance(&self, other: &Self) -> f32 {
        (*other - *self).length()
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
