pub mod gfx;
pub mod gui;
pub mod log;
pub mod version;
pub use version::Version;

/// Status codes returned by FFI functions.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    Error = 1,
    NotSupported = 2,
    InvalidOperation = 3,
}

impl Status {
    pub fn is_ok(&self) -> bool {
        *self == Status::Ok
    }
}

/// Internal library state
struct State {
    gfx: std::sync::Arc<std::sync::Mutex<gfx::Core>>,
    gfx_thread: std::thread::JoinHandle<()>,
    cancel_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl State {
    fn new(gfx: gfx::Core) -> Self {
        let cancel_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let gfx = std::sync::Arc::new(std::sync::Mutex::new(gfx));

        let gfx_clone = std::sync::Arc::clone(&gfx);
        let cancel_clone = std::sync::Arc::clone(&cancel_signal);

        let gfx_thread = std::thread::spawn(move || {
            gfx_thread_function(gfx_clone, cancel_clone);
        });

        State {
            gfx,
            gfx_thread,
            cancel_signal,
        }
    }

    fn shutdown(self) {
        self.cancel_signal
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = self.gfx_thread.join();
    }
}

static STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);

fn gfx_thread_function(
    gfx: std::sync::Arc<std::sync::Mutex<gfx::Core>>,
    cancel_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    log::Log::global().set_current_thread_name("gfx");
    info!("gfx thread started");

    while !cancel_signal.load(std::sync::atomic::Ordering::Relaxed) {
        let _core = gfx.lock().unwrap();
        // Perform work with mutable access to gfx_core
        info!("Performing gfx work");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    info!("gfx thread stopped");
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize(callback: unsafe extern "C" fn(event: *const log::FfiEvent)) -> i32 {
    log::Log::global().set_current_thread_name("main");

    let listener = log::FfiCallbackListener::new(callback);
    let _ = log::Log::global().add_listener(listener);

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
        Err(_) => {
            return Status::Error as i32;
        }
    };

    let state = State::new(gfx_core);
    match STATE.lock() {
        Ok(mut locked_state) => {
            if locked_state.is_some() {
                return Status::InvalidOperation as i32;
            }
            *locked_state = Some(state);
            Status::Ok as i32
        }
        Err(_) => Status::Error as i32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminate() -> i32 {
    match STATE.lock() {
        Ok(mut locked_state) => {
            if let Some(state) = locked_state.take() {
                state.shutdown();
                Status::Ok as i32
            } else {
                Status::Error as i32
            }
        }
        Err(_) => Status::Error as i32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn send_test_notification_to_gfx_thread(_data: i32) -> i32 {
    match STATE.lock() {
        Ok(locked_state) => {
            if locked_state.is_some() {
                Status::Ok as i32
            } else {
                Status::Error as i32
            }
        }
        Err(_) => Status::Error as i32,
    }
}