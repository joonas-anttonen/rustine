#![allow(dead_code)]

mod input;
pub use input::*;

use crate::gfx::{self, presentation, vulkan as vk};
use crate::*;
use crate::{debug, warning};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Parameters {
    pub platform: Platform,
    pub window_title: String,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub window_type: WindowType,
}

pub enum WindowType {
    Normal,
    Background,
    Taskbar,
    Popup,
}

pub struct GuiBuilder {
    params: Parameters,
}

impl GuiBuilder {
    /// Creates a new `GuiBuilder` with the given platform.
    pub fn new(platform: Platform) -> Self {
        Self {
            params: Parameters {
                platform,
                window_title: String::from("rustine::gui"),
                window_width: None,
                window_height: None,
                window_type: WindowType::Normal,
            },
        }
    }

    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.params.window_title = title.into();
        self
    }

    pub fn window_width(mut self, width: u32) -> Self {
        self.params.window_width = Some(width);
        self
    }

    pub fn window_height(mut self, height: u32) -> Self {
        self.params.window_height = Some(height);
        self
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.params.window_width = Some(width);
        self.params.window_height = Some(height);
        self
    }

    pub fn window_type(mut self, window_type: WindowType) -> Self {
        self.params.window_type = window_type;
        self
    }

    /// Builds the `Gui` instance.
    pub fn build(self, gfx: Arc<Mutex<gfx::Gfx>>, application: Box<dyn Application>) -> Arc<Gui> {
        Gui::new(gfx, application, self.params)
    }
}

pub trait Application {
    fn startup(&self, _gui: &Gui) {}
    fn on_key(&self, _gui: &Gui, _key: input::KeyEvent) {}
    fn on_char(&self, _gui: &Gui, _char: char) {}
    fn render(&self, _gui: &Gui, _frame: &mut gfx::RenderFrame) {}
}

pub struct Gui {
    gfx: Arc<Mutex<gfx::Gfx>>,
    gfx_surface: vk::VkSurfaceKHR,
    rwl_window: ffi::RwlWindow,

    application: Box<dyn Application>,
    damaged: AtomicBool,
}

impl Drop for Gui {
    fn drop(&mut self) {
        warning!("Gui::drop");

        let mut gfx = self.gfx.lock().unwrap();
        gfx.drop_presentation();

        unsafe {
            if !self.gfx_surface.is_null() {
                gfx.instance().destroy_surface(self.gfx_surface);
            }

            ffi::panic_if_error(ffi::rwlDestroyWindow(self.rwl_window));
            ffi::panic_if_error(ffi::rwlShutdown());
        }
    }
}

impl Gui {
    /// Wakes up the GUI event loop by posting an empty event.
    pub fn wake_up() {
        unsafe {
            ffi::rwlPostEmptyEvent();
        }
    }

    /// Requests the GUI to quit.
    ///
    /// Automatically wakes up the GUI event loop.
    pub fn request_quit(&self) {
        unsafe {
            ffi::rwlWindowRequestClose(self.rwl_window);
            ffi::rwlPostEmptyEvent();
        }
    }

    /// Marks the window as damaged, indicating that it needs to be redrawn.
    pub fn mark_damaged(&self) {
        self.damaged.store(true, Ordering::Release);
    }

    /// Checks if the window is damaged and clears the damaged flag.
    /// Returns true if the window was marked as damaged.
    fn is_damaged(&self) -> bool {
        self.damaged.swap(false, Ordering::Acquire)
    }

