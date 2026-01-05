#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

use std::ffi;

/// Opaque Wayland window handle
pub type RwlWindow = *mut ffi::c_void;

/// Opaque output handle
pub type RwlOutput = *mut ffi::c_void;

/// Window type for different Wayland layer shell surfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwlWindowType {
    Normal = 0,
    /// Desktop background (bottom layer, covers full screen)
    Background = 1,
    /// Taskbar/panel (top layer, typically anchored to top)
    Taskbar = 2,
}

/// Key action states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RwlAction(u32);

impl RwlAction {
    pub const RELEASE: u32 = 0;
    pub const PRESS: u32 = 1;
}

/// Modifier key flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RwlMod(u32);
impl std::ops::BitOr for RwlMod {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        RwlMod(self.0 | rhs.0)
    }
}
impl RwlMod {
    pub const NONE: u32 = 0;
    pub const SHIFT: u32 = 1 << 0;
    pub const CTRL: u32 = 1 << 1;
    pub const ALT: u32 = 1 << 2;
    pub const SUPER: u32 = 1 << 3;
}

/// Keyboard key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RwlKey(i32);

impl RwlKey {
    pub const UNKNOWN: i32 = -1;
    pub const SPACE: i32 = 32;
    pub const APOSTROPHE: i32 = 39;
    pub const COMMA: i32 = 44;
    pub const MINUS: i32 = 45;
    pub const PERIOD: i32 = 46;
    pub const SLASH: i32 = 47;
    pub const N0: i32 = 48;
    pub const N1: i32 = 49;
    pub const N2: i32 = 50;
    pub const N3: i32 = 51;
    pub const N4: i32 = 52;
    pub const N5: i32 = 53;
    pub const N6: i32 = 54;
    pub const N7: i32 = 55;
    pub const N8: i32 = 56;
    pub const N9: i32 = 57;
    pub const SEMICOLON: i32 = 59;
    pub const EQUAL: i32 = 61;
    pub const A: i32 = 65;
    pub const B: i32 = 66;
    pub const C: i32 = 67;
    pub const D: i32 = 68;
    pub const E: i32 = 69;
    pub const F: i32 = 70;
    pub const G: i32 = 71;
    pub const H: i32 = 72;
    pub const I: i32 = 73;
    pub const J: i32 = 74;
    pub const K: i32 = 75;
    pub const L: i32 = 76;
    pub const M: i32 = 77;
    pub const N: i32 = 78;
    pub const O: i32 = 79;
    pub const P: i32 = 80;
    pub const Q: i32 = 81;
    pub const R: i32 = 82;
    pub const S: i32 = 83;
    pub const T: i32 = 84;
    pub const U: i32 = 85;
    pub const V: i32 = 86;
    pub const W: i32 = 87;
    pub const X: i32 = 88;
    pub const Y: i32 = 89;
    pub const Z: i32 = 90;
    pub const LEFT_BRACKET: i32 = 91;
    pub const BACKSLASH: i32 = 92;
    pub const RIGHT_BRACKET: i32 = 93;
    pub const GRAVE_ACCENT: i32 = 96;
    pub const WORLD1: i32 = 161;
    pub const WORLD2: i32 = 162;
    pub const ESCAPE: i32 = 256;
    pub const ENTER: i32 = 257;
    pub const TAB: i32 = 258;
    pub const BACKSPACE: i32 = 259;
    pub const INSERT: i32 = 260;
    pub const DELETE: i32 = 261;
    pub const RIGHT: i32 = 262;
    pub const LEFT: i32 = 263;
    pub const DOWN: i32 = 264;
    pub const UP: i32 = 265;
    pub const PAGE_UP: i32 = 266;
    pub const PAGE_DOWN: i32 = 267;
    pub const HOME: i32 = 268;
    pub const END: i32 = 269;
    pub const CAPS_LOCK: i32 = 280;
    pub const SCROLL_LOCK: i32 = 281;
    pub const NUM_LOCK: i32 = 282;
    pub const PRINT_SCREEN: i32 = 283;
    pub const PAUSE: i32 = 284;
    pub const F1: i32 = 290;
    pub const F2: i32 = 291;
    pub const F3: i32 = 292;
    pub const F4: i32 = 293;
    pub const F5: i32 = 294;
    pub const F6: i32 = 295;
    pub const F7: i32 = 296;
    pub const F8: i32 = 297;
    pub const F9: i32 = 298;
    pub const F10: i32 = 299;
    pub const F11: i32 = 300;
    pub const F12: i32 = 301;
    pub const F13: i32 = 302;
    pub const F14: i32 = 303;
    pub const F15: i32 = 304;
    pub const F16: i32 = 305;
    pub const F17: i32 = 306;
    pub const F18: i32 = 307;
    pub const F19: i32 = 308;
    pub const F20: i32 = 309;
    pub const F21: i32 = 310;
    pub const F22: i32 = 311;
    pub const F23: i32 = 312;
    pub const F24: i32 = 313;
    pub const F25: i32 = 314;
    pub const KP0: i32 = 320;
    pub const KP1: i32 = 321;
    pub const KP2: i32 = 322;
    pub const KP3: i32 = 323;
    pub const KP4: i32 = 324;
    pub const KP5: i32 = 325;
    pub const KP6: i32 = 326;
    pub const KP7: i32 = 327;
    pub const KP8: i32 = 328;
    pub const KP9: i32 = 329;
    pub const KP_DECIMAL: i32 = 330;
    pub const KP_DIVIDE: i32 = 331;
    pub const KP_MULTIPLY: i32 = 332;
    pub const KP_SUBTRACT: i32 = 333;
    pub const KP_ADD: i32 = 334;
    pub const KP_ENTER: i32 = 335;
    pub const KP_EQUAL: i32 = 336;
    pub const LEFT_SHIFT: i32 = 340;
    pub const LEFT_CONTROL: i32 = 341;
    pub const LEFT_ALT: i32 = 342;
    pub const LEFT_SUPER: i32 = 343;
    pub const RIGHT_SHIFT: i32 = 344;
    pub const RIGHT_CONTROL: i32 = 345;
    pub const RIGHT_ALT: i32 = 346;
    pub const RIGHT_SUPER: i32 = 347;
    pub const MENU: i32 = 348;
    pub const COUNT: i32 = 349;
}

