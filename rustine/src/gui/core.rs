use crate::{Mailbox, RingBuffer, RunMode, Vector2f, Vector2i, Vector2u, log, utilities};

use crate::gui::*;

pub struct Parameters {
    pub window_title: String,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

pub struct GuiBuilder {
    params: Parameters,
}

impl GuiBuilder {
    /// Creates a new `GuiBuilder`.
    pub fn new() -> Self {
        Self {
            params: Parameters {
                window_title: String::from("rustine::gui"),
                window_width: None,
                window_height: None,
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

    /// Builds the `Gui` instance.
    pub fn build(self, gfx: Arc<Mutex<gfx::Gfx>>, application: Box<dyn Application>) -> Rc<Gui> {
        Gui::new(gfx, application, self.params)
    }
}

pub trait Application {
    fn startup(&self, _gui: &Gui) {}
    fn on_key(&self, _gui: &Gui, _key: input::KeyEvent) {}
    fn on_char(&self, _gui: &Gui, _char: char) {}
    fn on_mouse_enter(&self, _gui: &Gui, _event: input::MouseEnterEvent) {}
    fn on_mouse_leave(&self, _gui: &Gui, _event: input::MouseLeaveEvent) {}
    fn on_mouse_move(&self, _gui: &Gui, _event: input::MouseMoveEvent) {}
    fn on_mouse_button(&self, _gui: &Gui, _event: input::MouseButtonEvent) {}
    fn on_mouse_scroll(&self, _gui: &Gui, _event: input::MouseScrollEvent) {}
    fn render(&self, _gui: &Gui, _frame: &mut gfx::RenderFrame) {}
}

pub struct Gui {
    gfx: Arc<Mutex<gfx::Gfx>>,
    gfx_surface: vk::VkSurfaceKHR,
    glfw_window: ffi::GlfwWindow,

    application: Box<dyn Application>,
    damaged: AtomicBool,

    wanted_size: Vector2u,
    logical_size: Vector2u,
    mouse_position: Vector2f,
}

impl Drop for Gui {
    fn drop(&mut self) {
        log::drop!("Gui::drop");

        let mut gfx = self.gfx.lock().unwrap();
        gfx.drop_presentation();

        unsafe {
            if !self.gfx_surface.is_null() {
                gfx.instance().destroy_surface(self.gfx_surface);
            }

            ffi::panic_if_error(ffi::destroy_window(self.glfw_window));
            ffi::panic_if_error(ffi::shutdown());
        }
    }
}

/// Runs the main GUI loop.
///
/// Intended to be called from the main thread.
/// Behavior when calling this from a non-main thread is undefined.
pub fn run(gui: &Gui, mode: RunMode) {
    Gui::run(gui, mode);
}

impl Gui {
    fn create_glfw_window(
        parameters: &Parameters,
        requested_width: u32,
        requested_height: u32,
        platform: crate::Platform,
    ) -> ffi::GlfwWindow {
        unsafe {
            ffi::set_log_callback(ffi::GlfwLogSeverity::Error, Self::glfw_log_callback);
            ffi::panic_if_error(ffi::set_platform_hint(platform));
            ffi::panic_if_error(ffi::startup());

            let mut glfw_window = std::ptr::null_mut();
            ffi::panic_if_error(ffi::create_window(
                requested_width,
                requested_height,
                &mut glfw_window,
            ));

            let title_cstr = std::ffi::CString::new(parameters.window_title.as_str())
                .expect("window title must not contain null bytes");
            ffi::panic_if_error(ffi::set_window_title(glfw_window, title_cstr.as_ptr()));

            ffi::panic_if_error(ffi::set_pixel_size_callback(
                glfw_window,
                Self::glfw_pixel_size_callback,
            ));

            ffi::panic_if_error(ffi::set_logical_size_callback(
                glfw_window,
                Self::glfw_logical_size_callback,
            ));

            ffi::panic_if_error(ffi::set_key_callback(glfw_window, Self::glfw_key_callback));
            ffi::panic_if_error(ffi::set_char_callback(
                glfw_window,
                Self::glfw_char_callback,
            ));
            ffi::panic_if_error(ffi::set_pointer_enter_callback(
                glfw_window,
                Self::glfw_pointer_enter_callback,
            ));
            ffi::panic_if_error(ffi::set_pointer_leave_callback(
                glfw_window,
                Self::glfw_pointer_leave_callback,
            ));
            ffi::panic_if_error(ffi::set_pointer_motion_callback(
                glfw_window,
                Self::glfw_pointer_motion_callback,
            ));
            ffi::panic_if_error(ffi::set_pointer_button_callback(
                glfw_window,
                Self::glfw_pointer_button_callback,
            ));
            ffi::panic_if_error(ffi::set_pointer_scroll_callback(
                glfw_window,
                Self::glfw_pointer_scroll_callback,
            ));

            glfw_window
        }
    }

    /// Wakes up the GUI event loop by posting an empty event.
    pub fn wake_up() {
        unsafe {
            ffi::post_empty_event();
        }
    }

    /// Requests the GUI to quit.
    ///
    /// Automatically wakes up the GUI event loop.
    pub fn request_quit(&self) {
        unsafe {
            ffi::window_request_close(self.glfw_window);
            ffi::post_empty_event();
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

    fn run(gui: &Gui, mode: RunMode) {
        //let start_instant = std::time::Instant::now();
        let mut last_instant = std::time::Instant::now();
        let mut last_stat_instant = std::time::Instant::now();
        let mut frame_delta_times: RingBuffer<f64> = RingBuffer::new(120);

        const TARGET_FPS: f64 = 120.0;
        let target_frame_time = std::time::Duration::from_secs_f64(1.0 / TARGET_FPS);
        const CLOSE_ENOUGH: std::time::Duration = std::time::Duration::from_micros(500);

        gui.application.startup(gui);

        while !crate::should_exit() && !gui.should_close() {
            let frame_start = std::time::Instant::now();

            if let RunMode::Event = mode {
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

                // Try to reuse a cached render frame from the GFX thread to avoid
                // repeated large allocations every frame. Acquire while holding
                // the gfx mutex, then release before invoking the application
                // render callback to avoid deadlocks.
                let mut frame = {
                    let mut gfx = gui.gfx.lock().unwrap();
                    gfx.acquire_frame_for_gui(gui.pixel_size())
                };

                gui.application.render(gui, &mut frame);

                {
                    let gfx = gui.gfx.lock().unwrap();

                    gfx.render_mailbox().push(frame);

                    if let RunMode::Event = mode {
                        gfx.wake_up();
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
    }

    /// Creates a new `GuiBuilder` to configure and build a `Gui` instance.
    pub fn builder() -> GuiBuilder {
        GuiBuilder::new()
    }

    pub fn new(
        gfx: Arc<Mutex<gfx::Gfx>>,
        application: Box<dyn Application>,
        parameters: Parameters,
    ) -> Rc<Self> {
        let requested_width = parameters.window_width.unwrap_or(1280);
        let requested_height = parameters.window_height.unwrap_or(720);
        let wanted_size = Vector2u::new(requested_width, requested_height);

        let selected_platform = {
            let gfx_guard = gfx.lock().unwrap();
            gfx_guard.surface_platform_hint()
        };
        let glfw_window = Self::create_glfw_window(
            &parameters,
            requested_width,
            requested_height,
            selected_platform,
        );

        let mut raw_surface: *const std::ffi::c_void = std::ptr::null();
        let surface_status: ffi::GlfwStatus;
        {
            let gfx_guard = gfx.lock().unwrap();
            let raw_instance = gfx_guard.instance().handle().0;
            unsafe {
                surface_status =
                    ffi::create_window_surface(glfw_window, raw_instance, &mut raw_surface);
            }
        }
        ffi::panic_if_error(surface_status);
        let gfx_surface = vk::VkSurfaceKHR(raw_surface);

        let mut initial_logical_size = wanted_size;
        unsafe {
            let mut logical_width: u32 = 0;
            let mut logical_height: u32 = 0;
            if ffi::get_logical_size(glfw_window, &mut logical_width, &mut logical_height)
                == ffi::GlfwStatus::Ok
            {
                initial_logical_size = Vector2u::new(logical_width, logical_height);
            }
        }

        let gui = Rc::new(Self {
            gfx,
            gfx_surface,
            glfw_window,
            application,
            damaged: AtomicBool::new(true),
            wanted_size,
            logical_size: initial_logical_size,
            mouse_position: Vector2f::default(),
        });

        unsafe {
            let gui_raw_ptr = Rc::as_ptr(&gui);
            ffi::panic_if_error(ffi::set_window_user_pointer(
                gui.glfw_window,
                gui_raw_ptr as *const _,
            ));

            // Manually invoke the framebuffer size callback to initialize the swapchain.
            // This is necessary since some platforms won't trigger the callback
            // until the window is resized, and on Wayland we end up with no visible window at all.
            let mut width: u32 = 0;
            let mut height: u32 = 0;
            ffi::panic_if_error(ffi::get_pixel_size(
                gui.glfw_window,
                &mut width,
                &mut height,
            ));
            Self::glfw_pixel_size_callback(gui.glfw_window, width, height);
        }

        gui
    }

    pub fn should_close(&self) -> bool {
        unsafe { ffi::window_should_close(self.glfw_window) }
    }

    pub fn process_events(&self) {
        unsafe {
            ffi::panic_if_error(ffi::poll_events());
        }
    }

    pub fn wait_events(&self) {
        unsafe {
            ffi::panic_if_error(ffi::wait_events());
        }
    }

    pub fn wait_events_timeout_ms(&self, timeout_ms: u32) {
        unsafe {
            ffi::panic_if_error(ffi::wait_events_timeout(
                timeout_ms as u64 * 1000u64 * 1000u64,
            ));
        }
    }

    pub fn pixel_size(&self) -> Vector2u {
        let mut width: u32 = 0;
        let mut height: u32 = 0;

        unsafe {
            ffi::panic_if_error(ffi::get_pixel_size(
                self.glfw_window,
                &mut width,
                &mut height,
            ));
        }

        Vector2u::new(width, height)
    }

    pub fn logical_size(&self) -> Vector2u {
        self.logical_size
    }

    fn pointer_position_from_logical(&self, x: f64, y: f64) -> Vector2f {
        let logical = self.logical_size;
        if logical.x > 0 && logical.y > 0 {
            let pixel = self.pixel_size();
            let scale_x = pixel.x as f64 / logical.x as f64;
            let scale_y = pixel.y as f64 / logical.y as f64;
            Vector2f::new((x * scale_x) as f32, (y * scale_y) as f32)
        } else {
            Vector2f::new(x as f32, y as f32)
        }
    }

    fn pointer_delta_from_logical(&self, dx: f64, dy: f64) -> Vector2f {
        let logical = self.logical_size;
        if logical.x > 0 && logical.y > 0 {
            let pixel = self.pixel_size();
            let scale_x = pixel.x as f64 / logical.x as f64;
            let scale_y = pixel.y as f64 / logical.y as f64;
            Vector2f::new((dx * scale_x) as f32, (dy * scale_y) as f32)
        } else {
            Vector2f::new(dx as f32, dy as f32)
        }
    }

    pub fn create_dynamic_image(&self) -> gfx::Image {
        let mut gfx = self.gfx.lock().unwrap();
        gfx.create_dynamic_image()
    }

    pub fn image_mailbox(&self) -> Arc<Mailbox<(u32, crate::io::Image)>> {
        let gfx = self.gfx.lock().unwrap();
        gfx.image_mailbox()
    }

    unsafe extern "C" fn glfw_pixel_size_callback(
        window: ffi::GlfwWindow,
        width: u32,
        height: u32,
    ) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let mut gfx = gui.gfx.lock().unwrap();

                gui.mark_damaged();

                // If we have zero width or height, use the wanted size instead
                let requested_w = if width == 0 { gui.wanted_size.x } else { width };
                let requested_h = if height == 0 {
                    gui.wanted_size.y
                } else {
                    height
                };

                let presentation_parameters = presentation::Parameters {
                    width: requested_w,
                    height: requested_h,
                    surface_handle: gui.gfx_surface.to_ptr(),
                    vertical_sync: 0,
                };

                gfx.initialize_swapchain(presentation_parameters);
            }
        }
    }

    unsafe extern "C" fn glfw_logical_size_callback(
        window: ffi::GlfwWindow,
        width: u32,
        height: u32,
    ) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                gui.logical_size = Vector2u::new(width, height);
            }
        }
    }

    unsafe extern "C" fn glfw_key_callback(
        window: ffi::GlfwWindow,
        key: ffi::GlfwKey,
        scancode: i32,
        action: ffi::GlfwAction,
        mods: ffi::GlfwMod,
    ) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let key_event = input::KeyEvent {
                    key: key.to_input(),
                    action: action.to_input(),
                    mods: mods.to_input(),
                    scancode: scancode as u32,
                };

                if key_event.key == Key::UNKNOWN {
                    log::warning!("Gui::on_key: {:?} {:?}", key_event.key, key_event.action);
                }

                let gui = &mut *gui_ptr;

                gui.application.on_key(gui, key_event);
            }
        }
    }

