#![allow(dead_code)]

mod glfw_ffi;

use crate::gfx::{self, vulkan_ffi};
use crate::{debug, warning};

use std::sync::{Arc, Mutex};

pub struct Core {
    gfx: Arc<Mutex<gfx::Core>>,
    gfx_surface: vulkan_ffi::VkSurfaceKHR,
    glfw_window: glfw_ffi::GLFWwindow,
}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");

        unsafe {
            if !self.gfx_surface.is_null() {
                let gfx = self.gfx.lock().unwrap();
                vulkan_ffi::vkDestroySurfaceKHR(
                    gfx.vulkan_instance_handle(),
                    self.gfx_surface,
                    std::ptr::null(),
                );
            }
            if !self.glfw_window.is_null() {
                glfw_ffi::glfwDestroyWindow(self.glfw_window);
            }
            glfw_ffi::glfwTerminate();
        }
    }
}

impl Core {
    pub fn new(gfx: Arc<Mutex<gfx::Core>>) -> Self {
        unsafe {
            glfw_ffi::glfwInit();
            glfw_ffi::glfwWindowHint(glfw_ffi::CLIENT_API as i32, 0);
            //glfw_ffi::glfwWindowHint(glfw_ffi::DECORATED as i32, 0);
            glfw_ffi::glfwWindowHint(glfw_ffi::AUTO_ICONIFY as i32, 0);
        }

        let window_title = std::ffi::CString::new("Rustine").unwrap();
        let glfw_window = unsafe {
            glfw_ffi::glfwCreateWindow(
                800,
                600,
                window_title.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        unsafe {
            glfw_ffi::glfwSetFramebufferSizeCallback(
                glfw_window,
                Core::glfw_framebuffer_size_callback,
            );
        }

        let gfx_surface = {
            let mut surface_handle: vulkan_ffi::VkSurfaceKHR = std::ptr::null_mut();
            let result = unsafe {
                glfw_ffi::glfwCreateWindowSurface(
                    gfx.lock().unwrap().vulkan_instance_handle() as u64,
                    glfw_window,
                    std::ptr::null(),
                    &mut surface_handle,
                )
            };
            if result != 0 {
                panic!("Failed to create window surface: {}", result);
            }

            surface_handle
        };

        Core {
            gfx,
            gfx_surface,
            glfw_window,
        }
    }

    unsafe extern "C" fn glfw_framebuffer_size_callback(
        _window: glfw_ffi::GLFWwindow,
        _width: i32,
        _height: i32,
    ) {
        // Handle framebuffer size changes if needed
        debug!("Framebuffer size changed: {}x{}", _width, _height);
    }

    pub fn should_close(&self) -> bool {
        unsafe { glfw_ffi::glfwWindowShouldClose(self.glfw_window) != 0 }
    }

    pub fn process_events(&self) {
        unsafe {
            glfw_ffi::glfwWaitEventsTimeout(0.01);
        }
    }
}