    pub fn run(gui: &gui::Gui, exit_flag: &std::sync::atomic::AtomicBool, mode: gfx::LoopMode) {
        info!("GUI START");

        //let start_instant = std::time::Instant::now();
        let mut last_instant = std::time::Instant::now();
        let mut last_stat_instant = std::time::Instant::now();
        let mut frame_delta_times: RingBuffer<f64> = RingBuffer::new(120);

        const TARGET_FPS: f64 = 120.0;
        let target_frame_time = std::time::Duration::from_secs_f64(1.0 / TARGET_FPS);
        const CLOSE_ENOUGH: std::time::Duration = std::time::Duration::from_micros(500);

        gui.application.startup(gui);

        while !exit_flag.load(std::sync::atomic::Ordering::Relaxed) && !gui.should_close() {
            let frame_start = std::time::Instant::now();

            if let gfx::LoopMode::Event = mode {
                gui.wait_events();
            } else {
                gui.process_events();
            }

            // Only render if the window is damaged
            if gui.is_damaged() {
                //let t = now.duration_since(start_instant).as_secs_f64();
                let dt = frame_start.duration_since(last_instant).as_secs_f32();
                frame_delta_times.push(dt as f64);
                last_instant = frame_start;

                let mut frame = gfx::RenderFrame::new(gui.pixel_size());
                gui.application.render(gui, &mut frame);

                {
                    let gfx = gui.gfx.lock().unwrap();

                    if let Ok(mut pending) = gfx.render_mailbox().lock() {
                        pending.push_back(frame);

                        if let gfx::LoopMode::Event = mode {
                            gfx.wake_up();
                        }
                    }
                }

                if frame_start.duration_since(last_stat_instant).as_secs_f64() >= 1.0 {
                    if let Some((min, max, mean)) = frame_delta_times.min_max_mean() {
                        debug!(
                            "GUI frame dt -> min: {}, max: {}, mean: {}",
                            utilities::format_duration(min),
                            utilities::format_duration(max),
                            utilities::format_duration(mean)
                        );
                    }

                    last_stat_instant = frame_start;
                }
            }

            let elapsed = frame_start.elapsed();
            if elapsed < target_frame_time {
                let mut remaining = target_frame_time - elapsed;

                // Sleep for bulk of remaining time
                while remaining > CLOSE_ENOUGH {
                    gui.wait_events_timeout_ms(1);
                    remaining = target_frame_time.saturating_sub(frame_start.elapsed());
                }
            }
        }

        info!("GUI STOP");
    }

    /// Creates a new `GuiBuilder` to configure and build a `Gui` instance.
    pub fn builder(platform: Platform) -> GuiBuilder {
        GuiBuilder::new(platform)
    }