    unsafe extern "C" fn glfw_char_callback(window: ffi::GlfwWindow, codepoint: u32) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null()
                && let Some(c) = char::from_u32(codepoint)
            {
                let gui = &mut *gui_ptr;
                gui.application.on_char(gui, c);
            }
        }
    }

    unsafe extern "C" fn glfw_pointer_enter_callback(window: ffi::GlfwWindow, x: f64, y: f64) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let position = gui.pointer_position_from_logical(x, y);
                gui.mouse_position = position;

                let event = input::MouseEnterEvent { position };
                gui.application.on_mouse_enter(gui, event);
            }
        }
    }

    unsafe extern "C" fn glfw_pointer_leave_callback(window: ffi::GlfwWindow, x: f64, y: f64) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let position = gui.pointer_position_from_logical(x, y);
                gui.mouse_position = position;

                let event = input::MouseLeaveEvent { position };
                gui.application.on_mouse_leave(gui, event);
            }
        }
    }

    unsafe extern "C" fn glfw_pointer_motion_callback(window: ffi::GlfwWindow, x: f64, y: f64) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let position = gui.pointer_position_from_logical(x, y);
                gui.mouse_position = position;

                let event = input::MouseMoveEvent { position };
                gui.application.on_mouse_move(gui, event);
            }
        }
    }

    unsafe extern "C" fn glfw_pointer_button_callback(
        window: ffi::GlfwWindow,
        x: f64,
        y: f64,
        button: ffi::GlfwMouseButton,
        action: ffi::GlfwAction,
        mods: ffi::GlfwMod,
    ) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let position = gui.pointer_position_from_logical(x, y);
                gui.mouse_position = position;

                let event = input::MouseButtonEvent {
                    position,
                    button: button.to_input(),
                    action: action.to_input(),
                    mods: mods.to_input(),
                };
                gui.application.on_mouse_button(gui, event);
            }
        }
    }

    unsafe extern "C" fn glfw_pointer_scroll_callback(
        window: ffi::GlfwWindow,
        x: f64,
        y: f64,
        delta_x: f64,
        delta_y: f64,
        delta_discrete_x: i32,
        delta_discrete_y: i32,
        mods: ffi::GlfwMod,
    ) {
        unsafe {
            let gui_ptr = ffi::get_window_user_pointer(window) as *mut Gui;
            if !gui_ptr.is_null() {
                let gui = &mut *gui_ptr;
                let position = gui.pointer_position_from_logical(x, y);
                let delta = gui.pointer_delta_from_logical(delta_x, delta_y);
                log::debug!(
                    "Mouse scroll at logical position: {:?}, delta: {:?}, discrete delta: ({}, {})",
                    position,
                    delta,
                    delta_discrete_x,
                    delta_discrete_y
                );
                let event = input::MouseScrollEvent {
                    position,
                    delta,
                    delta_discrete: Vector2i::new(delta_discrete_x, delta_discrete_y),
                    mods: mods.to_input(),
                };
                gui.application.on_mouse_scroll(gui, event);
            }
        }
    }

    unsafe extern "C" fn glfw_log_callback(severity: u32, message: *const std::ffi::c_char) {
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

        crate::log::Log::global().append(sev, &message_str, "glfw");
    }
}

