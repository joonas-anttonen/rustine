use crate::{command, files, list, mount, preview, sysinfo};
use rustine::{
    AutoResetEvent, Mailbox, gfx,
    gui::{self, Key},
    log,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc, sync::atomic::AtomicBool};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Drives,
    Files,
}

struct MyApplicationState {
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
    files_preview_fit: gfx::Fit,
    files_preview_ratio: f32,
    files_preview_open: bool,
    files_preview_valid: bool,
    files_preview_entry: Option<files::EntryInfo>,
    preview_image: gfx::Image,
    preview_request_queue: Arc<Mailbox<preview::PreviewRequest>>,
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
                selected_tab: Tab::Files,
                devices: Vec::new(),
                drives_list: list::ListState::new(),
                files_entries: Vec::new(),
                files_dir: PathBuf::from("/home"), // TODO: Get current user's home directory
                files_list: list::ListState::new(),
                show_hidden_files: false,
                folder_selection_map: HashMap::new(),
                password_mode: false,
                password_buffer: String::new(),
                files_preview_fit: gfx::Fit::FILL_KEEP_ASPECT,
                files_preview_ratio: 0.5,
                files_preview_open: false,
                files_preview_valid: false,
                files_preview_entry: None,
                preview_image: gfx::Image::default(),
                preview_request_queue: Mailbox::new(Arc::new(AutoResetEvent::new())),
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

impl Default for MyApplication {
    fn default() -> Self {
        Self::new()
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
                state.drives_list.selected = None;
            }
            Err(e) => log::error!("Failed to get block devices: {}", e),
        }