/// Output information structure
#[derive(Debug, Clone)]
#[repr(C)]
pub struct RwlOutputInfo {
    /// Opaque pointer to the wl_output
    pub wl_output: *mut ffi::c_void,
    /// Output name (e.g., "HDMI-1", "DP-2")
    pub name: *const ffi::c_char,
    /// Output description
    pub description: *const ffi::c_char,
    /// Scale factor
    pub scale: i32,
    /// Physical width in pixels
    pub width: i32,
    /// Physical height in pixels
    pub height: i32,
}

/// Callback for pixel size changes (e.g., when compositor configures the surface)
pub type RwlPixelSizeCallback = unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

/// Callback for logical size changes (e.g., when compositor configures the surface)
pub type RwlLogicalSizeCallback = unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

/// Callback for key events
pub type RwlKeyCallback = unsafe extern "C" fn(
    window: RwlWindow,
    key: RwlKey,
    scancode: i32,
    action: RwlAction,
    mods: RwlMod,
);

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

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwlLogSeverity {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
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
    pub fn rwlSetLogCallback(min_severity: RwlLogSeverity, callback: RwlLogCallback);
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

    pub fn rwlSetPixelSizeCallback(window: RwlWindow, callback: RwlPixelSizeCallback) -> RwlStatus;
    pub fn rwlSetLogicalSizeCallback(
        window: RwlWindow,
        callback: RwlLogicalSizeCallback,
    ) -> RwlStatus;

    pub fn rwlSetKeyCallback(window: RwlWindow, callback: RwlKeyCallback) -> RwlStatus;

    pub fn rwlGetPixelSize(window: RwlWindow, width: *mut u32, height: *mut u32) -> RwlStatus;
    pub fn rwlGetLogicalSize(window: RwlWindow, width: *mut u32, height: *mut u32) -> RwlStatus;

    // Vulkan surface creation
    pub fn rwlCreateSurface(
        instance: crate::gfx::vulkan_ffi::VkInstance,
        window: RwlWindow,
        surface_out: *mut crate::gfx::vulkan_ffi::VkSurfaceKHR,
    ) -> RwlStatus;

    // Output management - Vulkan style enumeration
    // Call with outputs_out=NULL to get count, then call again with allocated buffer
    pub fn rwlEnumerateOutputs(count: *mut u32, outputs_out: *mut RwlOutputInfo) -> RwlStatus;

    // Event loop control
    pub fn rwlPollEvents() -> RwlStatus;
    pub fn rwlWaitEvents() -> RwlStatus;
    pub fn rwlWaitEventsTimeout(timeout_ns: u64) -> RwlStatus;
    pub fn rwlPostEmptyEvent() -> RwlStatus;
    pub fn rwlWindowShouldClose(window: RwlWindow) -> bool;
    pub fn rwlWindowRequestClose(window: RwlWindow) -> RwlStatus;
}
