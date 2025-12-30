#![allow(dead_code)]

mod glfw_ffi;

use crate::gfx::{self, presentation, vulkan_ffi};
use crate::{debug, lib_ffi, warning};

use std::sync::{Arc, Mutex};

pub struct Core {
    gfx: Arc<Mutex<gfx::Core>>,
    gfx_surface: vulkan_ffi::VkSurfaceKHR,
    glfw_window: glfw_ffi::GLFWwindow,
}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");

        let mut gfx = self.gfx.lock().unwrap();
        gfx.drop_queue();

        unsafe {
            if !self.gfx_surface.is_null() {
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

        let core = Core {
            gfx,
            gfx_surface,
            glfw_window,
        };

        // TODO: Think carefully about safety and ownership here
        unsafe {
            glfw_ffi::glfwSetWindowUserPointer(
                core.glfw_window,
                &core as *const Core as *mut std::ffi::c_void,
            );
        }

        unsafe {
            Self::glfw_framebuffer_size_callback(core.glfw_window, 800, 600);
        }
        core
    }

    unsafe extern "C" fn glfw_framebuffer_size_callback(
        window: glfw_ffi::GLFWwindow,
        width: i32,
        height: i32,
    ) {
        unsafe {
            let gui_ptr = glfw_ffi::glfwGetWindowUserPointer(window) as *mut Core;
            if !gui_ptr.is_null() {
                debug!("Framebuffer size changed: {}x{}", width, height);

                let gui = &mut *gui_ptr;
                let mut gfx = gui.gfx.lock().unwrap();

                gfx.drop_queue();

                let presentation_parameters = lib_ffi::PresentationParameters {
                    width: width as u32,
                    height: height as u32,
                    surface_handle: gui.gfx_surface as *const std::ffi::c_void,
                    vertical_sync: 0,
                };
                let presentation_provider =
                    presentation::SwapchainProvider::new(gfx.device(), presentation_parameters);
                gfx.initialize_queue(
                    gfx::presentation::PresentationMethod::Swapchain,
                    presentation_provider,
                );
            }
        }
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
