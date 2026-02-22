use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log};
use rustine_ed::application::*;

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

fn main() -> std::process::ExitCode {
    rustine::install_shutdown_signal_handlers(&SHUTDOWN_FLAG);

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    {
        let gfx_builder = rustine::gfx::Gfx::builder()
            .platform_hint(rustine::Platform::Wayland)
            .app_name("rustine-ed")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));

        let application = Box::new(MyApplication::new());

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-ed")
            .window_size(900, 600);
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

    std::process::ExitCode::SUCCESS
}
