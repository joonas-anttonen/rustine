#![allow(dead_code)]

mod glfw_ffi;
use glfw_ffi as glfw;

mod rwl_ffi;
use rwl_ffi as rwl;

mod input;
use input::{Action, Key, KeyEvent, Mods};

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
            if parameters.platform != gfx::Platform::Wayland {
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
            glfw::glfwSetKeyCallback(glfw_window, Core::glfw_key_callback);
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
            glfw::panic_if_error();
            if result != vk::VkResult::SUCCESS {
                panic!(
                    "Failed to create Vulkan surface: {:?}",
                    gfx::Status::from_code(result.0)
                );
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
            let core_raw_ptr = Arc::as_ptr(&core);

            glfw::glfwSetWindowUserPointer(core.glfw_window, core_raw_ptr as *mut _);
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

    unsafe extern "C" fn glfw_key_callback(
        window: glfw::GLFWwindow,
        key: i32,
        _scancode: i32,
        action: i32,
        mods: i32,
    ) {
        unsafe {
            let gui_ptr = glfw::glfwGetWindowUserPointer(window) as *mut Core;
            if !gui_ptr.is_null() {
                //let gui = &mut *gui_ptr;
                let key_event = KeyEvent {
                    key: Key::from_code(key),
                    action: Action::from_code(action),
                    mods: Mods::from_code(mods),
                };
                debug!(
                    "Key event: key={:?}, action={:?}, mods={:?}",
                    key_event.key.0, key_event.action.0, key_event.mods.0
                );
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

pub struct Gui {
    gfx: Arc<Mutex<gfx::Core>>,
    gfx_surface: vk::VkSurfaceKHR,
    rwl_window: rwl::RwlWindow,
}

impl Drop for Gui {
    fn drop(&mut self) {
        warning!("Gui::drop");

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

            rwl::panic_if_error(rwl::rwlDestroyWindow(self.rwl_window));
            rwl::panic_if_error(rwl::rwlShutdown());
        }
    }
}

impl Gui {
    pub fn new(gfx: Arc<Mutex<gfx::Core>>, parameters: StartupParameters) -> Arc<Self> {
        let rwl_window = unsafe {
            if parameters.platform != gfx::Platform::Wayland {
                panic!("Unsupported platform");
            }

            rwl::rwlSetLogCallback(Self::rwl_log_callback);
            rwl::panic_if_error(rwl::rwlStartup());

            // Outputs
            let mut output_count: u32 = 0;
            rwl::panic_if_error(rwl::rwlEnumerateOutputs(
                &mut output_count,
                std::ptr::null_mut(),
            ));
            let mut outputs: Vec<rwl::RwlOutputInfo> = Vec::with_capacity(output_count as usize);
            rwl::panic_if_error(rwl::rwlEnumerateOutputs(
                &mut output_count,
                outputs.as_mut_ptr(),
            ));
            outputs.set_len(output_count as usize);

            // DEBUG: Print output information
            for output in &outputs {
                let name = if output.name.is_null() {
                    "<null>"
                } else {
                    std::ffi::CStr::from_ptr(output.name)
                        .to_str()
                        .unwrap_or("<invalid utf8>")
                };
                let description = if output.description.is_null() {
                    "<null>"
                } else {
                    std::ffi::CStr::from_ptr(output.description)
                        .to_str()
                        .unwrap_or("<invalid utf8>")
                };
                debug!(
                    "Output: name={}, description={}, scale={}, width={}, height={}",
                    name, description, output.scale, output.width, output.height
                );
            }

            // DEBUG: Select eDP-1 or nothing
            let output = {
                outputs
                    .iter()
                    .find(|o| std::ffi::CStr::from_ptr(o.name).to_string_lossy() == "asdasd")
                    .map(|o| o.wl_output)
                    .unwrap_or(std::ptr::null_mut())
            };

            let mut rwl_window = std::ptr::null_mut();
            rwl::panic_if_error(rwl::rwlCreateWindow(
                rwl::RwlWindowType::Normal,
                output,
                1280,
                720,
                &mut rwl_window,
            ));

            rwl::panic_if_error(rwl::rwlSetPixelSizeCallback(
                rwl_window,
                Self::rwl_pixel_size_callback,
            ));
            rwl::panic_if_error(rwl::rwlSetLogicalSizeCallback(
                rwl_window,
                Self::rwl_logical_size_callback,
            ));

            rwl_window
        };

        let gfx_surface = unsafe {
            let mut surface_handle: vk::VkSurfaceKHR = std::ptr::null_mut();
            rwl::panic_if_error(rwl::rwlCreateSurface(
                gfx.lock().unwrap().vulkan_instance_handle(),
                rwl_window,
                &mut surface_handle,
            ));

            surface_handle
        };

        let gui = Arc::new(Self {
            gfx,
            gfx_surface: gfx_surface,
            rwl_window,
        });

        unsafe {
            let gui_raw_ptr = Arc::as_ptr(&gui);
            rwl::rwlSetWindowUserPointer(gui.rwl_window, gui_raw_ptr as *mut _);
        }

        // Manually invoke the framebuffer size callback to initialize the swapchain
        unsafe {
            let mut width: u32 = 0;
            let mut height: u32 = 0;
            rwl::panic_if_error(rwl::rwlGetPixelSize(
                gui.rwl_window,
                &mut width,
                &mut height,
            ));
            Self::rwl_pixel_size_callback(gui.rwl_window, width, height);
        }

        gui
    }

    pub fn should_close(&self) -> bool {
        unsafe { rwl::rwlWindowShouldClose(self.rwl_window) }
    }

    pub fn process_events(&self) {
        unsafe {
            rwl::panic_if_error(rwl::rwlPollEvents());
        }
    }

    unsafe extern "C" fn rwl_pixel_size_callback(window: rwl::RwlWindow, width: u32, height: u32) {
        unsafe {
            let gui_ptr = rwl::rwlGetWindowUserPointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                debug!("Pixel size changed: {}x{}", width, height);

                let gui = &mut *gui_ptr;
                let mut gfx = gui.gfx.lock().unwrap();

                gfx.drop_queue();

                // When minimized, width and height can be zero
                // but we can't create a swapchain with zero dimensions
                if width == 0 || height == 0 {
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

    unsafe extern "C" fn rwl_logical_size_callback(
        _window: rwl::RwlWindow,
        width: u32,
        height: u32,
    ) {
        // Implement logical size callback handling here if needed, just log for now
        debug!("Logical size changed: {}x{}", width, height);
    }

    unsafe extern "C" fn rwl_log_callback(severity: u32, message: *const std::ffi::c_char) {
        use crate::log::Severity;

        let message_str = unsafe {
            if message.is_null() {
                return;
            }
            std::ffi::CStr::from_ptr(message).to_string_lossy()
        };

        let sev = match severity {
            0 => Severity::Debug,
            1 => Severity::Info,
            2 => Severity::Warning,
            3 => Severity::Error,
            _ => Severity::Info,
        };

        crate::log::Log::global().append(sev, &message_str, "rustine_wl");
    }
}