        let target = state.files_dir.clone();
        update_files_entries(&mut state, target);
    }

    fn on_key(&self, gui: &rustine::gui::Gui, event: rustine::gui::KeyEvent) {
        let mut state = self.state.borrow_mut();

        // Skip key handling if in password mode and the key is not ENTER or ESCAPE
        if state.password_mode && (event.key != Key::ENTER && event.key != Key::ESCAPE) {
            return;
        }

        if event.key == Key::UNKNOWN {
            log::warning!("Application::on_key: {:?} {:?}", event.key, event.action);
        }

        if event.action != rustine::gui::Action::PRESS {
            return;
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();

        match event.key {
            Key::F1 => {
                state.selected_tab = Tab::Files;
                if state.files_list.selected.is_none() && !state.files_entries.is_empty() {
                    state.files_list.selected = Some(0);
                }
            }
            Key::F2 => {
                state.selected_tab = Tab::Drives;
                if state.drives_list.selected.is_none() && !state.devices.is_empty() {
                    state.drives_list.selected = Some(0);
                }
            }
            Key::UP => {
                let drives_len = state.devices.len();
                let files_len = state.files_entries.len();
                match state.selected_tab {
                    Tab::Drives => state.drives_list.select_prev(drives_len),
                    Tab::Files => {
                        state.files_list.select_prev(files_len);
                        cache_current_selection(&mut state);
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
                        cache_current_selection(&mut state);
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
                        update_files_entries(&mut state, target);
                    }
                }

                self.update_preview(&mut state);
            }
            Key::RIGHT => {}
            Key::LEFT if state.selected_tab == Tab::Files => {
                // Save current selection before leaving
                cache_current_selection(&mut state);

                let target = state
                    .files_dir
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| state.files_dir.clone());

                update_files_entries(&mut state, target);
                self.update_preview(&mut state);
            }
            Key::LEFT => {}
            Key::PERIOD if state.selected_tab == Tab::Files => {
                state.show_hidden_files = !state.show_hidden_files;

                cache_current_selection(&mut state);
                let target = state.files_dir.clone();

                update_files_entries(&mut state, target);
                self.update_preview(&mut state);
            }
            Key::PERIOD => {}
            Key::W if state.selected_tab == Tab::Files => {
                // Cycle preview fit mode
                state.files_preview_fit = match state.files_preview_fit {
                    gfx::Fit::FILL_KEEP_ASPECT => gfx::Fit::FIT_KEEP_ASPECT,
                    gfx::Fit::FIT_KEEP_ASPECT => gfx::Fit::FILL_KEEP_ASPECT,
                    _ => gfx::Fit::FILL_KEEP_ASPECT,
                };
            }
            Key::TAB if state.selected_tab == Tab::Files => {
                if event.mods.contains(gui::Mods::SHIFT) {
                    // Cycle preview ratio in 0.1 increments between 0.2 and 0.8
                    state.files_preview_ratio += 0.1;
                    if state.files_preview_ratio > 0.8 {
                        state.files_preview_ratio = 0.2;
                    }
                } else {
                    state.files_preview_open = !state.files_preview_open;
                    // Update preview entry based on current selection
                    self.update_preview(&mut state);
                }
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
                    update_files_entries(&mut state, mount_path);
                }
            }
            Key::M if state.selected_tab == Tab::Drives => {
                if let Some(idx) = state.drives_list.selected
                    && state.devices.get(idx).is_some()
                {
                    // Enter password mode for both mount and unmount
                    state.password_mode = true;
                    state.password_buffer.clear();
                    state.drives_list.selected = Some(idx);
                }
            }
            Key::M => {}
            Key::ENTER if state.password_mode => {
                // Execute mount or unmount with sudo
                toggle_mount(&mut state);

                // Clear password from memory
                state.password_buffer.clear();
                state.password_mode = false;
                state.drives_list.selected = None;
            }
            Key::ENTER => {
                if let Some(selected_entry) = &state
                    .files_list
                    .selected
                    .and_then(|idx| state.files_entries.get(idx))
                {
                    let path = &selected_entry.path;
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_ascii_lowercase());

                    const MEDIA_EXTS: [&str; 16] = [
                        "mp4", "mkv", "webm", "webp", "png", "jpg", "jpeg", "tiff", "gif", "heic",
                        "mov", "mp3", "wav", "ogg", "flac", "avi",
                    ];

                    let program = if let Some(e) = &ext {
                        if MEDIA_EXTS.contains(&e.as_str()) {
                            "firefox"
                        } else {
                            "code"
                        }
                    } else {
                        "xdg-open"
                    };

                    match command::spawn_command_with_path(program, &[], path) {
                        Ok(res) => match res {
                            command::CommandResult::Started(pid) => {
                                log::info!("Command started with PID: {}", pid);
                            }
                            command::CommandResult::Completed {
                                status,
                                stdout,
                                stderr,
                            } => {
                                if !status.success() {
                                    log::warning!("Command completed with status: {:?}", status);
                                    if !stdout.is_empty() {
                                        log::info!("stdout: {}", stdout);
                                    }
                                    if !stderr.is_empty() {
                                        log::warning!("stderr: {}", stderr);
                                    }
                                } else {
                                    log::info!("Command executed successfully");
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("Error running command: {}", e);
                        }
                    }
                }
            }
            Key::ESCAPE if state.password_mode => {
                // Cancel password entry
                state.password_buffer.clear();
                state.password_mode = false;
                state.drives_list.selected = None;
            }
            Key::Q => {
                gui.request_quit();
            }
            Key::DELETE => {
                if let Some(selected_entry) = &state
                    .files_list
                    .selected
                    .and_then(|idx| state.files_entries.get(idx))
                {
                    if selected_entry.entry_type != files::EntryType::File
                        && selected_entry.entry_type != files::EntryType::Directory
                    {
                        log::info!("Selected entry is not a file or directory");
                        return;
                    }

                    match files::delete_path(&selected_entry.path) {
                        Ok(res) => {
                            if let files::FileOpResult::Deleted(p) = res {
                                log::info!("Deleted: {}", p.display());

                                if let Some(mut idx) = state.files_list.selected {
                                    idx = idx.saturating_sub(1);
                                    if idx < state.files_entries.len() {
                                        state.files_list.selected = Some(idx);
                                        cache_current_selection(&mut state);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Error deleting: {}", e);
                        }
                    }

                    let target = state.files_dir.clone();
                    update_files_entries(&mut state, target);
                    self.update_preview(&mut state);
                }
            }
            Key::F12 => {
                if let Some(selected_entry) = &state
                    .files_list
                    .selected
                    .and_then(|idx| state.files_entries.get(idx))
                {
                    if selected_entry.entry_type != files::EntryType::File
                        && selected_entry.entry_type != files::EntryType::Directory
                    {
                        log::info!("Selected entry is not a file or directory");
                        return;
                    }

                    let path = if selected_entry.entry_type == files::EntryType::Directory {
                        &selected_entry.path
                    } else {
                        selected_entry.path.parent().unwrap_or(&selected_entry.path)
                    };

                    // Open terminal to current directory
                    match command::spawn_command_with_path(
                        "alacritty",
                        &["--working-directory"],
                        path,
                    ) {
                        Ok(res) => match res {
                            command::CommandResult::Started(pid) => {
                                log::info!("Command started with PID: {}", pid);
                            }
                            command::CommandResult::Completed {
                                status,
                                stdout,
                                stderr,
                            } => {
                                if !status.success() {
                                    log::warning!("Command completed with status: {:?}", status);
                                    if !stdout.is_empty() {
                                        log::info!("stdout: {}", stdout);
                                    }
                                    if !stderr.is_empty() {
                                        log::warning!("stderr: {}", stderr);
                                    }
                                } else {
                                    log::info!("Command executed successfully");
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("Error running command: {}", e);
                        }
                    }
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

    fn render(&self, _gui: &rustine::gui::Gui, frame: &mut gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();

        let window_w = frame.size.x as f32;
        let window_h = frame.size.y as f32;

        let text_font_metrics = gfx::fonts::get_font_metrics(gfx::fonts::CASKAYDIAMONO_FONT_ID)
            .expect("Font metrics exist");

        const TOP_BAR_HEIGHT: f32 = STATUS_BAR_HEIGHT;
        let line_height: f32 = text_font_metrics.ascender - text_font_metrics.descender;

        const STATUS_LIGHT_WIDTH: f32 = 8.0;
        let status_light_height: f32 = line_height - 2.0 - 2.0;
        const STATUS_LIGHT_MARGIN: f32 = 10.0;
        const TEXT_START_X: f32 = STATUS_LIGHT_MARGIN + STATUS_LIGHT_WIDTH + 8.0;
        const STATUS_BAR_HEIGHT: f32 = 28.0;

        let content_x = 0.0;
        let content_y = TOP_BAR_HEIGHT;
        let content_w = window_w;
        let content_h = window_h - TOP_BAR_HEIGHT;

        let bg_color = 0x1B232FFFu32;
        let text_color = 0xFFFFFFFFu32;
        let mounted_color = 0x3FB950FFu32;
        let unmounted_color = 0x79C0FFFFu32;

        // Fill background
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w: window_w,
                h: window_h,
            },
            bg_color,
        );

        let highlight_color = 0x21262DFFu32;

        // Update scroll offsets to keep selected items in view
        state.drives_list.update_scroll(line_height, content_h);
        state.drives_list.count = state.devices.len();
        state.files_list.update_scroll(line_height, content_h);
        state.files_list.count = state.files_entries.len();

        // Calculate layout based on preview panel state
        let preview_width = if state.files_preview_open && state.selected_tab == Tab::Files {
            content_w * state.files_preview_ratio
        } else {
            0.0
        };
        let list_width = content_w - preview_width;

        match state.selected_tab {
            Tab::Drives => {
                let content = gfx::Rectangle {
                    x: content_x,
                    y: content_y,
                    w: list_width,
                    h: content_h,
                };
                list::render_list(
                    frame,
                    content,
                    line_height,
                    highlight_color,
                    &state.drives_list,
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
                                let fs_str = device.fs_type.as_deref().unwrap_or("unknown");
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
                    && let Some(selected_idx) = state.drives_list.selected
                {
                    let prompt_y = content_y + (selected_idx as f32 * line_height)
                        - state.drives_list.scroll_offset;
                    let prompt_bg_color = 0x0D1117EEu32;

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
                let content = gfx::Rectangle {
                    x: content_x,
                    y: content_y,
                    w: list_width,
                    h: content_h,
                };

                // Render the file list
                list::render_list(
                    frame,
                    content,
                    line_height,
                    highlight_color,
                    &state.files_list,
                    |frame, index, y, _is_selected| {
                        if let Some(entry) = state.files_entries.get(index) {
                            let status_color = match entry.entry_type {
                                files::EntryType::Directory => unmounted_color,
                                files::EntryType::File => text_color,
                                files::EntryType::Symlink => mounted_color,
                                files::EntryType::Other => 0x8B949EFFu32,
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
                    let preview_border_color = 0x30363DFFu32;

                    // Draw preview panel border (left edge)
                    frame.fill_rectangle(
                        &gfx::Rectangle {
                            x: preview_x,
                            y: 0.0,
                            w: 1.0,
                            h: window_h,
                        },
                        preview_border_color,
                    );

                    // Display preview content
                    if state.files_preview_entry.is_some() && state.files_preview_valid {
                        let preview_padding = 0.0;

                        let image_area_y = preview_padding;
                        let image_area_w = preview_width - (preview_padding * 2.0);
                        let image_area_h = window_h - (preview_padding * 3.0);

                        // Always render the preview image - gfx will handle it if pixel buffer exists
                        frame.push_image(
                            &state.preview_image,
                            gfx::Rectangle {
                                x: preview_x + preview_padding,
                                y: image_area_y,
                                w: image_area_w,
                                h: image_area_h,
                            },
                            state.files_preview_fit,
                            0xFFFFFFFFu32,
                        );
                    } else {
                        let preview_text = "No preview";
                        frame.push_text(
                            preview_text,
                            preview_x + 10.0,
                            content_y + 20.0,
                            1.0,
                            0x6E7681FFu32,
                            gfx::fonts::CASKAYDIAMONO_FONT_ID,
                        );
                    }
                }
            }
        }

        let status_text = match state.selected_tab {
            Tab::Files => format!("{}", state.files_dir.display()),
            Tab::Drives => "M mount/unmount | E open".to_string(),
        };

        let status_y = 0.0;
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: 0.0,
                y: status_y,
                w: list_width,
                h: STATUS_BAR_HEIGHT,
            },
            bg_color,
        );

        frame.push_text(
            &status_text,
            content_x + 10.0,
            status_y + (STATUS_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
            1.0,
            text_color,
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );
    }
}

fn cache_current_selection(state: &mut std::cell::RefMut<'_, MyApplicationState>) {
    if let Some(idx) = state.files_list.selected
        && let Some(entry) = state.files_entries.get(idx)
    {
        let entry_name = entry.name.clone();
        let current_dir = state.files_dir.clone();
        state.folder_selection_map.insert(current_dir, entry_name);
    }
}

fn update_files_entries(state: &mut std::cell::RefMut<'_, MyApplicationState>, target: PathBuf) {
    match files::get_contents(&target, state.show_hidden_files) {
        Ok(entries) => {
            state.files_dir = target.clone();
            state.files_entries = entries;
            state.files_list.selected = if state.files_entries.is_empty() {
                None
            } else if let Some(last_name) = state.folder_selection_map.get(&target) {
                // Try to restore selection
                state
                    .files_entries
                    .iter()
                    .position(|e| &e.name == last_name)
            } else {
                Some(0)
            };
        }
        Err(e) => {
            log::error!("files::get_contents {} -> {}", target.display(), e)
        }
    }
}

fn toggle_mount(state: &mut std::cell::RefMut<'_, MyApplicationState>) {
    let Some(idx) = state.drives_list.selected else {
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
        let drive_name = device.path.split('/').next_back().unwrap_or("drive");
        let mount_path = format!("/media/{}", drive_name);
        let fs_type = device.fs_type.as_deref().unwrap_or("auto");

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
        }
        Err(e) => {
            log::error!("Failed to rescan block devices: {}", e)
        }
    }
}
