#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

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
pub type GLFWkeyfun =
    unsafe extern "C" fn(window: GLFWwindow, key: i32, scancode: i32, action: i32, mods: i32);

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
    pub fn glfwSetKeyCallback(window: GLFWwindow, callback: GLFWkeyfun) -> GLFWkeyfun;
    pub fn glfwCreateWindowSurface(
        instance: crate::gfx::vulkan_ffi::VkInstance,
        window: GLFWwindow,
        allocator: *const std::ffi::c_void,
        surface: *mut crate::gfx::vulkan_ffi::VkSurfaceKHR,
    ) -> crate::gfx::vulkan_ffi::VkResult;

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

pub const RELEASE: i32 = 0;
pub const PRESS: i32 = 1;
pub const REPEAT: i32 = 2;

pub const MOUSE_BUTTON_1: i32 = 0;
pub const MOUSE_BUTTON_2: i32 = 1;
pub const MOUSE_BUTTON_3: i32 = 2;
pub const MOUSE_BUTTON_4: i32 = 3;
pub const MOUSE_BUTTON_5: i32 = 4;
pub const MOUSE_BUTTON_6: i32 = 5;
pub const MOUSE_BUTTON_7: i32 = 6;
pub const MOUSE_BUTTON_8: i32 = 7;
pub const MOUSE_BUTTON_LEFT: i32 = MOUSE_BUTTON_1;
pub const MOUSE_BUTTON_RIGHT: i32 = MOUSE_BUTTON_2;
pub const MOUSE_BUTTON_MIDDLE: i32 = MOUSE_BUTTON_3;

pub const MOD_SHIFT: i32 = 0x0001;
pub const MOD_CONTROL: i32 = 0x0002;
pub const MOD_ALT: i32 = 0x0004;
pub const MOD_SUPER: i32 = 0x0008;
pub const MOD_CAPS_LOCK: i32 = 0x0010;
pub const MOD_NUM_LOCK: i32 = 0x0020;

