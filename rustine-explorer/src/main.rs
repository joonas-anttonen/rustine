use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log, scene::Scene};
use rustine_explorer::application::*;

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

fn main() -> std::process::ExitCode {
    rustine::install_shutdown_signal_handlers(&SHUTDOWN_FLAG);

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    log::debug!("STARTUP");

    {
        let gfx_builder = rustine::gfx::Gfx::builder()
            .platform_hint(rustine::Platform::Wayland)
            .app_name("rustine-desktop")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));

        let scene = Arc::new(Mutex::new(Scene::new(Arc::clone(&gfx))));
        let application = Box::new(MyApplication::new(Arc::clone(&scene)));

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-desktop")
            .window_size(1280, 720);
        let gui = gui_builder.build(Arc::clone(&gfx), application);

        let mode = rustine::RunMode::Continuous;
        std::thread::scope(|scope| {
            scope.spawn(|| {
                rustine::gfx::run(Arc::clone(&gfx), &SHUTDOWN_FLAG, mode);
            });

            scope.spawn(|| {
                rustine::scene::run(Arc::clone(&scene), &SHUTDOWN_FLAG, mode);
            });

            rustine::gui::run(&gui, &SHUTDOWN_FLAG, mode);

            SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
            gfx.lock().unwrap().wake_up();
            scene.lock().unwrap().wake_up();
        });
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}
