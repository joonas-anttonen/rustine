use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log};
use rustine_mech::application::*;

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

extern "C" fn handle_sigterm(_signal: i32) {
    log::info!("SIGTERM");
    SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
    rustine::gui::Gui::wake_up();
}
fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_sigterm as *const () as usize;
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

fn main() -> std::process::ExitCode {
    install_signal_handlers();

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    log::debug!("STARTUP");

    {
        let gfx_builder = rustine::gfx::Gfx::builder(rustine::Platform::Wayland)
            .app_name("rustine-mech")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));

        let application = Box::new(MyApplication::new());

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-mech")
            .window_size(900, 600)
            .window_type(rustine::gui::WindowType::Normal);
        let gui = gui_builder.build(Arc::clone(&gfx), application);

        let mode = rustine::RunMode::Event;
        std::thread::scope(|scope| {
            scope.spawn(|| {
                rustine::gfx::run(Arc::clone(&gfx), &SHUTDOWN_FLAG, mode);
            });

            rustine::gui::run(&gui, &SHUTDOWN_FLAG, mode);

            SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
            gfx.lock().unwrap().wake_up();
        });
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}
