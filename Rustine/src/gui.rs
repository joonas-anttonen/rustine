#![allow(dead_code)]

mod glfw_ffi;
use glfw_ffi as glfw;

use crate::gfx::{self, presentation, vulkan_ffi as vk};
use crate::{debug, warning};

use std::sync::{Arc, Mutex};

pub struct StartupParameters {
    pub platform: gfx::Platform,
    pub window_title: String,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

pub struct Core {
    gfx: Arc<Mutex<gfx::Core>>,
    gfx_surface: vk::VkSurfaceKHR,
    glfw_window: glfw::GLFWwindow,
}

impl Drop for Core {
    fn drop(&mut self) {
        warning!("Core::drop");

        let mut gfx = self.gfx.lock().unwrap();
        gfx.drop_queue();

        unsafe {
            if !self.gfx_surface.is_null() {
                vk::vkDestroySurfaceKHR(
                    gfx.vulkan_instance_handle(),
                    self.gfx_surface,
                    std::ptr::null(),
                );
            }
            if !self.glfw_window.is_null() {
                glfw::glfwDestroyWindow(self.glfw_window);
            }
            glfw::glfwTerminate();
        }
    }
}

impl Core {
    pub fn new(gfx: Arc<Mutex<gfx::Core>>, parameters: StartupParameters) -> Arc<Core> {
        let window_title = std::ffi::CString::new(parameters.window_title).unwrap();

        let glfw_window = unsafe {
            match parameters.platform {
                gfx::Platform::Windows => {
                    glfw::glfwInitHint(glfw::PLATFORM, glfw::PLATFORM_WIN32);
                }
                gfx::Platform::Wayland => {
                    glfw::glfwInitHint(glfw::PLATFORM, glfw::PLATFORM_WAYLAND);
                }
                gfx::Platform::X11 => {
                    glfw::glfwInitHint(glfw::PLATFORM, glfw::PLATFORM_X11);
                }
                _ => {
                    panic!("Unsupported platform");
                }
            }

            glfw::glfwInit();
            glfw::panic_if_error();

            let window_width = parameters.window_width.unwrap_or(1280);
            let window_height = parameters.window_height.unwrap_or(720);

            glfw::glfwWindowHint(glfw::CLIENT_API, glfw::FALSE);
            let glfw_window = glfw::glfwCreateWindow(
                window_width as i32,
                window_height as i32,
                window_title.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            glfw::panic_if_error();
            glfw::glfwSetWindowSizeLimits(glfw_window, 256, 144, -1, -1);

            // Center the window on the primary monitor
            // This won't work on Wayland
            {
                let primary_monitor = glfw::glfwGetPrimaryMonitor();
                let mut monitor_x: i32 = 0;
                let mut monitor_y: i32 = 0;
                let mut monitor_width: i32 = 0;
                let mut monitor_height: i32 = 0;
                glfw::glfwGetMonitorWorkarea(
                    primary_monitor,
                    &mut monitor_x,
                    &mut monitor_y,
                    &mut monitor_width,
                    &mut monitor_height,
                );
                let window_x = monitor_x + (monitor_width - window_width as i32) / 2;
                let window_y = monitor_y + (monitor_height - window_height as i32) / 2;
                glfw::glfwSetWindowPos(glfw_window, window_x, window_y);
            }

            glfw::glfwSetFramebufferSizeCallback(glfw_window, Core::glfw_framebuffer_size_callback);
            glfw_window
        };

        let gfx_surface = {
            let mut surface_handle: vk::VkSurfaceKHR = std::ptr::null_mut();
            let result = unsafe {
                glfw::glfwCreateWindowSurface(
                    gfx.lock().unwrap().vulkan_instance_handle(),
                    glfw_window,
                    std::ptr::null(),
                    &mut surface_handle,
                )
            };
            if result != 0 {
                panic!("glfwCreateWindowSurface");
            }

            surface_handle
        };

        let core = Arc::new(Core {
            gfx,
            gfx_surface,
            glfw_window,
        });

        // ASSUMPTION: Arc will place Core on the heap and it won't move.
        unsafe {
            let core_arc_cloned = Arc::clone(&core);
            let core_raw_ptr = Arc::into_raw(core_arc_cloned);

            glfw::glfwSetWindowUserPointer(core.glfw_window, core_raw_ptr as *mut std::ffi::c_void);
        }

        unsafe {
            let mut width: i32 = 0;
            let mut height: i32 = 0;
            glfw::glfwGetFramebufferSize(core.glfw_window, &mut width, &mut height);
            Core::glfw_framebuffer_size_callback(core.glfw_window, width, height);
        }
        core
    }

    unsafe extern "C" fn glfw_framebuffer_size_callback(
        window: glfw::GLFWwindow,
        width: i32,
        height: i32,
    ) {
        unsafe {
            let gui_ptr = glfw::glfwGetWindowUserPointer(window) as *mut Core;
            if !gui_ptr.is_null() {
                debug!("Framebuffer size changed: {}x{}", width, height);

                let gui = &mut *gui_ptr;
                let mut gfx = gui.gfx.lock().unwrap();

                gfx.drop_queue();

                // When minimized, width and height can be zero
                // but we can't create a swapchain with zero dimensions
                if width <= 0 || height <= 0 {
                    return;
                }

                let presentation_parameters = presentation::Parameters {
                    width: width as u32,
                    height: height as u32,
                    surface_handle: gui.gfx_surface as *const _,
                    vertical_sync: 0,
                };
                let presentation_provider =
                    presentation::SwapchainProvider::new(gfx.device(), presentation_parameters);
                gfx.initialize_swapchain_queue(presentation_provider);
            }
        }
    }

    pub fn should_close(&self) -> bool {
        unsafe { glfw::glfwWindowShouldClose(self.glfw_window) != 0 }
    }

    pub fn process_events(&self) {
        unsafe {
            glfw::glfwWaitEventsTimeout(0.01);
        }
    }
}