#[allow(nonstandard_style)]
mod ffi {
    /// Opaque GLFW window handle
    pub type GlfwWindow = *mut std::ffi::c_void;

    /// Key action states
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct GlfwAction(u32);

    impl GlfwAction {
        pub const RELEASE: u32 = 0;
        pub const PRESS: u32 = 1;
        pub const REPEAT: u32 = 2;

        pub fn to_input(self) -> crate::gui::input::Action {
            match self.0 {
                Self::RELEASE => crate::gui::input::Action::RELEASE,
                Self::PRESS => crate::gui::input::Action::PRESS,
                Self::REPEAT => crate::gui::input::Action::REPEAT,
                _ => crate::gui::input::Action::RELEASE,
            }
        }
    }

    /// Modifier key flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct GlfwMod(u32);
    impl std::ops::BitOr for GlfwMod {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self::Output {
            GlfwMod(self.0 | rhs.0)
        }
    }
    impl GlfwMod {
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
    pub struct GlfwKey(i32);

    impl GlfwKey {
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
                Self::BACKSPACE => crate::gui::input::Key::BACKSPACE,
                Self::DELETE => crate::gui::input::Key::DELETE,
                Self::TAB => crate::gui::input::Key::TAB,
                Self::Q => crate::gui::input::Key::Q,
                Self::W => crate::gui::input::Key::W,
                Self::E => crate::gui::input::Key::E,
                Self::R => crate::gui::input::Key::R,
                Self::T => crate::gui::input::Key::T,
                Self::Y => crate::gui::input::Key::Y,
                Self::U => crate::gui::input::Key::U,
                Self::I => crate::gui::input::Key::I,
                Self::O => crate::gui::input::Key::O,
                Self::P => crate::gui::input::Key::P,
                Self::A => crate::gui::input::Key::A,
                Self::S => crate::gui::input::Key::S,
                Self::D => crate::gui::input::Key::D,
                Self::F => crate::gui::input::Key::F,
                Self::G => crate::gui::input::Key::G,
                Self::H => crate::gui::input::Key::H,
                Self::J => crate::gui::input::Key::J,
                Self::K => crate::gui::input::Key::K,
                Self::L => crate::gui::input::Key::L,
                Self::Z => crate::gui::input::Key::Z,
                Self::X => crate::gui::input::Key::X,
                Self::C => crate::gui::input::Key::C,
                Self::V => crate::gui::input::Key::V,
                Self::B => crate::gui::input::Key::B,
                Self::N => crate::gui::input::Key::N,
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

