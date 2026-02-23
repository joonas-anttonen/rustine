use std::sync::{Arc, Mutex};

use rustine::{Version, log};
use rustine_mech::application::*;

fn main() -> std::process::ExitCode {
    rustine::reset_exit_request();
    rustine::install_shutdown_signal_handlers();

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    log::debug!("STARTUP");

    {
        let gfx_builder = rustine::gfx::Gfx::builder()
            .platform(rustine::Platform::Wayland)
            .app_name("rustine-mech")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));

        let application = Box::new(MyApplication::new());

        let gui_builder = rustine::gui::Gui::builder()
            .window_title("rustine-mech")
            .window_size(900, 600);
        let gui = gui_builder.build(Arc::clone(&gfx), application);

        let mode = rustine::RunMode::Event;
        std::thread::scope(|scope| {
            scope.spawn(|| {
                rustine::gfx::run(Arc::clone(&gfx), mode);
            });

            rustine::gui::run(&gui, mode);

            rustine::request_exit();
            gfx.lock().unwrap().wake_up();
        });
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}
