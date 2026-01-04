pub mod gfx;
pub mod gui;
pub mod log;
pub mod io;
pub mod version;
pub use version::Version;

use crate::gfx::Status;
use std::{sync::Arc, sync::Mutex, sync::atomic};

mod lib_ffi;

/// Internal library state
struct State {
    gfx: Arc<Mutex<gfx::Core>>,
    gfx_thread: std::thread::JoinHandle<()>,
    gfx_shutdown_signal: Arc<atomic::AtomicBool>,
}

impl State {
    fn new(gfx: gfx::Core) -> Self {
        let gfx_shutdown_signal = Arc::new(atomic::AtomicBool::new(false));
        let gfx = Arc::new(Mutex::new(gfx));

        let gfx_clone = Arc::clone(&gfx);
        let gfx_shutdown_signal_clone = Arc::clone(&gfx_shutdown_signal);

        let gfx_thread = std::thread::spawn(move || {
            gfx_thread_function(gfx_clone, gfx_shutdown_signal_clone);
        });

        State {
            gfx,
            gfx_thread,
            gfx_shutdown_signal,
        }
    }

    fn shutdown(self) {
        self.gfx_shutdown_signal
            .store(true, atomic::Ordering::Relaxed);
        let _ = self.gfx_thread.join();
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, shutdown_signal: Arc<atomic::AtomicBool>) {
    log::Log::global().set_current_thread_name("gfx");
    info!("gfx thread started");

    let mut skip_frame = false;
    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();

    while !shutdown_signal.load(atomic::Ordering::Relaxed) {
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

#[unsafe(no_mangle)]
pub extern "C" fn rustine_startup(startup_parameters: *const lib_ffi::StartupParameters) -> i32 {
    let mut state = STATE.lock().unwrap();
    if state.is_some() {
        return Status::InvalidOperation(-1).to_code();
    }

    // 1. Take a clone of the parameters to avoid excessive 'unsafe'.
    //    callback and host_name still remain unsafe.
    let in_params = unsafe { (*startup_parameters).clone() };

    // 2. Initialize logging
    let log = log::Log::global();
    log.set_current_thread_name("host");
    if in_params.callback.is_some() {
        let _ = log.add_listener(lib_ffi::CallbackListener {
            callback: in_params.callback.unwrap(),
        });
    }

    // 3. Convert startup parameters
    let params = gfx::StartupParameters {
        enable_debugging: in_params.enable_debugging != 0,
        host_platform: match in_params.host_platform {
            0 => gfx::Platform::Windows,
            1 => gfx::Platform::Wayland,
            2 => gfx::Platform::X11,
            3 => gfx::Platform::MacOS,
            _ => gfx::Platform::Windows,
        },
        host_version: in_params.host_version,
        host_name: unsafe {
            std::ffi::CStr::from_ptr(in_params.host_name as *const i8)
                .to_string_lossy()
                .into_owned()
        },
    };

    // 4. Build gfx::Core
    let mut builder = gfx::Core::builder(params);
    if in_params.physical_device_id != 0 {
        builder = builder.select_device_by_luid(in_params.physical_device_id);
    }
    let gfx_core = match builder.build() {
        Ok(core) => core,
        Err(e) => {
            return e.to_code();
        }
    };

    *state = Some(State::new(gfx_core));
    Status::Success.to_code()
}

#[unsafe(no_mangle)]
pub extern "C" fn rustine_shutdown() -> i32 {
    match STATE.lock() {
        Ok(mut locked_state) => {
            if let Some(state) = locked_state.take() {
                state.shutdown();
                Status::Success.to_code()
            } else {
                Status::NotSupported(-1).to_code()
            }
        }
        Err(_) => Status::NotSupported(-1).to_code(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rustine_gfx_initialize_presentation(
    params: *const gfx::presentation::Parameters,
) -> i32 {
    let state = STATE.lock().unwrap();
    if state.is_none() {
        return Status::InvalidOperation(-1).to_code();
    }

    let in_params = unsafe { (*params).clone() };

    let gfx_in_state = &state.as_ref().unwrap().gfx;
    let mut gfx = gfx_in_state.lock().unwrap();

    let shared_image_provider =
        gfx::presentation::SharedImageProvider::new(gfx.allocator(), in_params);
    gfx.initialize_shared_image_queue(shared_image_provider);

    Status::Success.to_code()
}