    /// Mouse button codes
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct GlfwMouseButton(i32);

    impl GlfwMouseButton {
        pub const UNKNOWN: i32 = 0;
        pub const LEFT: i32 = 1;
        pub const RIGHT: i32 = 2;
        pub const MIDDLE: i32 = 3;
        pub const BACK: i32 = 4;
        pub const FORWARD: i32 = 5;

        pub fn to_input(self) -> crate::gui::input::MouseButton {
            match self.0 {
                Self::LEFT => crate::gui::input::MouseButton::LEFT,
                Self::RIGHT => crate::gui::input::MouseButton::RIGHT,
                Self::MIDDLE => crate::gui::input::MouseButton::MIDDLE,
                Self::BACK => crate::gui::input::MouseButton::BACK,
                Self::FORWARD => crate::gui::input::MouseButton::FORWARD,
                _ => crate::gui::input::MouseButton::UNKNOWN,
            }
        }
    }

    /// Callback for pixel size changes (e.g., when compositor configures the surface)
    pub type GlfwPixelSizeCallback =
        unsafe extern "C" fn(window: GlfwWindow, width: u32, height: u32);

    /// Callback for logical size changes (e.g., when compositor configures the surface)
    pub type GlfwLogicalSizeCallback =
        unsafe extern "C" fn(window: GlfwWindow, width: u32, height: u32);

