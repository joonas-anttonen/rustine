use rustine::{Platform, RunMode, Version, gfx, gui, log, lua::LuaEngine, lua_ui, scene::Scene, ui_dom};
use std::{
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

enum UiMode {
    Immediate,
    Dom,
}

struct LuaMechApplication {
    lua_engine: std::cell::RefCell<LuaEngine>,
    ui_script_path: std::path::PathBuf,
    mouse_state: std::cell::RefCell<MouseState>,
    ui_mode: std::cell::RefCell<UiMode>,
    dom: std::cell::RefCell<Option<ui_dom::UiDom>>,
    dom_dirty: std::cell::RefCell<bool>,
}

#[derive(Default)]
struct MouseState {
    x: f32,
    y: f32,
    left_down: bool,
    left_pressed_this_frame: bool,
    left_released_this_frame: bool,
}

impl LuaMechApplication {
    fn new() -> Self {
        let mut lua_engine = LuaEngine::new();
        lua_engine.register_log_function();
        lua_ui::register_ui_functions(&mut lua_engine);

        // Set Lua package path to include our lua directory
        let lua_dir = std::env::current_dir()
            .unwrap()
            .join("lua")
            .to_string_lossy()
            .to_string();

        lua_engine
            .execute(&format!(
                r#"
                package.path = package.path .. ";{}/?.lua"
                "#,
                lua_dir.replace("\\", "\\\\")
            ))
            .unwrap();

        // Load the game UI script
        let script_path = Path::new("lua/simple_demo.lua");
        if script_path.exists() {
            let script = fs::read_to_string(script_path).expect("Failed to read UI script");
            lua_engine
                .execute(&script)
                .expect("Failed to execute UI script");
            log::info!("Loaded UI from {:?}", script_path);
        } else {
            log::warning!("UI script not found, using fallback UI");
            // Fallback to simple demo UI
            lua_engine.execute(r#"
                function render_ui()
                    local w, h = ui.get_window_size()
                    ui.rect(0, 0, w, h, 0x0D1117FF)
                    ui.text("Rustine Mech - Lua UI Demo", 20, 40, 1.0, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
                    ui.text("Lua UI system is working!", 20, 70, 1.0, 0x79C0FFFF, FONT_CASKAYDIA_MONO)
                    ui.text("Press R to reload, Q to quit", 20, 100, 1.0, 0x8B949EFF, FONT_CASKAYDIA_MONO)
                end
            "#).unwrap();
        }

        Self {
            lua_engine: std::cell::RefCell::new(lua_engine),
            ui_script_path: script_path.to_path_buf(),
            mouse_state: std::cell::RefCell::new(MouseState::default()),
            ui_mode: std::cell::RefCell::new(UiMode::Immediate),
            dom: std::cell::RefCell::new(None),
            dom_dirty: std::cell::RefCell::new(true),
        }
    }

    fn reload_ui_script(&self) {
        if self.ui_script_path.exists() {
            let script = fs::read_to_string(&self.ui_script_path).unwrap();
            if let Err(e) = self.lua_engine.borrow_mut().execute(&script) {
                log::error!("Failed to reload UI script: {}", e);
            } else {
                log::info!("Reloaded UI script");
                *self.dom_dirty.borrow_mut() = true;
            }
        }
    }

    fn rebuild_dom_if_needed(&self, window_width: f32, window_height: f32) {
        if *self.dom_dirty.borrow() {
            log::info!("Rebuilding DOM...");
            let mut lua = self.lua_engine.borrow_mut();
            match lua_ui::build_dom_from_lua(&mut lua) {
                Ok(mut new_dom) => {
                    new_dom.compute_layout(window_width, window_height);
                    *self.dom.borrow_mut() = Some(new_dom);
                    *self.dom_dirty.borrow_mut() = false;
                    log::info!("DOM rebuilt successfully");
                }
                Err(e) => {
                    log::error!("Failed to build DOM: {}", e);
                }
            }
        }
    }
}

impl gui::Application for LuaMechApplication {
    fn startup(&self, _gui: &gui::Gui) {
        log::info!("LuaMechApplication::startup");
    }

    fn on_key(&self, gui: &gui::Gui, event: gui::KeyEvent) {
        use gui::{Action, Key};

        if event.action == Action::PRESS {
            match event.key {
                Key::Q | Key::ESCAPE => {
                    log::info!("Quit requested");
                    gui.request_quit();
                }
                Key::R => {
                    // Reload UI script
                    log::info!("Reloading UI script...");
                    self.reload_ui_script();
                    gui.mark_damaged();
                }
                Key::D => {
                    // Toggle DOM mode
                    let new_mode = match *self.ui_mode.borrow() {
                        UiMode::Immediate => {
                            log::info!("Switching to DOM mode");
                            *self.dom_dirty.borrow_mut() = true;
                            UiMode::Dom
                        }
                        UiMode::Dom => {
                            log::info!("Switching to Immediate mode");
                            UiMode::Immediate
                        }
                    };
                    *self.ui_mode.borrow_mut() = new_mode;
                    gui.mark_damaged();
                }
                _ => {}
            }
        }
    }

    fn render(&self, _gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let window_w = frame.size.x as f32;
        let window_h = frame.size.y as f32;

        // Update mouse state
        let mut mouse = self.mouse_state.borrow_mut();
        lua_ui::update_input_state(lua_ui::UiInputState {
            mouse_x: mouse.x,
            mouse_y: mouse.y,
            mouse_left_down: mouse.left_down,
            mouse_left_pressed: mouse.left_pressed_this_frame,
            mouse_left_released: mouse.left_released_this_frame,
        });
        drop(mouse);

        // Render based on mode
        match *self.ui_mode.borrow() {
            UiMode::Immediate => {
                // Set current frame for Lua drawing
                lua_ui::set_current_frame(frame);

                // Call Lua render function
                let mut lua = self.lua_engine.borrow_mut();
                if let Err(e) = lua.call_function("render_ui", &[]) {
                    log::error!("Error calling render_ui: {}", e);
                    // Render error message
                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: 0.0,
                            y: 0.0,
                            w: window_w,
                            h: window_h,
                        },
                        0x0D1117FF,
                    );
                    frame.push_text(
                        "Lua Error:",
                        20.0,
                        40.0,
                        1.0,
                        0xFF7B72FF,
                        gfx::fonts::CASKAYDIAMONO_FONT_ID,
                    );
                    frame.push_text(
                        &e,
                        20.0,
                        70.0,
                        1.0,
                        0xFFFFFFFF,
                        gfx::fonts::CASKAYDIAMONO_FONT_ID,
                    );
                }

                // Clear frame reference
                lua_ui::clear_current_frame();
            }
            UiMode::Dom => {
                // Rebuild DOM if needed
                self.rebuild_dom_if_needed(window_w, window_h);

                // Update DOM with mouse state
                if let Some(ref mut dom) = *self.dom.borrow_mut() {
                    let mouse = self.mouse_state.borrow();
                    dom.update_mouse(
                        mouse.x,
                        mouse.y,
                        mouse.left_down,
                        mouse.left_pressed_this_frame,
                    );
                    drop(mouse);

                    // Render DOM
                    dom.render(frame);

                    // Check for button clicks and call callbacks
                    if dom.was_clicked("test_button") {
                        log::info!("Test button clicked via DOM!");
                        // Call Lua callback if defined
                        drop(dom);
                        let mut lua = self.lua_engine.borrow_mut();
                        if let Err(e) = lua.call_function("on_test_click", &[]) {
                            log::warning!("Callback error: {}", e);
                        }
                    }
                } else {
                    // No DOM yet, show loading message
                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: 0.0,
                            y: 0.0,
                            w: window_w,
                            h: window_h,
                        },
                        0x0D1117FF,
                    );
                    frame.push_text(
                        "Building DOM...",
                        20.0,
                        40.0,
                        1.0,
                        0xFFFFFFFF,
                        gfx::fonts::CASKAYDIAMONO_FONT_ID,
                    );
                }
            }
        }

        // Reset per-frame mouse states
        let mut mouse = self.mouse_state.borrow_mut();
        mouse.left_pressed_this_frame = false;
        mouse.left_released_this_frame = false;
    }
}

