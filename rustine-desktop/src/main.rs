use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log};
use rustine_desktop::sysinfo;

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

extern "C" fn handle_sigterm(_signal: i32) {
    log::info!("SIGTERM");
    SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
    rustine::gui::Gui::wake_up();
}
fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_sigterm as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            let os_error = std::io::Error::last_os_error();
            eprintln!("Failed to install SIGINT handler: {os_error:?}",);
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            let os_error = std::io::Error::last_os_error();
            eprintln!("Failed to install SIGTERM handler: {os_error:?}",);
        }
    }
}

struct Arguments {}
fn parse_arguments() -> Option<Arguments> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 {
        None
    } else {
        Some(Arguments {})
    }
}

fn main() -> std::process::ExitCode {
    install_signal_handlers();

    let args = parse_arguments();
    if args.is_none() {
        eprintln!("Usage: {}", "");
        return std::process::ExitCode::from(1);
    }

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    log::debug!("STARTUP");

    let application = Box::new(MyApplication {
        state: std::cell::RefCell::new(MyApplicationState {
            frame_index: 0,
            devices: Vec::new(),
            selected_index: None,
            password_mode: false,
            password_buffer: String::new(),
            mounting_index: None,
        }),
    });

    {
        let gfx_builder = rustine::gfx::Gfx::builder(rustine::Platform::Wayland)
            .app_name("rustine-desktop")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-desktop")
            .window_size(1280, 720)
            .window_type(rustine::gui::WindowType::Normal);

        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));
        let gui = gui_builder.build(gfx.clone(), application);
        let mode = rustine::gfx::LoopMode::Continuous;

        std::thread::scope(|scope| {
            scope.spawn(|| {
                rustine::gfx::Gfx::run(gfx.clone(), &SHUTDOWN_FLAG, mode);
            });

            rustine::gui::Gui::run(&gui, &SHUTDOWN_FLAG, mode);

            SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
            gfx.lock().unwrap().wake_up();
        });
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}

struct MyApplicationState {
    frame_index: usize,
    devices: Vec<sysinfo::BlockDevice>,
    selected_index: Option<usize>,
    password_mode: bool,
    password_buffer: String,
    mounting_index: Option<usize>,
}

struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl rustine::gui::Application for MyApplication {
    fn startup(&self, _gui: &rustine::gui::Gui) {
        log::debug!("Application::startup");

        let mut state = self.state.borrow_mut();

        // Enumerate all block devices
        match sysinfo::get_block_devices() {
            Ok(devices) => {
                state.devices = devices.into_iter().filter(|d| d.is_partition).collect();
                state.selected_index = Some(0);
                state.password_mode = false;
                state.password_buffer = String::new();
                state.mounting_index = None;
            }
            Err(e) => log::error!("Failed to get block devices: {}", e),
        }
    }

    fn on_key(&self, gui: &rustine::gui::Gui, _key: rustine::gui::KeyEvent) {
        log::debug!("Application::on_key: {:?} {:?}", _key.key, _key.action);

        if _key.action != rustine::gui::Action::PRESS {
            return;
        }

        let mut state = self.state.borrow_mut();
        let partition_count = state.devices.iter().filter(|d| d.is_partition).count();

        match _key.key {
            rustine::gui::Key::UP => {
                if partition_count > 0 {
                    state.selected_index = Some(match state.selected_index {
                        Some(idx) if idx > 0 => idx - 1,
                        Some(_) => partition_count - 1, // Wrap to bottom
                        None => 0,
                    });
                }
            }
            rustine::gui::Key::DOWN => {
                if partition_count > 0 {
                    state.selected_index = Some(match state.selected_index {
                        Some(idx) if idx < partition_count - 1 => idx + 1,
                        Some(_) => 0, // Wrap to top
                        None => 0,
                    });
                }
            }
            rustine::gui::Key::E => {
                if let Some(idx) = state.selected_index {
                    if let Some(device) = state.devices.get(idx) {
                        if let Some(mount_info) = &device.mount_info {
                            let mount_path = &mount_info.mount_point;
                            let dispatch_cmd = format!("[float] alacritty --command yazi {}", mount_path);
                            match std::process::Command::new("hyprctl")
                                .arg("dispatch")
                                .arg("exec")
                                .arg(dispatch_cmd)
                                .spawn()
                            {
                                Ok(_) => log::info!("Launched yazi at {}", mount_path),
                                Err(e) => log::error!("Failed to launch yazi: {}", e),
                            }
                        }
                    }
                }
            }
            rustine::gui::Key::M => {
                if let Some(idx) = state.selected_index {
                    if state.devices.get(idx).is_some() {
                        // Enter password mode for both mount and unmount
                        state.password_mode = true;
                        state.password_buffer.clear();
                        state.mounting_index = Some(idx);
                    }
                }
            }
            rustine::gui::Key::ENTER => {
                if state.password_mode {
                    // Execute mount or unmount with sudo
                    if let Some(idx) = state.mounting_index {
                        if let Some(device) = state.devices.get(idx) {
                            if let Some(mount_info) = &device.mount_info {
                                // Unmount
                                match rustine_desktop::mount::umount_with_sudo(&mount_info.mount_point, &state.password_buffer) {
                                    Ok(_) => {
                                        log::info!("Successfully unmounted {} from {}", device.path, mount_info.mount_point);
                                        // Rescan drives after unmount
                                        match rustine_desktop::sysinfo::get_block_devices() {
                                            Ok(devices) => {
                                                state.devices = devices.into_iter().filter(|d| d.is_partition).collect();
                                            }
                                            Err(e) => log::error!("Failed to rescan block devices: {}", e),
                                        }
                                    }
                                    Err(e) => log::error!("Failed to unmount {}: {}", device.path, e),
                                }
                            } else {
                                // Mount
                                let drive_name = device.path.split('/').last().unwrap_or("drive");
                                let mount_path = format!("/media/{}", drive_name);
                                let fs_type = device.fs_type.as_ref().map(|s| s.as_str()).unwrap_or("auto");

                                match rustine_desktop::mount::mount_with_sudo(&device.path, &mount_path, fs_type, &state.password_buffer) {
                                    Ok(_) => {
                                        log::info!("Successfully mounted {} at {}", device.path, mount_path);
                                        // Rescan drives after mount
                                        match rustine_desktop::sysinfo::get_block_devices() {
                                            Ok(devices) => {
                                                state.devices = devices.into_iter().filter(|d| d.is_partition).collect();
                                            }
                                            Err(e) => log::error!("Failed to rescan block devices: {}", e),
                                        }
                                    }
                                    Err(e) => log::error!("Failed to mount {}: {}", device.path, e),
                                }
                            }
                        }
                    }

                    // Clear password from memory
                    state.password_buffer.clear();
                    state.password_mode = false;
                    state.mounting_index = None;
                }
            }
            rustine::gui::Key::ESCAPE => {
                if state.password_mode {
                    // Cancel password entry
                    state.password_buffer.clear();
                    state.password_mode = false;
                    state.mounting_index = None;
                } else {
                    gui.request_quit();
                }
            }
            _ => {}
        }
    }

    fn on_char(&self, _gui: &rustine::gui::Gui, c: char) {
        log::debug!("Application::on_char: U+{:04X} ('{}')", c as u32, c);
        
        let mut state = self.state.borrow_mut();
        if state.password_mode {
            // Collect password characters (no visual feedback)
            if c == '\x08' || c == '\x7f' {
                // Backspace
                state.password_buffer.pop();
            } else if c.is_ascii() && !c.is_control() {
                state.password_buffer.push(c);
            }
        }
    }

    fn render(&self, _gui: &rustine::gui::Gui, frame: &mut rustine::gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();

        let w = frame.size.x as f32;
        let h = frame.size.y as f32;

        let text_font_metrics =
            rustine::gfx::fonts::get_font_metrics(rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID)
                .expect("Font metrics exist");

        const TOP_BAR_HEIGHT: f32 = 48.0;
        const SIDE_BAR_WIDTH: f32 = 48.0;
        let line_height: f32 = text_font_metrics.ascender - text_font_metrics.descender;

        const STATUS_LIGHT_WIDTH: f32 = 8.0;
        let status_light_height: f32 = line_height - 2.0 - 2.0;
        const STATUS_LIGHT_MARGIN: f32 = 10.0;
        const TEXT_START_X: f32 = STATUS_LIGHT_MARGIN + STATUS_LIGHT_WIDTH + 8.0;

        let content_x = SIDE_BAR_WIDTH;
        let content_y = TOP_BAR_HEIGHT;
        let _content_w = w - SIDE_BAR_WIDTH;
        let content_h = h - TOP_BAR_HEIGHT;

        let bar_color = 0x1B232F_FFu32;
        let bg_color = 0x0D1117_FFu32;
        let text_color = 0xFFFFFF_FFu32;
        let mounted_color = 0x3FB950_FFu32;
        let unmounted_color = 0x79C0FF_FFu32;

        // Fill background
        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            bg_color,
        );

        // Draw top bar
        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w,
                h: TOP_BAR_HEIGHT,
            },
            bar_color,
        );

        // Draw side bar
        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: 0.0,
                y: TOP_BAR_HEIGHT,
                w: SIDE_BAR_WIDTH,
                h: content_h,
            },
            bar_color,
        );

        // Title in top bar
        frame.push_text(
            "Drive Manager",
            content_x + 10.0,
            (TOP_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
            1.0,
            text_color,
            rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );

        let mut y = content_y;

        let highlight_color = 0x21262D_FFu32;

        // Draw all drives in a flat list (partitions only)
        for (index, device) in state.devices.iter().enumerate() {
            let is_selected = state.selected_index == Some(index);

            // Draw highlight background for selected item
            if is_selected {
                frame.fill_rectangle(
                    &rustine::gfx::Rectangle {
                        x: content_x,
                        y: y,
                        w: _content_w,
                        h: line_height,
                    },
                    highlight_color,
                );
            }

            let size_str = rustine::utilities::format_bytes_iec(device.size.unwrap_or(0) as usize);

            // Determine color and text based on mount status
            let (status_color, drive_text) = if let Some(mount_info) = &device.mount_info {
                // Mounted: green status light
                let text = format!(
                    "{} ({}) -> {} ({})",
                    device.path, size_str, mount_info.mount_point, mount_info.fs_type
                );
                (mounted_color, text)
            } else {
                // Unmounted: blue status light
                let fs_str = device
                    .fs_type
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let text = format!("{} ({}) ({})", device.path, size_str, fs_str);
                (unmounted_color, text)
            };

            // Draw status light
            frame.fill_rectangle(
                &rustine::gfx::Rectangle {
                    x: content_x + STATUS_LIGHT_MARGIN,
                    y: y + 2.0,
                    w: STATUS_LIGHT_WIDTH,
                    h: status_light_height,
                },
                status_color,
            );

            frame.push_text(
                &drive_text,
                content_x + TEXT_START_X,
                y + text_font_metrics.ascender,
                1.0,
                text_color,
                rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
            );
            y += line_height;
        }

        // Draw password entry prompt if in password mode
        if state.password_mode {
            if let Some(mounting_idx) = state.mounting_index {
                // Calculate Y position for the password prompt (on top of the mounting line)
                let prompt_y = content_y + (mounting_idx as f32 * line_height);
                
                // Draw semi-transparent background overlay for the prompt
                let prompt_bg_color = 0x0D1117_EEu32;
                frame.fill_rectangle(
                    &rustine::gfx::Rectangle {
                        x: content_x,
                        y: prompt_y,
                        w: _content_w,
                        h: line_height,
                    },
                    prompt_bg_color,
                );

                // Draw password prompt text
                let prompt_text = "[sudo] password: ";
                frame.push_text(
                    prompt_text,
                    content_x + TEXT_START_X,
                    prompt_y + text_font_metrics.ascender,
                    1.0,
                    text_color,
                    rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
                );
            }
        }

        state.frame_index += 1;
    }
}