    /// Callback for key events
    pub type GlfwKeyCallback = unsafe extern "C" fn(
        window: GlfwWindow,
        key: GlfwKey,
        scancode: i32,
        action: GlfwAction,
        mods: GlfwMod,
    );

    /// Callback for character input (UTF-32 codepoint)
    pub type GlfwCharCallback = unsafe extern "C" fn(window: GlfwWindow, codepoint: u32);

    /// Callback for pointer enter events
    pub type GlfwPointerEnterCallback = unsafe extern "C" fn(window: GlfwWindow, x: f64, y: f64);

    /// Callback for pointer leave events
    pub type GlfwPointerLeaveCallback = unsafe extern "C" fn(window: GlfwWindow, x: f64, y: f64);

    /// Callback for pointer motion events
    pub type GlfwPointerMotionCallback = unsafe extern "C" fn(window: GlfwWindow, x: f64, y: f64);

    /// Callback for pointer button events
    pub type GlfwPointerButtonCallback = unsafe extern "C" fn(
        window: GlfwWindow,
        x: f64,
        y: f64,
        button: GlfwMouseButton,
        action: GlfwAction,
        mods: GlfwMod,
    );

    /// Callback for pointer scroll events
    pub type GlfwPointerScrollCallback = unsafe extern "C" fn(
        window: GlfwWindow,
        x: f64,
        y: f64,
        delta_x: f64,
        delta_y: f64,
        delta_discrete_x: i32,
        delta_discrete_y: i32,
        mods: GlfwMod,
    );

    /// Callback for logging messages from the library
    /// severity: 0=Debug, 1=Info, 2=Warning, 3=Error
    pub type GlfwLogCallback =
        unsafe extern "C" fn(severity: u32, message: *const std::ffi::c_char);