    pub fn new(
        gfx: Arc<Mutex<gfx::Gfx>>,
        application: Box<dyn Application>,
        parameters: Parameters,
    ) -> Arc<Self> {
        if parameters.platform != Platform::Wayland {
            panic!("Unsupported platform");
        }

        let rwl_window = unsafe {
            ffi::rwlSetLogCallback(ffi::RwlLogSeverity::Error, Self::rwl_log_callback);
            ffi::panic_if_error(ffi::rwlStartup());

            // Outputs
            let mut output_count: u32 = 0;
            ffi::panic_if_error(ffi::rwlEnumerateOutputs(
                &mut output_count,
                std::ptr::null_mut(),
            ));
            let mut outputs: Vec<ffi::RwlOutputInfo> = Vec::with_capacity(output_count as usize);
            ffi::panic_if_error(ffi::rwlEnumerateOutputs(
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
            let _output = {
                outputs
                    .iter()
                    .find(|o| std::ffi::CStr::from_ptr(o.name).to_string_lossy() == "asdasd")
                    .map(|o| o.wl_output)
                    .unwrap_or(std::ptr::null_mut())
            };

            let rwl_window_params = match parameters.window_type {
                WindowType::Normal => (
                    ffi::RwlWindowType::Normal,
                    std::ptr::null_mut(),
                    parameters.window_width.unwrap_or(1280),
                    parameters.window_height.unwrap_or(720),
                ),
                WindowType::Background => (
                    ffi::RwlWindowType::Background,
                    std::ptr::null_mut(),
                    parameters.window_width.unwrap_or(0),
                    parameters.window_height.unwrap_or(0),
                ),
                WindowType::Popup => (
                    ffi::RwlWindowType::Popup,
                    std::ptr::null_mut(),
                    parameters.window_width.unwrap_or(1280),
                    parameters.window_height.unwrap_or(720),
                ),
                WindowType::Taskbar => (
                    ffi::RwlWindowType::Taskbar,
                    std::ptr::null_mut(),
                    parameters.window_width.unwrap_or(0),
                    parameters.window_height.unwrap_or(48),
                ),
            };

            let mut rwl_window = std::ptr::null_mut();
            ffi::panic_if_error(ffi::rwlCreateWindow(
                rwl_window_params.0,
                rwl_window_params.1,
                rwl_window_params.2,
                rwl_window_params.3,
                &mut rwl_window,
            ));

            ffi::panic_if_error(ffi::rwlSetPixelSizeCallback(
                rwl_window,
                Self::rwl_pixel_size_callback,
            ));

            ffi::panic_if_error(ffi::rwlSetLogicalSizeCallback(
                rwl_window,
                Self::rwl_logical_size_callback,
            ));

            ffi::panic_if_error(ffi::rwlSetKeyCallback(rwl_window, Self::rwl_key_callback));
            ffi::panic_if_error(ffi::rwlSetCharCallback(rwl_window, Self::rwl_char_callback));

            rwl_window
        };

        let mut wl_output = std::ptr::null_mut();
        let mut wl_surface = std::ptr::null_mut();
        unsafe {
            ffi::panic_if_error(ffi::rwlGetWaylandHandles(
                rwl_window,
                &mut wl_output,
                &mut wl_surface,
            ));
        }

        let gfx_surface = gfx
            .lock()
            .unwrap()
            .instance()
            .create_wayland_surface(wl_output as *const _, wl_surface as *const _);

        let gui = Arc::new(Self {
            gfx,
            gfx_surface,
            rwl_window,
            application,
            damaged: AtomicBool::new(true),
        });

        unsafe {
            let gui_raw_ptr = Arc::as_ptr(&gui);
            ffi::panic_if_error(ffi::rwlSetWindowUserPointer(
                gui.rwl_window,
                gui_raw_ptr as *const _,
            ));

            // Manually invoke the framebuffer size callback to initialize the swapchain
            let mut width: u32 = 0;
            let mut height: u32 = 0;
            ffi::panic_if_error(ffi::rwlGetPixelSize(
                gui.rwl_window,
                &mut width,
                &mut height,
            ));
            Self::rwl_pixel_size_callback(gui.rwl_window, width, height);
        }

        gui
    }

    pub fn should_close(&self) -> bool {
        unsafe { ffi::rwlWindowShouldClose(self.rwl_window) }
    }

    pub fn process_events(&self) {
        unsafe {
            ffi::panic_if_error(ffi::rwlPollEvents());
        }
    }

    pub fn wait_events(&self) {
        unsafe {
            ffi::panic_if_error(ffi::rwlWaitEvents());
        }
    }

    pub fn wait_events_timeout_ms(&self, timeout_ms: u32) {
        unsafe {
            ffi::panic_if_error(ffi::rwlWaitEventsTimeout(
                timeout_ms as u64 * 1000u64 * 1000u64,
            ));
        }
    }

    pub fn pixel_size(&self) -> Vector2u {
        let mut width: u32 = 0;
        let mut height: u32 = 0;

        unsafe {
            ffi::panic_if_error(ffi::rwlGetPixelSize(
                self.rwl_window,
                &mut width,
                &mut height,
            ));
        }

        Vector2u::new(width, height)
    }

    pub fn create_dynamic_image(&self) -> gfx::Image {
        let mut gfx = self.gfx.lock().unwrap();
        gfx.create_dynamic_image()
    }

    pub fn image_mailbox(&self) -> Arc<Mutex<std::collections::VecDeque<(u32, crate::io::Image)>>> {
        let gfx = self.gfx.lock().unwrap();
        gfx.image_mailbox()
    }

    unsafe extern "C" fn rwl_pixel_size_callback(window: ffi::RwlWindow, width: u32, height: u32) {
        unsafe {
            let gui_ptr = ffi::rwlGetWindowUserPointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                debug!("Pixel size changed: {}x{}", width, height);

                let gui = &mut *gui_ptr;
                let mut gfx = gui.gfx.lock().unwrap();

                gui.mark_damaged();

                // When minimized, width and height can be zero
                // but we can't create a swapchain with zero dimensions
                if width == 0 || height == 0 {
                    return;
                }

                let presentation_parameters = presentation::Parameters {
                    width,
                    height,
                    surface_handle: gui.gfx_surface.to_ptr(),
                    vertical_sync: 0,
                };

                gfx.initialize_swapchain(presentation_parameters);
            }
        }
    }

    unsafe extern "C" fn rwl_logical_size_callback(
        _window: ffi::RwlWindow,
        _width: u32,
        _height: u32,
    ) {
        // Implement when and if needed
    }

    unsafe extern "C" fn rwl_key_callback(
        window: ffi::RwlWindow,
        key: ffi::RwlKey,
        scancode: i32,
        action: ffi::RwlAction,
        mods: ffi::RwlMod,
    ) {
        unsafe {
            let gui_ptr = ffi::rwlGetWindowUserPointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let key_event = input::KeyEvent {
                    key: key.to_input(),
                    action: action.to_input(),
                    mods: mods.to_input(),
                    scancode: scancode as u32,
                };

                let gui = &mut *gui_ptr;

                gui.application.on_key(gui, key_event);
            }
        }
    }

