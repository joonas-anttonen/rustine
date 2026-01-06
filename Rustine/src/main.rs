use rustine::{error, info};
use rustine::{gfx, gui, log::*, version::Version};

#[cfg(unix)]
use libc;
use std::io;
use std::sync::{Arc, Mutex, atomic};
use std::thread;

static EXIT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn handle_termination_signal(_signal: i32) {
    info!("SIGTERM");
    EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
}

#[cfg(unix)]
fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_termination_signal as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            let os_error = io::Error::last_os_error();
            eprintln!("Failed to install SIGINT handler: {os_error:?}",);
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            let os_error = io::Error::last_os_error();
            eprintln!("Failed to install SIGTERM handler: {os_error:?}",);
        }
    }
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

fn main() {
    let platform = if cfg!(target_os = "linux") {
        gfx::Platform::Wayland
    } else {
        panic!("Unsupported platform");
    };

    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleListener::new(true));

    install_signal_handlers();

    info!("STARTUP");

    {
        let params = gfx::StartupParameters {
            enable_debugging: true,
            host_platform: platform,
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
        info!("{dev}");

        let gfx = Arc::new(Mutex::new(gfx_core));

        let gui_params = gui::StartupParameters {
            platform,
            window_title: "Rustine".to_string(),
            window_width: Some(1920),
            window_height: Some(1080),
        };
        let gui = gui::Gui::new(Arc::clone(&gfx), gui_params);

        thread::scope(|s| {
            s.spawn(|| {
                gfx_thread_function(Arc::clone(&gfx), &EXIT_FLAG);
            });

            gui_thread_function(&gui, &EXIT_FLAG);

            EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
        });
    }

    info!("SHUTDOWN");
}

fn gui_thread_function(gui: &gui::Gui, exit_flag: &atomic::AtomicBool) {
    info!("GUI START");

    while !exit_flag.load(atomic::Ordering::Relaxed) && !gui.should_close() {
        gui.process_events();
    }

    info!("GUI STOP");
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("GFX START");

    let mut skip_frame = false;
    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();

    while !exit_flag.load(atomic::Ordering::Relaxed) {
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

    info!("GFX STOP");
}