    /// Status codes returned by glfw backend functions
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    pub enum GlfwStatus {
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
    pub enum GlfwLogSeverity {
        Debug = 0,
        Info = 1,
        Warning = 2,
        Error = 3,
    }

    impl GlfwStatus {
        /// Check if the status represents an error
        pub fn is_error(self) -> bool {
            self as u32 != 0
        }
    }

    pub fn panic_if_error(status: GlfwStatus) {
        if status.is_error() {
            panic!("GLFW backend error: {:?}", status);
        }
    }

    #[derive(Default, Clone, Copy)]
    struct CallbackState {
        pixel_size: Option<GlfwPixelSizeCallback>,
        logical_size: Option<GlfwLogicalSizeCallback>,
        key: Option<GlfwKeyCallback>,
        char_input: Option<GlfwCharCallback>,
        pointer_enter: Option<GlfwPointerEnterCallback>,
        pointer_leave: Option<GlfwPointerLeaveCallback>,
        pointer_motion: Option<GlfwPointerMotionCallback>,
        pointer_button: Option<GlfwPointerButtonCallback>,
        pointer_scroll: Option<GlfwPointerScrollCallback>,
    }

    static CALLBACKS: std::sync::LazyLock<
        std::sync::Mutex<std::collections::HashMap<usize, CallbackState>>,
    > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    static LOG_CALLBACK: std::sync::Mutex<Option<(GlfwLogSeverity, GlfwLogCallback)>> =
        std::sync::Mutex::new(None);

    const GLFW_FALSE: i32 = 0;
    const GLFW_TRUE: i32 = 1;
    const GLFW_PRESS: i32 = 1;

    const GLFW_CLIENT_API: i32 = 0x00022001;
    const GLFW_NO_API: i32 = 0;
    const GLFW_RESIZABLE: i32 = 0x00020003;
    const GLFW_PLATFORM: i32 = 0x00050003;

    const GLFW_PLATFORM_WIN32: i32 = 0x00060001;
    const GLFW_PLATFORM_COCOA: i32 = 0x00060002;
    const GLFW_PLATFORM_WAYLAND: i32 = 0x00060003;
    const GLFW_PLATFORM_X11: i32 = 0x00060004;

    #[link(name = "glfw3", kind = "static")]
    unsafe extern "C" {
        fn glfwInit() -> i32;
        fn glfwInitHint(hint: i32, value: i32);
        fn glfwTerminate();
        fn glfwPollEvents();
        fn glfwWaitEvents();
        fn glfwWaitEventsTimeout(timeout: f64);
        fn glfwPostEmptyEvent();

        fn glfwWindowHint(hint: i32, value: i32);
        fn glfwCreateWindow(
            width: i32,
            height: i32,
            title: *const std::ffi::c_char,
            monitor: *mut std::ffi::c_void,
            share: *mut std::ffi::c_void,
        ) -> GlfwWindow;
        fn glfwDestroyWindow(window: GlfwWindow);
        fn glfwSetWindowTitle(window: GlfwWindow, title: *const std::ffi::c_char);

        fn glfwWindowShouldClose(window: GlfwWindow) -> i32;
        fn glfwSetWindowShouldClose(window: GlfwWindow, value: i32);

        fn glfwSetWindowUserPointer(window: GlfwWindow, pointer: *mut std::ffi::c_void);
        fn glfwGetWindowUserPointer(window: GlfwWindow) -> *mut std::ffi::c_void;

        fn glfwGetFramebufferSize(window: GlfwWindow, width: *mut i32, height: *mut i32);
        fn glfwGetWindowSize(window: GlfwWindow, width: *mut i32, height: *mut i32);
        fn glfwGetCursorPos(window: GlfwWindow, x: *mut f64, y: *mut f64);
        fn glfwGetKey(window: GlfwWindow, key: i32) -> i32;

        fn glfwCreateWindowSurface(
            instance: *mut std::ffi::c_void,
            window: GlfwWindow,
            allocator: *const std::ffi::c_void,
            surface: *mut *const std::ffi::c_void,
        ) -> i32;

        fn glfwSetErrorCallback(
            callback: Option<unsafe extern "C" fn(i32, *const std::ffi::c_char)>,
        );
        fn glfwSetFramebufferSizeCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, i32, i32)>,
        );
        fn glfwSetWindowSizeCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, i32, i32)>,
        );
        fn glfwSetKeyCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, i32, i32, i32, i32)>,
        );
        fn glfwSetCharCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, u32)>,
        );
        fn glfwSetCursorEnterCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, i32)>,
        );
        fn glfwSetCursorPosCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, f64, f64)>,
        );
        fn glfwSetMouseButtonCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, i32, i32, i32)>,
        );
        fn glfwSetScrollCallback(
            window: GlfwWindow,
            callback: Option<unsafe extern "C" fn(GlfwWindow, f64, f64)>,
        );
    }

    unsafe extern "C" fn glfw_error_callback(_code: i32, message: *const std::ffi::c_char) {
        let guard = LOG_CALLBACK.lock().unwrap();
        if let Some((min_severity, callback)) = *guard {
            if (GlfwLogSeverity::Error as u32) >= (min_severity as u32) {
                unsafe {
                    callback(GlfwLogSeverity::Error as u32, message);
                }
            }
        }
    }

    unsafe extern "C" fn glfw_framebuffer_size_callback(
        window: GlfwWindow,
        width: i32,
        height: i32,
    ) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.pixel_size)
        };
        if let Some(callback) = callback {
            unsafe {
                callback(window, width.max(0) as u32, height.max(0) as u32);
            }
        }
    }

    unsafe extern "C" fn glfw_window_size_callback(window: GlfwWindow, width: i32, height: i32) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.logical_size)
        };
        if let Some(callback) = callback {
            unsafe {
                callback(window, width.max(0) as u32, height.max(0) as u32);
            }
        }
    }

    unsafe extern "C" fn glfw_key_callback(
        window: GlfwWindow,
        key: i32,
        scancode: i32,
        action: i32,
        mods: i32,
    ) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard.get(&(window as usize)).and_then(|state| state.key)
        };
        if let Some(callback) = callback {
            unsafe {
                callback(
                    window,
                    GlfwKey(key),
                    scancode,
                    GlfwAction(action as u32),
                    GlfwMod(mods as u32),
                );
            }
        }
    }

    unsafe extern "C" fn glfw_char_callback(window: GlfwWindow, codepoint: u32) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.char_input)
        };
        if let Some(callback) = callback {
            unsafe {
                callback(window, codepoint);
            }
        }
    }

    unsafe extern "C" fn glfw_cursor_enter_callback(window: GlfwWindow, entered: i32) {
        let (enter_callback, leave_callback) = {
            let guard = CALLBACKS.lock().unwrap();
            if let Some(state) = guard.get(&(window as usize)) {
                (state.pointer_enter, state.pointer_leave)
            } else {
                (None, None)
            }
        };

        let mut x = 0.0;
        let mut y = 0.0;
        unsafe {
            glfwGetCursorPos(window, &mut x, &mut y);
        }

        unsafe {
            if entered == GLFW_TRUE {
                if let Some(callback) = enter_callback {
                    callback(window, x, y);
                }
            } else if let Some(callback) = leave_callback {
                callback(window, x, y);
            }
        }
    }

    unsafe extern "C" fn glfw_cursor_pos_callback(window: GlfwWindow, x: f64, y: f64) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.pointer_motion)
        };
        if let Some(callback) = callback {
            unsafe {
                callback(window, x, y);
            }
        }
    }

    unsafe extern "C" fn glfw_mouse_button_callback(
        window: GlfwWindow,
        button: i32,
        action: i32,
        mods: i32,
    ) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.pointer_button)
        };
        if let Some(callback) = callback {
            let mut x = 0.0;
            let mut y = 0.0;
            unsafe {
                glfwGetCursorPos(window, &mut x, &mut y);
                callback(
                    window,
                    x,
                    y,
                    GlfwMouseButton::from_glfw(button),
                    GlfwAction(action as u32),
                    GlfwMod(mods as u32),
                );
            }
        }
    }

    unsafe extern "C" fn glfw_scroll_callback(window: GlfwWindow, delta_x: f64, delta_y: f64) {
        let callback = {
            let guard = CALLBACKS.lock().unwrap();
            guard
                .get(&(window as usize))
                .and_then(|state| state.pointer_scroll)
        };
        if let Some(callback) = callback {
            let mut x = 0.0;
            let mut y = 0.0;
            unsafe {
                glfwGetCursorPos(window, &mut x, &mut y);
                callback(
                    window,
                    x,
                    y,
                    delta_x,
                    delta_y,
                    delta_x.round() as i32,
                    delta_y.round() as i32,
                    current_mods(window),
                );
            }
        }
    }

    pub unsafe fn set_log_callback(min_severity: GlfwLogSeverity, callback: GlfwLogCallback) {
        {
            let mut guard = LOG_CALLBACK.lock().unwrap();
            *guard = Some((min_severity, callback));
        }
        unsafe {
            glfwSetErrorCallback(Some(glfw_error_callback));
        }
    }

    pub unsafe fn startup() -> GlfwStatus {
        let result = unsafe { glfwInit() };
        if result == GLFW_TRUE {
            GlfwStatus::Ok
        } else {
            GlfwStatus::InternalError
        }
    }

    pub unsafe fn set_platform_hint(platform: crate::Platform) -> GlfwStatus {
        let glfw_platform = match platform {
            crate::Platform::Windows => GLFW_PLATFORM_WIN32,
            crate::Platform::Wayland => GLFW_PLATFORM_WAYLAND,
            crate::Platform::X11 => GLFW_PLATFORM_X11,
            crate::Platform::MacOS => GLFW_PLATFORM_COCOA,
        };

        unsafe {
            glfwInitHint(GLFW_PLATFORM, glfw_platform);
        }

        GlfwStatus::Ok
    }

    pub unsafe fn shutdown() -> GlfwStatus {
        unsafe {
            glfwTerminate();
        }
        GlfwStatus::Ok
    }

    pub unsafe fn create_window(
        width: u32,
        height: u32,
        window_out: *mut GlfwWindow,
    ) -> GlfwStatus {
        if window_out.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        unsafe {
            glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
            glfwWindowHint(GLFW_RESIZABLE, GLFW_TRUE);
        }

        let title = c"rustine::gui";
        let window = unsafe {
            glfwCreateWindow(
                width.max(1) as i32,
                height.max(1) as i32,
                title.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if window.is_null() {
            return GlfwStatus::InternalError;
        }

        {
            let mut guard = CALLBACKS.lock().unwrap();
            guard.insert(window as usize, CallbackState::default());
        }

        unsafe {
            glfwSetFramebufferSizeCallback(window, Some(glfw_framebuffer_size_callback));
            glfwSetWindowSizeCallback(window, Some(glfw_window_size_callback));
            glfwSetKeyCallback(window, Some(glfw_key_callback));
            glfwSetCharCallback(window, Some(glfw_char_callback));
            glfwSetCursorEnterCallback(window, Some(glfw_cursor_enter_callback));
            glfwSetCursorPosCallback(window, Some(glfw_cursor_pos_callback));
            glfwSetMouseButtonCallback(window, Some(glfw_mouse_button_callback));
            glfwSetScrollCallback(window, Some(glfw_scroll_callback));
            *window_out = window;
        }

        GlfwStatus::Ok
    }

    pub unsafe fn destroy_window(window: GlfwWindow) -> GlfwStatus {
        if window.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        {
            let mut guard = CALLBACKS.lock().unwrap();
            guard.remove(&(window as usize));
        }
        unsafe {
            glfwDestroyWindow(window);
        }
        GlfwStatus::Ok
    }

    pub unsafe fn set_window_title(
        window: GlfwWindow,
        title: *const std::ffi::c_char,
    ) -> GlfwStatus {
        if window.is_null() || title.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        unsafe {
            glfwSetWindowTitle(window, title);
        }
        GlfwStatus::Ok
    }

    pub unsafe fn create_window_surface(
        window: GlfwWindow,
        instance: *mut std::ffi::c_void,
        surface_out: *mut *const std::ffi::c_void,
    ) -> GlfwStatus {
        if window.is_null() || instance.is_null() || surface_out.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        let vk_success: i32 = 0;
        let result =
            unsafe { glfwCreateWindowSurface(instance, window, std::ptr::null(), surface_out) };
        if result == vk_success {
            GlfwStatus::Ok
        } else {
            GlfwStatus::InternalError
        }
    }

    pub unsafe fn set_window_user_pointer(
        window: GlfwWindow,
        pointer: *const std::ffi::c_void,
    ) -> GlfwStatus {
        if window.is_null() {
            return GlfwStatus::InvalidArgument;
        }
        unsafe {
            glfwSetWindowUserPointer(window, pointer.cast_mut());
        }
        GlfwStatus::Ok
    }

    pub unsafe fn get_window_user_pointer(window: GlfwWindow) -> *mut std::ffi::c_void {
        if window.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { glfwGetWindowUserPointer(window) }
    }

    pub unsafe fn set_pixel_size_callback(
        window: GlfwWindow,
        callback: GlfwPixelSizeCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pixel_size = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_logical_size_callback(
        window: GlfwWindow,
        callback: GlfwLogicalSizeCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.logical_size = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_key_callback(window: GlfwWindow, callback: GlfwKeyCallback) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.key = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_char_callback(window: GlfwWindow, callback: GlfwCharCallback) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.char_input = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_pointer_enter_callback(
        window: GlfwWindow,
        callback: GlfwPointerEnterCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pointer_enter = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_pointer_leave_callback(
        window: GlfwWindow,
        callback: GlfwPointerLeaveCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pointer_leave = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_pointer_motion_callback(
        window: GlfwWindow,
        callback: GlfwPointerMotionCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pointer_motion = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_pointer_button_callback(
        window: GlfwWindow,
        callback: GlfwPointerButtonCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pointer_button = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn set_pointer_scroll_callback(
        window: GlfwWindow,
        callback: GlfwPointerScrollCallback,
    ) -> GlfwStatus {
        let mut guard = CALLBACKS.lock().unwrap();
        if let Some(state) = guard.get_mut(&(window as usize)) {
            state.pointer_scroll = Some(callback);
            GlfwStatus::Ok
        } else {
            GlfwStatus::InvalidArgument
        }
    }

    pub unsafe fn get_pixel_size(
        window: GlfwWindow,
        width: *mut u32,
        height: *mut u32,
    ) -> GlfwStatus {
        if window.is_null() || width.is_null() || height.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        let mut pixel_width: i32 = 0;
        let mut pixel_height: i32 = 0;
        unsafe {
            glfwGetFramebufferSize(window, &mut pixel_width, &mut pixel_height);
            *width = pixel_width.max(0) as u32;
            *height = pixel_height.max(0) as u32;
        }
        GlfwStatus::Ok
    }

    pub unsafe fn get_logical_size(
        window: GlfwWindow,
        width: *mut u32,
        height: *mut u32,
    ) -> GlfwStatus {
        if window.is_null() || width.is_null() || height.is_null() {
            return GlfwStatus::InvalidArgument;
        }

        let mut logical_width: i32 = 0;
        let mut logical_height: i32 = 0;
        unsafe {
            glfwGetWindowSize(window, &mut logical_width, &mut logical_height);
            *width = logical_width.max(0) as u32;
            *height = logical_height.max(0) as u32;
        }
        GlfwStatus::Ok
    }

    pub unsafe fn poll_events() -> GlfwStatus {
        unsafe {
            glfwPollEvents();
        }
        GlfwStatus::Ok
    }

    pub unsafe fn wait_events() -> GlfwStatus {
        unsafe {
            glfwWaitEvents();
        }
        GlfwStatus::Ok
    }

    pub unsafe fn wait_events_timeout(timeout_ns: u64) -> GlfwStatus {
        let timeout_seconds = timeout_ns as f64 / 1_000_000_000.0;
        unsafe {
            glfwWaitEventsTimeout(timeout_seconds);
        }
        GlfwStatus::Ok
    }

    pub unsafe fn post_empty_event() -> GlfwStatus {
        unsafe {
            glfwPostEmptyEvent();
        }
        GlfwStatus::Ok
    }

    pub unsafe fn window_should_close(window: GlfwWindow) -> bool {
        if window.is_null() {
            return true;
        }
        unsafe { glfwWindowShouldClose(window) != GLFW_FALSE }
    }

    pub unsafe fn window_request_close(window: GlfwWindow) -> GlfwStatus {
        if window.is_null() {
            return GlfwStatus::InvalidArgument;
        }
        unsafe {
            glfwSetWindowShouldClose(window, GLFW_TRUE);
        }
        GlfwStatus::Ok
    }

    pub unsafe fn current_mods(window: GlfwWindow) -> GlfwMod {
        if window.is_null() {
            return GlfwMod(0);
        }

        let mut mods = 0u32;
        unsafe {
            if glfwGetKey(window, GlfwKey::LEFT_SHIFT) == GLFW_PRESS
                || glfwGetKey(window, GlfwKey::RIGHT_SHIFT) == GLFW_PRESS
            {
                mods |= GlfwMod::SHIFT;
            }
            if glfwGetKey(window, GlfwKey::LEFT_CONTROL) == GLFW_PRESS
                || glfwGetKey(window, GlfwKey::RIGHT_CONTROL) == GLFW_PRESS
            {
                mods |= GlfwMod::CTRL;
            }
            if glfwGetKey(window, GlfwKey::LEFT_ALT) == GLFW_PRESS
                || glfwGetKey(window, GlfwKey::RIGHT_ALT) == GLFW_PRESS
            {
                mods |= GlfwMod::ALT;
            }
            if glfwGetKey(window, GlfwKey::LEFT_SUPER) == GLFW_PRESS
                || glfwGetKey(window, GlfwKey::RIGHT_SUPER) == GLFW_PRESS
            {
                mods |= GlfwMod::SUPER;
            }
        }

        GlfwMod(mods)
    }

    impl GlfwMouseButton {
        pub fn from_glfw(button: i32) -> Self {
            match button {
                0 => Self(Self::LEFT),
                1 => Self(Self::RIGHT),
                2 => Self(Self::MIDDLE),
                3 => Self(Self::BACK),
                4 => Self(Self::FORWARD),
                _ => Self(Self::UNKNOWN),
            }
        }
    }
}
