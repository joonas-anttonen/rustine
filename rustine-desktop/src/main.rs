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
        state: std::cell::RefCell::new(MyApplicationState { frame_index: 0 }),
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
}

struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl rustine::gui::Application for MyApplication {
    fn startup(&self, _gui: &rustine::gui::Gui) {
        log::debug!("Application::startup");

        // Enumerate mounted drives
        match sysinfo::get_mounted_drives() {
            Ok(mounted) => {
                log::info!("Mounted");
                for drive in mounted {
                    log::info!(
                        "{} -> {} ({})",
                        drive.device,
                        drive.mount_point,
                        drive.fs_type
                    );
                }
            }
            Err(e) => log::error!("Failed to get mounted drives: {}", e),
        }

        // Enumerate unmounted drives
        match sysinfo::get_unmounted_drives() {
            Ok(unmounted) => {
                log::info!("Unmounted");
                if unmounted.is_empty() {
                    log::info!("  (none)");
                } else {
                    for drive in unmounted.iter().filter(|d| d.is_partition) {
                        let size_str = if let Some(size) = drive.size {
                            format!("{:.2} GB", size as f64 / 1_000_000_000.0)
                        } else {
                            "unknown size".to_string()
                        };
                        let fs_str = drive
                            .fs_type
                            .as_ref()
                            .map(|s| format!("{}", s))
                            .unwrap_or_default();
                        log::info!("{} ({}) ({})", drive.path, size_str, fs_str);
                    }
                }
            }
            Err(e) => log::error!("Failed to get unmounted drives: {}", e),
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

        let content_x = SIDE_BAR_WIDTH;
        let content_y = TOP_BAR_HEIGHT;
        //let content_w = w - SIDE_BAR_WIDTH;
        let content_h = h - TOP_BAR_HEIGHT;

        let bar_color = 0x1B232F_FFu32;

        // Fill background
        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            bar_color,
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
            rustine::gfx::fonts::get_font_metrics(rustine::gfx::fonts::DEPARTUREMONO_FONT_ID)
                .expect("Font metrics exist");

        // Switch text color every other frame between pastel red and green
        let frame_indicator_color = if state.frame_index % 2 == 0 {
            0xFFAAAA_FFu32
        } else {
            0xAAFFAA_FFu32
        };

        frame.fill_rectangle(
            &rustine::gfx::Rectangle {
                x: 4.0,
                y: 4.0,
                w: 10.0,
                h: 10.0,
            },
            frame_indicator_color,
        );

        let text_color = 0xFFFFFF_FFu32;

        let counts = rustine::alloc::counts();
        //let count = counts.0 - counts.1;
        let current = rustine::alloc::current_bytes();
        let peak = rustine::alloc::peak_bytes();
        let vram = rustine::gfx::Gfx::current_allocated_vram_bytes();
        let rss = rustine::alloc::rss_bytes();

        let txt = format!(
            "FRAME {}\nALLOCS ({}) {}, peak: {} | VRAM: {} | RAM: {}",
            state.frame_index.to_string(),
            counts.0,
            rustine::utilities::format_bytes_iec(current),
            rustine::utilities::format_bytes_iec(peak),
            rustine::utilities::format_bytes_iec(vram),
            rustine::utilities::format_bytes_iec(rss.unwrap_or(0)),
        );

        // Draw debug text
        frame.push_text(
            txt.as_str(),
            content_x,
            content_y + text_font_metrics.ascender,
            1.0,
            text_color,
            rustine::gfx::fonts::DEPARTUREMONO_FONT_ID,
        );

        state.frame_index += 1;
    }
}
