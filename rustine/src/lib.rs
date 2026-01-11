pub mod gfx;
pub mod gui;
pub mod io;
pub mod log;
pub use log::Log;
pub mod alloc;
mod ringbuffer;
pub use ringbuffer::RingBuffer;
pub mod version;
pub use version::Version;

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