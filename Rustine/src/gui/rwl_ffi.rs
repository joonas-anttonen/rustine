#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

use std::ffi;

/// Opaque Wayland window handle
pub type RwlWindow = *mut ffi::c_void;

/// Window type for different Wayland layer shell surfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwlWindowType {
    /// Desktop background (bottom layer, covers full screen)
    Background = 0,
    /// Taskbar/panel (top layer, typically anchored to top)
    Taskbar = 1,
}

/// Callback for pixel size changes (e.g., when compositor configures the surface)
pub type RwlPixelSizeCallback = unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

/// Callback for logical size changes (e.g., when compositor configures the surface)
pub type RwlLogicalSizeCallback = unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

/// Callback for frame timing (compositor is ready for next frame)
pub type RwlFrameCallback = unsafe extern "C" fn(window: RwlWindow);

/// Callback for logging messages from the library
/// severity: 0=Debug, 1=Info, 2=Warning, 3=Error
pub type RwlLogCallback = unsafe extern "C" fn(severity: u32, message: *const ffi::c_char);

/// Status codes returned by rwl functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwlStatus {
    Ok = 0,
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NoDisplay = 3,
    NoRegistry = 4,
    NoCompositor = 5,
    NoLayerShell = 6,
    InvalidArgument = 7,
    InternalError = 8,
}

impl RwlStatus {
    /// Check if the status represents an error
    pub fn is_error(self) -> bool {
        self as u32 != 0
    }
}

pub fn panic_if_error(status: RwlStatus) {
    if status.is_error() {
        panic!("Rustine-WL error: {:?}", status);
    }
}

#[link(name = "rustine-wl", kind = "static")]
unsafe extern "C" {
    // Initialization and shutdown
    pub fn rwlStartup() -> RwlStatus;
    pub fn rwlSetLogCallback(callback: RwlLogCallback);
    pub fn rwlShutdown() -> RwlStatus;

    // Window management
    pub fn rwlCreateWindow(
        window_type: RwlWindowType,
        output: *const ffi::c_void,
        width: u32,
        height: u32,
        window_out: *mut RwlWindow,
    ) -> RwlStatus;

    pub fn rwlDestroyWindow(window: RwlWindow) -> RwlStatus;

    pub fn rwlSetWindowUserPointer(window: RwlWindow, pointer: *mut ffi::c_void) -> RwlStatus;
    pub fn rwlGetWindowUserPointer(window: RwlWindow) -> *mut ffi::c_void;

    pub fn rwlSetPixelSizeCallback(
        window: RwlWindow,
        callback: RwlPixelSizeCallback,
    ) -> RwlStatus;
    pub fn rwlSetLogicalSizeCallback(
        window: RwlWindow,
        callback: RwlLogicalSizeCallback,
    ) -> RwlStatus;

    pub fn rwlSetFrameCallback(window: RwlWindow, callback: RwlFrameCallback) -> RwlStatus;

    pub fn rwlGetPixelSize(window: RwlWindow, width: *mut u32, height: *mut u32) -> RwlStatus;
    pub fn rwlGetLogicalSize(window: RwlWindow, width: *mut u32, height: *mut u32) -> RwlStatus;

    // Vulkan surface creation
    pub fn rwlCreateSurface(
        instance: crate::gfx::vulkan_ffi::VkInstance,
        window: RwlWindow,
        surface_out: *mut crate::gfx::vulkan_ffi::VkSurfaceKHR,
    ) -> RwlStatus;

    // Event loop control
    pub fn rwlProcessEvents() -> RwlStatus;
    pub fn rwlWindowShouldClose(window: RwlWindow) -> bool;
    pub fn rwlWindowRequestClose(window: RwlWindow) -> RwlStatus;
}