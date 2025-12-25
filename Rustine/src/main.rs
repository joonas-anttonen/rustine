mod gfx;
mod gui;
mod log;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let log = log::Log::global();
    log.set_current_thread_name("main");
    log.add_listener(log::ConsoleLogListener::new());

    let params = gfx::ApiParameters {
        enable_debugging: true,
        platform: gfx::parameters::Platform::Windows,
        required_api_version: gfx::Version::new(1, 4, 0),
        app_version: gfx::Version::new(0, 1, 0),
        app_engine_version: gfx::Version::new(0, 1, 0),
        app_name: "Rustine".to_string(),
        app_engine_name: "Rustine".to_string(),
    };

    let gfx_core = match gfx::core::Core::new(&params) {
        Ok(core) => core,
        Err(e) => {
            error!("Failed to create gfx::Core: {}", e);
            return;
        }
    };
    match gfx_core.enumerate_physical_devices() {
        Ok(devices) => {
            for device in devices {
                info!("Found device: {}", device);
            }
        }
        Err(e) => {
            error!("Failed to enumerate physical devices: {}", e);
        }
    }

    // Spawn cancelable background thread with mutable access to gfx_core
    let gfx = Arc::new(Mutex::new(gfx_core));
    let gfx_cancel_signal = AtomicBool::new(false);

    let gui = gui::Core::new(Arc::clone(&gfx));

    thread::scope(|s| {
        s.spawn(|| {
            gfx_thread_function(Arc::clone(&gfx), &gfx_cancel_signal);
        });

        gui_thread_function(&gui);

        gfx_cancel_signal.store(true, Ordering::Relaxed);
    });
}

fn gui_thread_function(_gui: &gui::Core) {
    // Keep main thread alive for a bit, then signal cancellation
    for _ in 0..5 {
        warning!("Performing gui work");
        thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::core::Core>>, cancel_signal: &AtomicBool) {
    log::Log::global().set_current_thread_name("gfx");

    info!("gfx thread started");

    while !cancel_signal.load(Ordering::Relaxed) {
        if let Ok(_core) = gfx.lock() {
            // Perform work with mutable access to gfx_core
            info!("Performing gfx work");
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }

    info!("gfx thread stopped");
}
