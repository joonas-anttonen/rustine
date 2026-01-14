use crate::{mount, sysinfo};
use rustine::log;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Drives,
}

impl Tab {
    fn hotkey(&self) -> &'static str {
        match self {
            Tab::Drives => "F1",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Tab::Drives => "Drives",
        }
    }
}

struct MyApplicationState {
    frame_index: usize,
    selected_tab: Tab,
    devices: Vec<sysinfo::BlockDevice>,
    selected_index: Option<usize>,
    password_mode: bool,
    password_buffer: String,
    mounting_index: Option<usize>,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState {
                frame_index: 0,
                selected_tab: Tab::Drives,
                devices: Vec::new(),
                selected_index: None,
                password_mode: false,
                password_buffer: String::new(),
                mounting_index: None,
            }),
        }
    }
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
        if _key.key == rustine::gui::Key::UNKNOWN {
            log::warning!("Application::on_key: {:?} {:?}", _key.key, _key.action);
        }

        if _key.action != rustine::gui::Action::PRESS {
            return;
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();

        let mut state = self.state.borrow_mut();
        let partition_count = state.devices.iter().filter(|d| d.is_partition).count();

        match _key.key {
            rustine::gui::Key::F1 => {
                state.selected_tab = Tab::Drives;
            }
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
                            let dispatch_cmd =
                                format!("[float] alacritty --command yazi {}", mount_path);
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
                                match mount::umount_with_sudo(
                                    &mount_info.mount_point,
                                    &state.password_buffer,
                                ) {
                                    Ok(_) => {
                                        log::info!(
                                            "Successfully unmounted {} from {}",
                                            device.path,
                                            mount_info.mount_point
                                        );
                                        // Rescan drives after unmount
                                        match sysinfo::get_block_devices() {
                                            Ok(devices) => {
                                                state.devices = devices
                                                    .into_iter()
                                                    .filter(|d| d.is_partition)
                                                    .collect();
                                            }
                                            Err(e) => {
                                                log::error!("Failed to rescan block devices: {}", e)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to unmount {}: {}", device.path, e)
                                    }
                                }
                            } else {
                                // Mount
                                let drive_name = device.path.split('/').last().unwrap_or("drive");
                                let mount_path = format!("/media/{}", drive_name);
                                let fs_type = device
                                    .fs_type
                                    .as_ref()
                                    .map(|s| s.as_str())
                                    .unwrap_or("auto");

                                match mount::mount_with_sudo(
                                    &device.path,
                                    &mount_path,
                                    fs_type,
                                    &state.password_buffer,
                                ) {
                                    Ok(_) => {
                                        log::info!(
                                            "Successfully mounted {} at {}",
                                            device.path,
                                            mount_path
                                        );
                                        // Rescan drives after mount
                                        match sysinfo::get_block_devices() {
                                            Ok(devices) => {
                                                state.devices = devices
                                                    .into_iter()
                                                    .filter(|d| d.is_partition)
                                                    .collect();
                                            }
                                            Err(e) => {
                                                log::error!("Failed to rescan block devices: {}", e)
                                            }
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

    fn on_char(&self, gui: &rustine::gui::Gui, c: char) {
        // Assume the window is damaged after handling a character input
        gui.mark_damaged();

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

        const TOP_BAR_HEIGHT: f32 = 32.0;
        let line_height: f32 = text_font_metrics.ascender - text_font_metrics.descender;

        const STATUS_LIGHT_WIDTH: f32 = 8.0;
        let status_light_height: f32 = line_height - 2.0 - 2.0;
        const STATUS_LIGHT_MARGIN: f32 = 10.0;
        const TEXT_START_X: f32 = STATUS_LIGHT_MARGIN + STATUS_LIGHT_WIDTH + 8.0;

        let content_x = 0.0;
        let content_y = TOP_BAR_HEIGHT;
        let content_w = w;
        let _content_h = h - TOP_BAR_HEIGHT;

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

        // Draw tab hotkey indicator and title in top bar
        let tab_hotkey = state.selected_tab.hotkey();
        let tab_title = state.selected_tab.title();

        //let hotkey_bg_color = 0x388BFD_FFu32;
        let hotkey_selected_bg_color = 0x3FB950_FFu32;
        let hotkey_text_color = 0x0D1117_FFu32;

        // Measure hotkey text width for background box
        let hotkey_padding = 6.0;
        let hotkey_height = line_height;
        let hotkey_width = (tab_hotkey.len() as f32
            * rustine::gfx::fonts::get_font_size(rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID)
            / 2.0)
            + (hotkey_padding * 2.0);

        let hotkey_x = content_x + 10.0;
        let hotkey_y = (TOP_BAR_HEIGHT - hotkey_height) / 2.0;

        // Draw hotkey background
        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: hotkey_x,
                y: hotkey_y,
                w: hotkey_width,
                h: hotkey_height,
            },
            hotkey_selected_bg_color,
        );

        // Draw hotkey text
        frame.push_text(
            tab_hotkey,
            hotkey_x + hotkey_padding,
            (TOP_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
            1.0,
            hotkey_text_color,
            rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );

        // Draw tab title
        frame.push_text(
            tab_title,
            hotkey_x + hotkey_width + 10.0,
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
                        w: content_w,
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
                        w: content_w,
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
