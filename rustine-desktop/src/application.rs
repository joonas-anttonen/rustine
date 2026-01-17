use crate::{files, list, mount, preview, sysinfo};
use rustine::{ConcurrentMailbox, gfx, gui::Key, log};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Drives,
    Files,
}

impl Tab {
    fn hotkey(&self) -> &'static str {
        match self {
            Tab::Drives => "F1",
            Tab::Files => "F2",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Tab::Drives => "Drives",
            Tab::Files => "Files",
        }
    }
}

struct MyApplicationState {
    frame_index: usize,
    selected_tab: Tab,
    devices: Vec<sysinfo::BlockDevice>,
    drives_list: list::ListState,
    files_entries: Vec<files::EntryInfo>,
    files_dir: PathBuf,
    files_list: list::ListState,
    show_hidden_files: bool,
    folder_selection_map: HashMap<PathBuf, String>,
    password_mode: bool,
    password_buffer: String,
    mounting_index: Option<usize>,
    files_preview_open: bool,
    files_preview_valid: bool,
    files_preview_entry: Option<files::EntryInfo>,
    preview_image: gfx::Image,
    preview_request_queue: Arc<ConcurrentMailbox<preview::PreviewRequest>>,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
    exit_flag: Arc<AtomicBool>,
    preview_request_flag: Arc<AtomicBool>,
}

