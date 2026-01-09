use rustine::{debug, error, info};
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
        gui.wait_events_timeout(16);
    }

    info!("GUI STOP");
}

fn format_duration(seconds: f64) -> String {
    if seconds >= 3600.0 {
        format!("{:.0} h", seconds / 3600.0)
    } else if seconds >= 60.0 {
        format!("{:.0} m", seconds / 60.0)
    } else if seconds >= 1.0 {
        format!("{:.0} s", seconds)
    } else if seconds >= 1e-3 {
        format!("{:.0} ms", seconds * 1e3)
    } else if seconds >= 1e-6 {
        format!("{:.0} us", seconds * 1e6)
    } else {
        format!("{:.0} ns", seconds * 1e9)
    }
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("GFX START");

    const TARGET_FPS: f64 = 120.0;
    let target_frame_time = std::time::Duration::from_secs_f64(1.0 / TARGET_FPS);
    const SPIN_THRESHOLD: std::time::Duration = std::time::Duration::from_micros(500);

    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();
    let mut last_stat_instant = std::time::Instant::now();
    let mut frame_delta_times: rustine::RingBuffer<f64> = rustine::RingBuffer::new(120);

    while !exit_flag.load(atomic::Ordering::Relaxed) {
        let frame_start = std::time::Instant::now();

        {
            let mut core = gfx.lock().unwrap();

            let now = std::time::Instant::now();
            let t = now.duration_since(start_instant).as_secs_f64();
            let dt = now.duration_since(last_instant).as_secs_f32();
            frame_delta_times.push(dt as f64);
            core.render(t, dt);
            last_instant = now;

            let stat_now = std::time::Instant::now();
            if stat_now.duration_since(last_stat_instant).as_secs_f64() >= 1.0 {
                let frame_cpu_times = core.frame_cpu_times();
                if let Some((min, max, mean)) = frame_cpu_times.min_max_mean() {
                    debug!(
                        "Frame CPU -> min: {}, max: {}, mean: {}",
                        format_duration(min),
                        format_duration(max),
                        format_duration(mean)
                    );
                }
                if let Some((min, max, mean)) = frame_delta_times.min_max_mean() {
                    debug!(
                        "Frame Delta -> min: {}, max: {}, mean: {}",
                        format_duration(min as f64),
                        format_duration(max as f64),
                        format_duration(mean as f64)
                    );
                }
                last_stat_instant = stat_now;
            }
        }

        // Adaptive spin-sleep frame rate limiting
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_time {
            let mut remaining = target_frame_time - elapsed;

            // Sleep for bulk of remaining time
            while remaining > SPIN_THRESHOLD {
                std::thread::sleep(std::time::Duration::from_millis(1));
                remaining = target_frame_time.saturating_sub(frame_start.elapsed());
            }

            // Spin for precise timing
            while frame_start.elapsed() < target_frame_time {
                std::hint::spin_loop();
            }
        }
    }

    info!("GFX STOP");
}
