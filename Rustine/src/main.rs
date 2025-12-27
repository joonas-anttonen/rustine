use rustine::{error, info};
use rustine::{gfx, gui, log::ConsoleLogListener, log::Log, version::Version};

use std::sync::{Arc, Mutex, atomic};
use std::{thread, time};

fn main() {
    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleLogListener::new(true));

    info!("Enter");

    {
        let params = gfx::ApiParameters {
            enable_debugging: true,
            platform: gfx::Platform::Windows,
            required_api_version: Version::new(1, 4, 0),
            app_version: Version::new(0, 1, 0),
            app_engine_version: Version::new(0, 1, 0),
            app_name: "Rustine".to_string(),
            app_engine_name: "Rustine".to_string(),
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

    info!("Exit");
}

fn gui_thread_function(gui: &gui::Core) {
    // Keep main thread alive for a bit, then signal cancellation
    while !gui.should_close() {
        //warning!("Performing gui work");

        gui.process_events();
    }
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, cancel_signal: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("gfx thread started");

    while !cancel_signal.load(atomic::Ordering::Relaxed) {
        let _core = gfx.lock().unwrap();

        // Perform work with mutable access to gfx_core
        info!("Performing gfx work");

        thread::sleep(time::Duration::from_millis(100));
    }

    info!("gfx thread stopped");
}
