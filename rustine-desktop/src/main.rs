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
                let mounted_count = devices.iter().filter(|d| d.mount_info.is_some()).count();
                let unmounted_count = devices.len() - mounted_count;
                log::info!(
                    "Found {} devices ({} mounted, {} unmounted)",
                    devices.len(),
                    mounted_count,
                    unmounted_count
                );
                state.devices = devices;
            }
            Err(e) => log::error!("Failed to get block devices: {}", e),
        }
    }

    fn on_key(&self, gui: &rustine::gui::Gui, _key: rustine::gui::KeyEvent) {
        log::debug!("Application::on_key: {:?} {:?}", _key.key, _key.action);

        if _key.key == rustine::gui::Key::ESCAPE {
            gui.request_quit();
        }
    }

    fn on_char(&self, _gui: &rustine::gui::Gui, c: char) {
        log::debug!("Application::on_char: U+{:04X} ('{}')", c as u32, c);
    }

    fn render(&self, _gui: &rustine::gui::Gui, frame: &mut rustine::gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();

        let w = frame.size.x as f32;
        let h = frame.size.y as f32;

        const TOP_BAR_HEIGHT: f32 = 48.0;
        const SIDE_BAR_WIDTH: f32 = 48.0;
        const LINE_HEIGHT: f32 = 20.0;

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

        let text_font_metrics =
            rustine::gfx::fonts::get_font_metrics(rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID)
                .expect("Font metrics exist");

        // Title in top bar
        frame.push_text(
            "Drive Manager",
            content_x + 10.0,
            (TOP_BAR_HEIGHT / 2.0) + (text_font_metrics.ascender / 2.0),
            1.0,
            text_color,
            rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );

        let mut y = content_y + text_font_metrics.ascender;

        const STATUS_LIGHT_WIDTH: f32 = 4.0;
        const STATUS_LIGHT_HEIGHT: f32 = 14.0;
        const STATUS_LIGHT_MARGIN: f32 = 10.0;
        const TEXT_START_X: f32 = STATUS_LIGHT_MARGIN + STATUS_LIGHT_WIDTH + 8.0;

        // Draw all drives in a flat list (partitions only)
        for device in state.devices.iter().filter(|d| d.is_partition) {
            let status_y = y - (STATUS_LIGHT_HEIGHT / 2.0) - 2.0;
            
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
                    y: status_y,
                    w: STATUS_LIGHT_WIDTH,
                    h: STATUS_LIGHT_HEIGHT,
                },
                status_color,
            );

            frame.push_text(
                &drive_text,
                content_x + TEXT_START_X,
                y - text_font_metrics.descender,
                1.0,
                text_color,
                rustine::gfx::fonts::CASKAYDIAMONO_FONT_ID,
            );
            y += LINE_HEIGHT;
        }

        state.frame_index += 1;
    }
}