    unsafe extern "C" fn rwl_char_callback(window: ffi::RwlWindow, codepoint: u32) {
        unsafe {
            let gui_ptr = ffi::rwlGetWindowUserPointer(window) as *mut Gui;
            if !gui_ptr.is_null()
                && let Some(c) = char::from_u32(codepoint)
            {
                let gui = &mut *gui_ptr;
                gui.application.on_char(gui, c);
            }
        }
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

mod ffi {
    /// Opaque Wayland window handle
    pub type RwlWindow = *mut std::ffi::c_void;

    /// Opaque output handle
    pub type RwlOutput = *mut std::ffi::c_void;

    /// Window type for different Wayland layer shell surfaces
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    pub enum RwlWindowType {
        Normal = 0,
        /// Desktop background (bottom layer, covers full screen)
        Background = 1,
        /// Taskbar/panel (top layer, typically anchored to top)
        Taskbar = 2,
        /// Popup window (top layer, typically transient)
        Popup = 3,
    }

    /// Key action states
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct RwlAction(u32);

    impl RwlAction {
        pub const RELEASE: u32 = 0;
        pub const PRESS: u32 = 1;

        pub fn to_input(self) -> crate::gui::input::Action {
            match self.0 {
                Self::RELEASE => crate::gui::input::Action::RELEASE,
                Self::PRESS => crate::gui::input::Action::PRESS,
                _ => crate::gui::input::Action::RELEASE,
            }
        }
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

        pub fn to_input(self) -> crate::gui::input::Mods {
            crate::gui::input::Mods(self.0 as i32)
        }
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

        pub fn to_input(self) -> crate::gui::input::Key {
            match self.0 {
                Self::SPACE => crate::gui::input::Key::SPACE,
                Self::ESCAPE => crate::gui::input::Key::ESCAPE,
                Self::ENTER => crate::gui::input::Key::ENTER,
                Self::TAB => crate::gui::input::Key::TAB,
                Self::E => crate::gui::input::Key::E,
                Self::M => crate::gui::input::Key::M,
                Self::UP => crate::gui::input::Key::UP,
                Self::DOWN => crate::gui::input::Key::DOWN,
                Self::LEFT => crate::gui::input::Key::LEFT,
                Self::RIGHT => crate::gui::input::Key::RIGHT,
                Self::PERIOD => crate::gui::input::Key::PERIOD,
                Self::MINUS => crate::gui::input::Key::MINUS,
                Self::PAGE_UP => crate::gui::input::Key::PAGE_UP,
                Self::PAGE_DOWN => crate::gui::input::Key::PAGE_DOWN,
                Self::HOME => crate::gui::input::Key::HOME,
                Self::END => crate::gui::input::Key::END,
                Self::F1 => crate::gui::input::Key::F1,
                Self::F2 => crate::gui::input::Key::F2,
                Self::F3 => crate::gui::input::Key::F3,
                Self::F4 => crate::gui::input::Key::F4,
                Self::F5 => crate::gui::input::Key::F5,
                Self::F6 => crate::gui::input::Key::F6,
                Self::F7 => crate::gui::input::Key::F7,
                Self::F8 => crate::gui::input::Key::F8,
                Self::F9 => crate::gui::input::Key::F9,
                Self::F10 => crate::gui::input::Key::F10,
                Self::F11 => crate::gui::input::Key::F11,
                Self::F12 => crate::gui::input::Key::F12,
                _ => crate::gui::input::Key::UNKNOWN,
            }
        }
    }

    /// Output information structure
    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct RwlOutputInfo {
        /// Opaque pointer to the wl_output
        pub wl_output: *mut std::ffi::c_void,
        /// Output name (e.g., "HDMI-1", "DP-2")
        pub name: *const std::ffi::c_char,
        /// Output description
        pub description: *const std::ffi::c_char,
        /// Scale factor
        pub scale: i32,
        /// Physical width in pixels
        pub width: i32,
        /// Physical height in pixels
        pub height: i32,
    }

    /// Callback for pixel size changes (e.g., when compositor configures the surface)
    pub type RwlPixelSizeCallback =
        unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

    /// Callback for logical size changes (e.g., when compositor configures the surface)
    pub type RwlLogicalSizeCallback =
        unsafe extern "C" fn(window: RwlWindow, width: u32, height: u32);

    /// Callback for key events
    pub type RwlKeyCallback = unsafe extern "C" fn(
        window: RwlWindow,
        key: RwlKey,
        scancode: i32,
        action: RwlAction,
        mods: RwlMod,
    );

    /// Callback for character input (UTF-32 codepoint)
    pub type RwlCharCallback = unsafe extern "C" fn(window: RwlWindow, codepoint: u32);

    /// Callback for logging messages from the library
    /// severity: 0=Debug, 1=Info, 2=Warning, 3=Error
    pub type RwlLogCallback = unsafe extern "C" fn(severity: u32, message: *const std::ffi::c_char);

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
            output: *const std::ffi::c_void,
            width: u32,
            height: u32,
            window_out: *mut RwlWindow,
        ) -> RwlStatus;

        pub fn rwlDestroyWindow(window: RwlWindow) -> RwlStatus;

        pub fn rwlSetWindowUserPointer(
            window: RwlWindow,
            pointer: *const std::ffi::c_void,
        ) -> RwlStatus;
        pub fn rwlGetWindowUserPointer(window: RwlWindow) -> *mut std::ffi::c_void;

        pub fn rwlSetPixelSizeCallback(
            window: RwlWindow,
            callback: RwlPixelSizeCallback,
        ) -> RwlStatus;
        pub fn rwlSetLogicalSizeCallback(
            window: RwlWindow,
            callback: RwlLogicalSizeCallback,
        ) -> RwlStatus;

        pub fn rwlSetKeyCallback(window: RwlWindow, callback: RwlKeyCallback) -> RwlStatus;
        pub fn rwlSetCharCallback(window: RwlWindow, callback: RwlCharCallback) -> RwlStatus;

        pub fn rwlGetPixelSize(window: RwlWindow, width: *mut u32, height: *mut u32) -> RwlStatus;
        pub fn rwlGetLogicalSize(window: RwlWindow, width: *mut u32, height: *mut u32)
        -> RwlStatus;

        pub fn rwlGetWaylandHandles(
            window: RwlWindow,
            out_display: *mut *mut std::ffi::c_void,
            out_surface: *mut *mut std::ffi::c_void,
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
}
