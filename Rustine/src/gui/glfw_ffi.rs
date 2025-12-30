#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

pub type GLFWwindow = *mut std::ffi::c_void;
pub type GLFWframebuffersizefun = unsafe extern "C" fn(window: GLFWwindow, width: i32, height: i32);

#[link(name = "glfw3", kind = "static")]
unsafe extern "C" {
    pub fn glfwInit() -> i32;
    pub fn glfwTerminate();

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
    pub fn glfwSetWindowUserPointer(window: GLFWwindow, pointer: *mut std::ffi::c_void);
    pub fn glfwGetWindowUserPointer(window: GLFWwindow) -> *mut std::ffi::c_void;

    pub fn glfwPollEvents();
    pub fn glfwWaitEvents();
    pub fn glfwWaitEventsTimeout(timeout: f64);

    pub fn glfwSetFramebufferSizeCallback(
        window: GLFWwindow,
        cbfun: GLFWframebuffersizefun,
    ) -> GLFWframebuffersizefun;

    pub fn glfwCreateWindowSurface(
        instance: u64,
        window: GLFWwindow,
        allocator: *const std::ffi::c_void,
        surface: *mut crate::gfx::vulkan_ffi::VkSurfaceKHR,
    ) -> i32;
}

pub const CLIENT_API: i32 = 0x00022001;
pub const DECORATED: i32 = 0x00020005;
pub const AUTO_ICONIFY: i32 = 0x00020006;
