use rustine::{error, info};
use rustine::{gfx, gui, log::ConsoleLogListener, log::Log, version::Version};

use std::sync::{Arc, Mutex, atomic};
use std::thread;

fn main() {
    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleLogListener::new(true));

    info!("STARTUP");

    {
        let params = gfx::StartupParameters {
            enable_debugging: true,
            host_platform: gfx::Platform::Windows,
            host_version: Version::new(0, 1, 0),
            host_name: "rustine-app".to_string(),
        };

        let gfx_core = match gfx::Core::builder(params).select_optimal_device().build() {
            Ok(core) => core,
            Err(e) => {
                error!("Failed to build gfx::Core: {}", e);
                return;
            }
        };

        let dev = gfx_core.selected_physical_device();
        info!("Selected device: {}", dev);

        let gfx = Arc::new(Mutex::new(gfx_core));
        let gfx_cancel_signal = atomic::AtomicBool::new(false);

        let gui = gui::Core::new(Arc::clone(&gfx));

        thread::scope(|s| {
            s.spawn(|| {
                gfx_thread_function(Arc::clone(&gfx), &gfx_cancel_signal);
            });

            gui_thread_function(&gui);

            gfx_cancel_signal.store(true, atomic::Ordering::Relaxed);
        });
    }

    info!("SHUTDOWN");
}

fn gui_thread_function(gui: &gui::Core) {
    while !gui.should_close() {
        gui.process_events();
    }
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, cancel_signal: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("gfx thread started");

    let mut skip_frame = false;
    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();

    while !cancel_signal.load(atomic::Ordering::Relaxed) {
        {
            let attempted_lock = gfx.try_lock();
            if attempted_lock.is_err() {
                skip_frame = true;
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            if skip_frame {
                info!("CONTINUE");
                skip_frame = false;
            }
            let mut core = attempted_lock.unwrap();

            let now = std::time::Instant::now();
            let t = now.duration_since(start_instant).as_secs_f64();
            let dt = now.duration_since(last_instant).as_secs_f32();
            core.render(t, dt);
            last_instant = now;
        }

        // NOTE: Sleeping seems necessary
        // If we don't sleep, the host thread struggles to acquire the lock,
        // causing unresponsiveness, especially when resizing the window.
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    info!("gfx thread stopped");
}