pub const KEY_UNKNOWN: i32 = -1;
pub const KEY_SPACE: i32 = 32;
pub const KEY_APOSTROPHE: i32 = 39;
pub const KEY_COMMA: i32 = 44;
pub const KEY_MINUS: i32 = 45;
pub const KEY_PERIOD: i32 = 46;
pub const KEY_SLASH: i32 = 47;
pub const KEY_0: i32 = 48;
pub const KEY_1: i32 = 49;
pub const KEY_2: i32 = 50;
pub const KEY_3: i32 = 51;
pub const KEY_4: i32 = 52;
pub const KEY_5: i32 = 53;
pub const KEY_6: i32 = 54;
pub const KEY_7: i32 = 55;
pub const KEY_8: i32 = 56;
pub const KEY_9: i32 = 57;
pub const KEY_SEMICOLON: i32 = 59;
pub const KEY_EQUAL: i32 = 61;
pub const KEY_A: i32 = 65;
pub const KEY_B: i32 = 66;
pub const KEY_C: i32 = 67;
pub const KEY_D: i32 = 68;
pub const KEY_E: i32 = 69;
pub const KEY_F: i32 = 70;
pub const KEY_G: i32 = 71;
pub const KEY_H: i32 = 72;
pub const KEY_I: i32 = 73;
pub const KEY_J: i32 = 74;
pub const KEY_K: i32 = 75;
pub const KEY_L: i32 = 76;
pub const KEY_M: i32 = 77;
pub const KEY_N: i32 = 78;
pub const KEY_O: i32 = 79;
pub const KEY_P: i32 = 80;
pub const KEY_Q: i32 = 81;
pub const KEY_R: i32 = 82;
pub const KEY_S: i32 = 83;
pub const KEY_T: i32 = 84;
pub const KEY_U: i32 = 85;
pub const KEY_V: i32 = 86;
pub const KEY_W: i32 = 87;
pub const KEY_X: i32 = 88;
pub const KEY_Y: i32 = 89;
pub const KEY_Z: i32 = 90;
pub const KEY_LEFT_BRACKET: i32 = 91;
pub const KEY_BACKSLASH: i32 = 92;
pub const KEY_RIGHT_BRACKET: i32 = 93;
pub const KEY_GRAVE_ACCENT: i32 = 96;
pub const KEY_WORLD_1: i32 = 161;
pub const KEY_WORLD_2: i32 = 162;
pub const KEY_ESCAPE: i32 = 256;
pub const KEY_ENTER: i32 = 257;
pub const KEY_TAB: i32 = 258;
pub const KEY_BACKSPACE: i32 = 259;
pub const KEY_INSERT: i32 = 260;
pub const KEY_DELETE: i32 = 261;
pub const KEY_RIGHT: i32 = 262;
pub const KEY_LEFT: i32 = 263;
pub const KEY_DOWN: i32 = 264;
pub const KEY_UP: i32 = 265;
pub const KEY_PAGE_UP: i32 = 266;
pub const KEY_PAGE_DOWN: i32 = 267;
pub const KEY_HOME: i32 = 268;
pub const KEY_END: i32 = 269;
pub const KEY_CAPS_LOCK: i32 = 280;
pub const KEY_SCROLL_LOCK: i32 = 281;
pub const KEY_NUM_LOCK: i32 = 282;
pub const KEY_PRINT_SCREEN: i32 = 283;
pub const KEY_PAUSE: i32 = 284;
pub const KEY_F1: i32 = 290;
pub const KEY_F2: i32 = 291;
pub const KEY_F3: i32 = 292;
pub const KEY_F4: i32 = 293;
pub const KEY_F5: i32 = 294;
pub const KEY_F6: i32 = 295;
pub const KEY_F7: i32 = 296;
pub const KEY_F8: i32 = 297;
pub const KEY_F9: i32 = 298;
pub const KEY_F10: i32 = 299;
pub const KEY_F11: i32 = 300;
pub const KEY_F12: i32 = 301;
pub const KEY_F13: i32 = 302;
pub const KEY_F14: i32 = 303;
pub const KEY_F15: i32 = 304;
pub const KEY_F16: i32 = 305;
pub const KEY_F17: i32 = 306;
pub const KEY_F18: i32 = 307;
pub const KEY_F19: i32 = 308;
pub const KEY_F20: i32 = 309;
pub const KEY_F21: i32 = 310;
pub const KEY_F22: i32 = 311;
pub const KEY_F23: i32 = 312;
pub const KEY_F24: i32 = 313;
pub const KEY_F25: i32 = 314;
pub const KEY_KP_0: i32 = 320;
pub const KEY_KP_1: i32 = 321;
pub const KEY_KP_2: i32 = 322;
pub const KEY_KP_3: i32 = 323;
pub const KEY_KP_4: i32 = 324;
pub const KEY_KP_5: i32 = 325;
pub const KEY_KP_6: i32 = 326;
pub const KEY_KP_7: i32 = 327;
pub const KEY_KP_8: i32 = 328;
pub const KEY_KP_9: i32 = 329;
pub const KEY_KP_DECIMAL: i32 = 330;
pub const KEY_KP_DIVIDE: i32 = 331;
pub const KEY_KP_MULTIPLY: i32 = 332;
pub const KEY_KP_SUBTRACT: i32 = 333;
pub const KEY_KP_ADD: i32 = 334;
pub const KEY_KP_ENTER: i32 = 335;
pub const KEY_KP_EQUAL: i32 = 336;
pub const KEY_LEFT_SHIFT: i32 = 340;
pub const KEY_LEFT_CONTROL: i32 = 341;
pub const KEY_LEFT_ALT: i32 = 342;
pub const KEY_LEFT_SUPER: i32 = 343;
pub const KEY_RIGHT_SHIFT: i32 = 344;
pub const KEY_RIGHT_CONTROL: i32 = 345;
pub const KEY_RIGHT_ALT: i32 = 346;
pub const KEY_RIGHT_SUPER: i32 = 347;
pub const KEY_MENU: i32 = 348;

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