impl MyApplication {
    pub fn new() -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState {
                frame_index: 0,
                selected_tab: Tab::Drives,
                devices: Vec::new(),
                drives_list: list::ListState::new(),
                files_entries: Vec::new(),
                files_dir: PathBuf::from("/"),
                files_list: list::ListState::new(),
                show_hidden_files: false,
                folder_selection_map: HashMap::new(),
                password_mode: false,
                password_buffer: String::new(),
                mounting_index: None,
                files_preview_open: false,
                files_preview_valid: false,
                files_preview_entry: None,
                preview_image: gfx::Image::default(),
                preview_request_queue: ConcurrentMailbox::new(),
            }),
            exit_flag: Arc::new(AtomicBool::new(false)),
            preview_request_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    fn update_preview(&self, state: &mut std::cell::RefMut<'_, MyApplicationState>) {
        if state.files_preview_open {
            if let Some(idx) = state.files_list.selected
                && let Some(entry) = state.files_entries.get(idx)
            {
                let entry_path = entry.path.clone();
                state.files_preview_entry = Some(entry.clone());

                if preview::can_preview(&entry_path) {
                    state.files_preview_valid = true;
                    state
                        .preview_request_queue
                        .push(preview::PreviewRequest::Load(entry_path));
                    self.preview_request_flag
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                } else {
                    state.files_preview_valid = false;
                    state
                        .preview_request_queue
                        .push(preview::PreviewRequest::Clear);
                    self.preview_request_flag
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        } else {
            state.files_preview_valid = false;
            // Clear preview when closing
            state
                .preview_request_queue
                .push(preview::PreviewRequest::Clear);
            self.preview_request_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.preview_request_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl rustine::gui::Application for MyApplication {
    fn startup(&self, gui: &rustine::gui::Gui) {
        log::debug!("Application::startup");

        let mut state = self.state.borrow_mut();

        // Create the dynamic image for preview
        state.preview_image = gui.create_dynamic_image();

        // Spawn the preview worker thread
        let image_mailbox = gui.image_mailbox();
        let request_queue = Arc::clone(&state.preview_request_queue);
        let preview_image_id = state.preview_image.id;
        let exit_flag = Arc::clone(&self.exit_flag);
        let preview_request_flag = Arc::clone(&self.preview_request_flag);

        std::thread::spawn(move || {
            preview::preview_worker_thread(
                request_queue,
                image_mailbox,
                preview_image_id,
                &exit_flag,
                &preview_request_flag,
            );
        });

        // Enumerate all block devices
        match sysinfo::get_block_devices() {
            Ok(devices) => {
                state.devices = devices.into_iter().filter(|d| d.is_partition).collect();
                state.drives_list.selected = if state.devices.is_empty() {
                    None
                } else {
                    Some(0)
                };
                state.password_mode = false;
                state.password_buffer = String::new();
                state.mounting_index = None;
            }
            Err(e) => log::error!("Failed to get block devices: {}", e),
        }

        match files::get_contents(&state.files_dir, state.show_hidden_files) {
            Ok(entries) => {
                state.files_entries = entries;
                state.files_list.selected = if state.files_entries.is_empty() {
                    None
                } else {
                    if let Some(last_name) = state.folder_selection_map.get(&state.files_dir) {
                        state
                            .files_entries
                            .iter()
                            .position(|e| &e.name == last_name)
                    } else {
                        Some(0)
                    }
                };
            }
            Err(e) => log::error!("Failed to get directory contents: {}", e),
        }
    }

    fn on_key(&self, gui: &rustine::gui::Gui, key: rustine::gui::KeyEvent) {
        if key.key == Key::UNKNOWN {
            log::warning!("Application::on_key: {:?} {:?}", key.key, key.action);
        }

        if key.action != rustine::gui::Action::PRESS {
            return;
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();

        let mut state = self.state.borrow_mut();

        match key.key {
            Key::F1 => {
                state.selected_tab = Tab::Drives;
                if state.drives_list.selected.is_none() && !state.devices.is_empty() {
                    state.drives_list.selected = Some(0);
                }
            }
            Key::F2 => {
                state.selected_tab = Tab::Files;
                if state.files_list.selected.is_none() && !state.files_entries.is_empty() {
                    state.files_list.selected = Some(0);
                }
            }
            Key::UP => {
                let drives_len = state.devices.len();
                let files_len = state.files_entries.len();
                match state.selected_tab {
                    Tab::Drives => state.drives_list.select_prev(drives_len),
                    Tab::Files => {
                        state.files_list.select_prev(files_len);
                        // Update preview if open
                        self.update_preview(&mut state);
                    }
                }
            }
            Key::DOWN => {
                let drives_len = state.devices.len();
                let files_len = state.files_entries.len();
                match state.selected_tab {
                    Tab::Drives => state.drives_list.select_next(drives_len),
                    Tab::Files => {
                        state.files_list.select_next(files_len);
                        // Update preview if open
                        self.update_preview(&mut state);
                    }
                }
            }
            Key::RIGHT if state.selected_tab == Tab::Files => {
                if let Some(idx) = state.files_list.selected {
                    let target_path = if let Some(entry) = state.files_entries.get(idx) {
                        let is_dir = matches!(entry.entry_type, files::EntryType::Directory)
                            || (entry.entry_type == files::EntryType::Symlink
                                && entry.path.is_dir());

                        if is_dir {
                            // Save current selection
                            let entry_name = entry.name.clone();
                            let current_dir = state.files_dir.clone();
                            let target = entry.path.clone();
                            state.folder_selection_map.insert(current_dir, entry_name);
                            Some(target)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(target) = target_path {
                        match files::get_contents(&target, state.show_hidden_files) {
                            Ok(entries) => {
                                state.files_dir = target.clone();
                                state.files_entries = entries;
                                state.files_list.selected = if state.files_entries.is_empty() {
                                    None
                                } else if let Some(last_name) =
                                    state.folder_selection_map.get(&target)
                                {
                                    // Try to restore the previously selected item
                                    state
                                        .files_entries
                                        .iter()
                                        .position(|e| &e.name == last_name)
                                } else {
                                    Some(0)
                                };
                            }
                            Err(e) => {
                                log::error!("Failed to enter directory {}: {}", target.display(), e)
                            }
                        }
                    }
                }

                self.update_preview(&mut state);
            }
            Key::RIGHT => {}
            Key::LEFT if state.selected_tab == Tab::Files => {
                // Save current selection before leaving
                if let Some(idx) = state.files_list.selected
                    && let Some(entry) = state.files_entries.get(idx)
                {
                    let entry_name = entry.name.clone();
                    let current_dir = state.files_dir.clone();
                    state.folder_selection_map.insert(current_dir, entry_name);
                }

                let parent = state
                    .files_dir
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| state.files_dir.clone());

                match files::get_contents(&parent, state.show_hidden_files) {
                    Ok(entries) => {
                        state.files_dir = parent.clone();
                        state.files_entries = entries;
                        state.files_list.selected = if state.files_entries.is_empty() {
                            None
                        } else if let Some(last_name) = state.folder_selection_map.get(&parent) {
                            // Try to restore the previously selected item
                            state
                                .files_entries
                                .iter()
                                .position(|e| &e.name == last_name)
                        } else {
                            Some(0)
                        };
                    }
                    Err(e) => log::error!("Failed to go up to {}: {}", parent.display(), e),
                }

                self.update_preview(&mut state);
            }
            Key::LEFT => {}
            Key::PERIOD if state.selected_tab == Tab::Files => {
                state.show_hidden_files = !state.show_hidden_files;

                match files::get_contents(&state.files_dir, state.show_hidden_files) {
                    Ok(entries) => {
                        state.files_entries = entries;
                        let len = state.files_entries.len();

                        if len == 0 {
                            state.files_list.selected = None;
                        } else if let Some(last_name) =
                            state.folder_selection_map.get(&state.files_dir)
                        {
                            // Try to find and select the previously saved file
                            state.files_list.selected = state
                                .files_entries
                                .iter()
                                .position(|e| &e.name == last_name);
                        } else {
                            // Otherwise try to keep roughly the same position
                            let previous = state.files_list.selected.unwrap_or(0);
                            state.files_list.selected = Some(previous.min(len - 1));
                        }
                    }
                    Err(e) => log::error!("Failed to get directory contents: {}", e),
                }
            }
            Key::PERIOD => {}
            Key::TAB if state.selected_tab == Tab::Files => {
                state.files_preview_open = !state.files_preview_open;
                // Update preview entry based on current selection
                self.update_preview(&mut state);
            }
            Key::TAB => {}
            Key::E if state.selected_tab == Tab::Drives => {
                if let Some(idx) = state.drives_list.selected
                    && let Some(device) = state.devices.get(idx)
                    && let Some(mount_info) = &device.mount_info
                {
                    let mount_path = PathBuf::from(&mount_info.mount_point);

                    // Switch to Files tab and navigate to the mounted folder
                    state.selected_tab = Tab::Files;
                    match files::get_contents(&mount_path, state.show_hidden_files) {
                        Ok(entries) => {
                            state.files_dir = mount_path.clone();
                            state.files_entries = entries;
                            state.files_list.selected = if state.files_entries.is_empty() {
                                None
                            } else {
                                Some(0)
                            };
                        }
                        Err(e) => {
                            log::error!("Failed to read directory {}: {}", mount_path.display(), e)
                        }
                    }
                }
            }
            Key::M if state.selected_tab == Tab::Drives => {
                if let Some(idx) = state.drives_list.selected
                    && state.devices.get(idx).is_some()
                {
                    // Enter password mode for both mount and unmount
                    state.password_mode = true;
                    state.password_buffer.clear();
                    state.mounting_index = Some(idx);
                }
            }
            Key::M => {}
            Key::ENTER if state.password_mode => {
                // Execute mount or unmount with sudo
                toggle_mount(&mut state);

                // Clear password from memory
                state.password_buffer.clear();
                state.password_mode = false;
                state.mounting_index = None;
            }
            Key::ENTER => {}
            Key::ESCAPE if state.password_mode => {
                // Cancel password entry
                state.password_buffer.clear();
                state.password_mode = false;
                state.mounting_index = None;
            }
            Key::ESCAPE => {
                gui.request_quit();
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

    fn render(&self, _gui: &rustine::gui::Gui, frame: &mut gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();

        let w = frame.size.x as f32;
        let h = frame.size.y as f32;

        let text_font_metrics = gfx::fonts::get_font_metrics(gfx::fonts::CASKAYDIAMONO_FONT_ID)
            .expect("Font metrics exist");

        const TOP_BAR_HEIGHT: f32 = 32.0;
        let line_height: f32 = text_font_metrics.ascender - text_font_metrics.descender;

        const STATUS_LIGHT_WIDTH: f32 = 8.0;
        let status_light_height: f32 = line_height - 2.0 - 2.0;
        const STATUS_LIGHT_MARGIN: f32 = 10.0;
        const TEXT_START_X: f32 = STATUS_LIGHT_MARGIN + STATUS_LIGHT_WIDTH + 8.0;
        const STATUS_BAR_HEIGHT: f32 = 28.0;

        let content_x = 0.0;
        let content_y = TOP_BAR_HEIGHT;
        let content_w = w;
        let content_h = h - TOP_BAR_HEIGHT - STATUS_BAR_HEIGHT;

        let bar_color = 0x1B232F_FFu32;
        let bg_color = 0x1B232F_FFu32;
        let text_color = 0xFFFFFF_FFu32;
        let mounted_color = 0x3FB950_FFu32;
        let unmounted_color = 0x79C0FF_FFu32;

        // Fill background
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            bg_color,
        );

        // Draw top bar
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w,
                h: TOP_BAR_HEIGHT,
            },
            bar_color,
        );

        // Draw tab hotkey indicators and titles in top bar
        let hotkey_selected_bg_color = 0x3FB950_FFu32;
        let hotkey_inactive_bg_color = 0x21262D_FFu32;
        let hotkey_text_color = 0x0D1117_FFu32;
        let tabs = [Tab::Drives, Tab::Files];
        let hotkey_padding = 6.0;
        let hotkey_height = line_height;
        let mut tab_x = content_x + 10.0;

        for tab in tabs.iter() {
            let tab_hotkey = tab.hotkey();
            let tab_title = tab.title();
            let is_selected_tab = *tab == state.selected_tab;
            let hotkey_width = (tab_hotkey.len() as f32
                * gfx::fonts::get_font_size(gfx::fonts::CASKAYDIAMONO_FONT_ID)
                / 2.0)
                + (hotkey_padding * 2.0);
            let title_width = tab_title.len() as f32
                * gfx::fonts::get_font_size(gfx::fonts::CASKAYDIAMONO_FONT_ID)
                / 2.0;

            let hotkey_y = (TOP_BAR_HEIGHT - hotkey_height) / 2.0;
            let hotkey_bg = if is_selected_tab {
                hotkey_selected_bg_color
            } else {
                hotkey_inactive_bg_color
            };

            frame.fill_rectangle(
                &gfx::Rectangle {
                    x: tab_x,
                    y: hotkey_y,
                    w: hotkey_width,
                    h: hotkey_height,
                },
                hotkey_bg,
            );

            frame.push_text(
                tab_hotkey,
                tab_x + hotkey_padding,
                (TOP_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
                1.0,
                hotkey_text_color,
                gfx::fonts::CASKAYDIAMONO_FONT_ID,
            );

            frame.push_text(
                tab_title,
                tab_x + hotkey_width + 10.0,
                (TOP_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
                1.0,
                text_color,
                gfx::fonts::CASKAYDIAMONO_FONT_ID,
            );

            tab_x += hotkey_width + title_width + 28.0;
        }

        let highlight_color = 0x21262D_FFu32;

        // Update scroll offsets to keep selected items in view
        state.drives_list.update_scroll(line_height, content_h);
        state.files_list.update_scroll(line_height, content_h);

        match state.selected_tab {
            Tab::Drives => {
                list::render_list(
                    frame,
                    content_x,
                    content_w,
                    content_y,
                    content_h,
                    line_height,
                    highlight_color,
                    state.drives_list.selected,
                    state.devices.len(),
                    state.drives_list.scroll_offset,
                    |frame, index, y, _is_selected| {
                        if let Some(device) = state.devices.get(index) {
                            let size_str = rustine::utilities::format_bytes_iec(
                                device.size.unwrap_or(0) as usize,
                            );

                            let (status_color, drive_text) = if let Some(mount_info) =
                                &device.mount_info
                            {
                                let text = format!(
                                    "{} ({}) -> {} ({})",
                                    device.path,
                                    size_str,
                                    mount_info.mount_point,
                                    mount_info.fs_type
                                );
                                (mounted_color, text)
                            } else {
                                let fs_str = device
                                    .fs_type
                                    .as_ref()
                                    .map(|s| s.as_str())
                                    .unwrap_or("unknown");
                                let text = format!("{} ({}) ({})", device.path, size_str, fs_str);
                                (unmounted_color, text)
                            };

                            frame.fill_rectangle(
                                &gfx::Rectangle {
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
                                gfx::fonts::CASKAYDIAMONO_FONT_ID,
                            );
                        }
                    },
                );

                // Draw password entry prompt if in password mode
                if state.password_mode
                    && let Some(mounting_idx) = state.mounting_index
                {
                    let prompt_y = content_y + (mounting_idx as f32 * line_height)
                        - state.drives_list.scroll_offset;
                    let prompt_bg_color = 0x0D1117_EEu32;

                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: content_x,
                            y: prompt_y,
                            w: content_w,
                            h: line_height,
                        },
                        prompt_bg_color,
                    );

                    let prompt_text = "[sudo] password: ";
                    frame.push_text(
                        prompt_text,
                        content_x + TEXT_START_X,
                        prompt_y + text_font_metrics.ascender,
                        1.0,
                        text_color,
                        gfx::fonts::CASKAYDIAMONO_FONT_ID,
                    );
                }
            }
            Tab::Files => {
                // Calculate layout based on preview panel state
                let preview_width = if state.files_preview_open {
                    content_w / 2.0
                } else {
                    0.0
                };
                let list_width = content_w - preview_width;

                // Render the file list
                list::render_list(
                    frame,
                    content_x,
                    list_width,
                    content_y,
                    content_h,
                    line_height,
                    highlight_color,
                    state.files_list.selected,
                    state.files_entries.len(),
                    state.files_list.scroll_offset,
                    |frame, index, y, _is_selected| {
                        if let Some(entry) = state.files_entries.get(index) {
                            let status_color = match entry.entry_type {
                                files::EntryType::Directory => unmounted_color,
                                files::EntryType::File => text_color,
                                files::EntryType::Symlink => mounted_color,
                                files::EntryType::Other => 0x8B949E_FFu32,
                            };

                            frame.fill_rectangle(
                                &gfx::Rectangle {
                                    x: content_x + STATUS_LIGHT_MARGIN,
                                    y: y + 2.0,
                                    w: STATUS_LIGHT_WIDTH,
                                    h: status_light_height,
                                },
                                status_color,
                            );

                            frame.push_text(
                                &entry.name,
                                content_x + TEXT_START_X,
                                y + text_font_metrics.ascender,
                                1.0,
                                text_color,
                                gfx::fonts::CASKAYDIAMONO_FONT_ID,
                            );
                        }
                    },
                );

                // Render the preview panel if open
                if state.files_preview_open {
                    let preview_x = content_x + list_width;
                    let preview_panel_bg_color = 0x0D1117_FFu32;
                    let preview_border_color = 0x30363D_FFu32;

                    // Draw preview panel background
                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: preview_x,
                            y: content_y,
                            w: preview_width,
                            h: content_h,
                        },
                        preview_panel_bg_color,
                    );

                    // Draw preview panel border (left edge)
                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: preview_x,
                            y: content_y,
                            w: 1.0,
                            h: content_h,
                        },
                        preview_border_color,
                    );

                    // Display preview content
                    if state.files_preview_entry.is_some() && state.files_preview_valid {
                        let preview_padding = 0.0;

                        // Calculate image display area (below the title)
                        let image_area_y = line_height + preview_padding;
                        let image_area_w = preview_width - (preview_padding * 2.0);
                        let image_area_h = content_h - line_height - (preview_padding * 3.0);

                        // Always render the preview image - gfx will handle it if pixel buffer exists
                        frame.push_image(
                            &state.preview_image,
                            None,
                            gfx::Rectangle {
                                x: preview_x + preview_padding,
                                y: image_area_y,
                                w: image_area_w,
                                h: image_area_h,
                            },
                            gfx::Fit::FIT_KEEP_ASPECT,
                            0xFFFFFF_FFu32,
                        );
                    } else {
                        let preview_text = "No preview";
                        frame.push_text(
                            preview_text,
                            preview_x + 10.0,
                            content_y + 20.0,
                            1.0,
                            0x6E7681_FFu32,
                            gfx::fonts::CASKAYDIAMONO_FONT_ID,
                        );
                    }
                }
            }
        }

        // Status bar at the bottom
        let status_bar_color = 0x161B22_FFu32;
        let status_text = match state.selected_tab {
            Tab::Files => format!("{} | TAB toggle preview", state.files_dir.display()),
            Tab::Drives => "↑/↓ select | M mount/unmount | E open".to_string(),
        };

        let status_y = h - STATUS_BAR_HEIGHT;
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: 0.0,
                y: status_y,
                w,
                h: STATUS_BAR_HEIGHT,
            },
            status_bar_color,
        );

        frame.push_text(
            &status_text,
            content_x + 10.0,
            status_y + (STATUS_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
            1.0,
            text_color,
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );

        state.frame_index += 1;
    }
}

fn toggle_mount(state: &mut std::cell::RefMut<'_, MyApplicationState>) {
    let Some(idx) = state.mounting_index else {
        return;
    };
    let Some(device) = state.devices.get(idx) else {
        return;
    };

    if let Some(mount_info) = &device.mount_info {
        // Unmount
        match mount::umount_with_sudo(&mount_info.mount_point, &state.password_buffer) {
            Ok(_) => {
                log::info!("Unmounted {} from {}", device.path, mount_info.mount_point);
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

        match mount::mount_with_sudo(&device.path, &mount_path, fs_type, &state.password_buffer) {
            Ok(_) => {
                log::info!("Mounted {} at {}", device.path, mount_path);
            }
            Err(e) => log::error!("Failed to mount {}: {}", device.path, e),
        }
    }

    // Rescan drives after
    match sysinfo::get_block_devices() {
        Ok(devices) => {
            state.devices = devices.into_iter().filter(|d| d.is_partition).collect();
            let drives_len = state.devices.len();
            state.drives_list.clamp(drives_len);
        }
        Err(e) => {
            log::error!("Failed to rescan block devices: {}", e)
        }
    }
}
