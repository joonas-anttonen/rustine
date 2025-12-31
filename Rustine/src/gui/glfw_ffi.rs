#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

pub fn panic_if_error() {
    unsafe {
        let mut description_ptr: *const i8 = std::ptr::null();
        let error_code = glfwGetError(&mut description_ptr);
        if error_code != 0 {
            let description_cstr = std::ffi::CStr::from_ptr(description_ptr);
            let description_str = description_cstr.to_string_lossy();
            panic!("GLFW {} {}", error_code, description_str);
        }
    }
}

pub type GLFWwindow = *mut std::ffi::c_void;
pub type GLFWmonitor = *mut std::ffi::c_void;
pub type GLFWframebuffersizefun = unsafe extern "C" fn(window: GLFWwindow, width: i32, height: i32);

#[link(name = "glfw3", kind = "static")]
unsafe extern "C" {
    pub fn glfwInitHint(hint: i32, value: i32);
    pub fn glfwInit() -> i32;
    pub fn glfwTerminate();
    pub fn glfwGetError(description: *mut *const i8) -> i32;

    pub fn glfwCreateWindow(
        width: i32,
        height: i32,
        title: *const i8,
        monitor: *mut std::ffi::c_void,
        share: *mut std::ffi::c_void,
    ) -> GLFWwindow;
    pub fn glfwDestroyWindow(window: GLFWwindow);
    pub fn glfwWindowShouldClose(window: GLFWwindow) -> i32;
    pub fn glfwWindowHint(hint: i32, value: i32);
    pub fn glfwSetWindowPos(window: GLFWwindow, xpos: i32, ypos: i32);
    pub fn glfwSetWindowSizeLimits(
        window: GLFWwindow,
        minwidth: i32,
        minheight: i32,
        maxwidth: i32,
        maxheight: i32,
    );
    pub fn glfwSetWindowUserPointer(window: GLFWwindow, pointer: *mut std::ffi::c_void);
    pub fn glfwGetWindowUserPointer(window: GLFWwindow) -> *mut std::ffi::c_void;
    pub fn glfwGetFramebufferSize(window: GLFWwindow, width: *mut i32, height: *mut i32);
    pub fn glfwPollEvents();
    pub fn glfwWaitEvents();
    pub fn glfwWaitEventsTimeout(timeout: f64);

    pub fn glfwSetFramebufferSizeCallback(
        window: GLFWwindow,
        cbfun: GLFWframebuffersizefun,
    ) -> GLFWframebuffersizefun;

    pub fn glfwCreateWindowSurface(
        instance: crate::gfx::vulkan_ffi::VkInstance,
        window: GLFWwindow,
        allocator: *const std::ffi::c_void,
        surface: *mut crate::gfx::vulkan_ffi::VkSurfaceKHR,
    ) -> i32;

    pub fn glfwGetPrimaryMonitor() -> GLFWmonitor;
    pub fn glfwGetMonitors(count: *mut i32) -> *mut GLFWmonitor;
    pub fn glfwGetMonitorWorkarea(
        monitor: GLFWmonitor,
        xpos: *mut i32,
        ypos: *mut i32,
        width: *mut i32,
        height: *mut i32,
    );
}

pub const PLATFORM: i32 = 0x00050003;
pub const ANY_PLATFORM: i32 = 0x00060000;
pub const PLATFORM_WIN32: i32 = 0x00060001;
pub const PLATFORM_COCOA: i32 = 0x00060002;
pub const PLATFORM_WAYLAND: i32 = 0x00060003;
pub const PLATFORM_X11: i32 = 0x00060004;
pub const PLATFORM_NULL: i32 = 0x00060005;

pub const FALSE: i32 = 0;
pub const TRUE: i32 = 1;
pub const CLIENT_API: i32 = 0x00022001;
pub const DECORATED: i32 = 0x00020005;
pub const AUTO_ICONIFY: i32 = 0x00020006;

/// GLFW error codes
#[derive(Debug)]
pub struct GLFWerror(i32);

impl GLFWerror {
    /// No error has occurred.
    pub const NO_ERROR: i32 = 0;
    /// GLFW has not been initialized.
    ///
    /// This occurs if a GLFW function was called that must not be called unless the
    /// library is initialized.
    pub const NOT_INITIALIZED: i32 = 0x00010001;
    /// No context is current for this thread.
    ///
    /// This occurs if a GLFW function was called that needs and operates on the
    /// current OpenGL or OpenGL ES context but no context is current on the calling
    /// thread. One such function is glfwSwapInterval.
    pub const NO_CURRENT_CONTEXT: i32 = 0x00010002;
    /// One of the arguments to the function was an invalid enum value.
    pub const INVALID_ENUM: i32 = 0x00010003;
    /// One of the arguments to the function was an invalid value.
    ///
    /// For example, requesting a non-existent OpenGL or OpenGL ES version like 2.7.
    /// Requesting a valid but unavailable OpenGL or OpenGL ES version will instead
    /// result in a VERSION_UNAVAILABLE error.
    pub const INVALID_VALUE: i32 = 0x00010004;
    /// A memory allocation failed.
    pub const OUT_OF_MEMORY: i32 = 0x00010005;
    /// GLFW could not find support for the requested API on the system.
    ///
    /// The installed graphics driver does not support the requested API, or does not
    /// support it via the chosen context creation API.
    pub const API_UNAVAILABLE: i32 = 0x00010006;
    /// The requested OpenGL or OpenGL ES version is not available.
    ///
    /// The requested OpenGL or OpenGL ES version (including any requested context
    /// or framebuffer hints) is not available on this machine.
    pub const VERSION_UNAVAILABLE: i32 = 0x00010007;
    /// A platform-specific error occurred that does not match any of the more specific categories.
    pub const PLATFORM_ERROR: i32 = 0x00010008;
    /// The requested format is not supported or available.
    ///
    /// If emitted during window creation, the requested pixel format is not supported.
    /// If emitted when querying the clipboard, the contents of the clipboard could
    /// not be converted to the requested format.
    pub const FORMAT_UNAVAILABLE: i32 = 0x00010009;
    /// The specified window does not have an OpenGL or OpenGL ES context.
    pub const NO_WINDOW_CONTEXT: i32 = 0x0001000A;
    /// The specified cursor shape is not available.
    ///
    /// The specified standard cursor shape is not available, either because the
    /// current platform cursor theme does not provide it or because it is not
    /// available on the platform.
    pub const CURSOR_UNAVAILABLE: i32 = 0x0001000B;
    /// The requested feature is not provided by the platform.
    ///
    /// The requested feature is not provided by the platform, so GLFW is unable to
    /// implement it. The error can be ignored unless the feature is critical to the application.
    pub const FEATURE_UNAVAILABLE: i32 = 0x0001000C;
    /// The requested feature is not implemented for the platform.
    ///
    /// The requested feature has not yet been implemented in GLFW for this platform.
    pub const FEATURE_UNIMPLEMENTED: i32 = 0x0001000D;
    /// Platform unavailable or no matching platform was found.
    ///
    /// If emitted during initialization, no matching platform was found. Failure to detect
    /// any platform usually only happens on non-macOS Unix systems, either when no window
    /// system is running or the program was run from a terminal that does not have the
    /// necessary environment variables.
    pub const PLATFORM_UNAVAILABLE: i32 = 0x0001000E;
}
