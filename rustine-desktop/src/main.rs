use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log};

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

extern "C" fn handle_sigterm(_signal: i32) {
    log::info!("SIGTERM");
    SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
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

    let application = Box::new(MyApplication);

    {
        let gfx_builder = rustine::gfx::Gfx::builder(rustine::Platform::Wayland)
            .app_name("rustine-desktop")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-desktop")
            .window_type(rustine::gui::WindowType::Normal);

        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));
        let gui = gui_builder.build(gfx.clone(), application);

        std::thread::scope(|scope| {
            scope.spawn(|| {
                rustine::gfx::Gfx::run(gfx.clone(), &SHUTDOWN_FLAG, rustine::gfx::LoopMode::Event);
            });

            rustine::gui::Gui::run(&gui, &SHUTDOWN_FLAG, rustine::gfx::LoopMode::Event);

            SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
            gfx.lock().unwrap().signal_work_available();
        });
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}

struct MyApplication;

impl rustine::gui::Application for MyApplication {
    fn startup(&self) {
        log::debug!("Application::startup");
    }

    fn on_key(&self, _key: rustine::gui::KeyEvent) {
        log::debug!("Application::on_key: {:?} {:?}", _key.key, _key.action);
    }

    fn render(&self, frame: &mut rustine::gfx::RenderFrame) {
        let w = frame.size.x as f32;
        let h = frame.size.y as f32;

        const TOP_BAR_HEIGHT: f32 = 48.0;
        const SIDE_BAR_WIDTH: f32 = 48.0;

        let content_x = SIDE_BAR_WIDTH;
        let content_y = TOP_BAR_HEIGHT;
        //let content_w = w - SIDE_BAR_WIDTH;
        let content_h = h - TOP_BAR_HEIGHT;

        let color = 0xFFFFFF_FFu32;
        let bar_color = 0x1B232F_FFu32;

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

        // Draw debug text
        frame.push_text(
            "DEBUG",
            content_x,
            content_y + text_font_metrics.ascender,
            1.0,
            color,
            rustine::gfx::fonts::DEPARTUREMONO_FONT_ID,
        );
    }
}