fn main() {
    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));
    log::info!("Rustine Mech - Lua UI Demo");

    let exit_flag = Arc::new(AtomicBool::new(false));

    // Initialize graphics
    let gfx = gfx::Gfx::builder(Platform::Wayland)
        .app_name("Rustine Mech")
        .app_version(Version::new(0, 1, 0))
        .debugging(true)
        .build()
        .unwrap();

    let am_gfx = Arc::new(Mutex::new(gfx));

    // Create scene
    let scene = Scene::new(Arc::clone(&am_gfx));
    let am_scene = Arc::new(Mutex::new(scene));

    // Start scene thread
    let scene_exit_flag = Arc::clone(&exit_flag);
    let scene_thread_am_scene = Arc::clone(&am_scene);
    let scene_thread = thread::spawn(move || {
        rustine::scene::run(scene_thread_am_scene, &scene_exit_flag, RunMode::Event);
    });

    // Start GFX thread
    let gfx_exit_flag = Arc::clone(&exit_flag);
    let gfx_thread_am_gfx = Arc::clone(&am_gfx);
    let gfx_thread = thread::spawn(move || {
        rustine::gfx::run(gfx_thread_am_gfx, &gfx_exit_flag, RunMode::Event);
    });

    // Create GUI with Lua application
    let application = Box::new(LuaMechApplication::new());
    let gui = gui::Gui::builder(Platform::Wayland)
        .window_title("Rustine Mech - Lua UI")
        .window_size(1280, 720)
        .build(Arc::clone(&am_gfx), application);

    // Run GUI loop
    gui::run(&gui, &exit_flag, RunMode::Continuous);

    log::info!("Shutting down...");
    exit_flag.store(true, Ordering::Relaxed);

    // Wake up other threads
    {
        let scene = am_scene.lock().unwrap();
        scene.wake_up();
    }
    {
        let gfx = am_gfx.lock().unwrap();
        gfx.wake_up();
    }

    scene_thread.join().unwrap();
    gfx_thread.join().unwrap();

    log::info!("Goodbye!");
}
