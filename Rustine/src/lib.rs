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
    InvalidOperation = 2,
}

impl Status {
    pub fn is_ok(&self) -> bool {
        *self == Status::Ok
    }
}

// Internal library state. Add fields as needed.
struct State {}

impl State {
    fn new() -> Self {
        State {}
    }
}

// Global, single-assignment holder for the state.
// Not thread-safe: callers must ensure no concurrent access.
use std::mem::MaybeUninit;
static mut STATE: MaybeUninit<State> = MaybeUninit::uninit();
static mut INITIALIZED: bool = false;

/// Returns true if the library has been initialized.
pub fn is_initialized() -> bool {
    unsafe { core::ptr::read(core::ptr::addr_of!(INITIALIZED)) }
}

/// Internal helper to access mutable state safely.
/// Returns an error if `initialize()` has not been called.
/*pub(crate) fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Result<R, &'static str> {
    unsafe {
        if !core::ptr::read(core::ptr::addr_of!(INITIALIZED)) {
            return Err("Rustine state not initialized; call initialize() first");
        }
        let state_ptr = core::ptr::addr_of_mut!(STATE);
        let s_ptr: *mut State = (*state_ptr).as_mut_ptr();
        Ok(f(&mut *s_ptr))
    }
}*/

#[unsafe(no_mangle)]
pub extern "C" fn initialize() -> i32 {
    // Attempt to set the state once. Subsequent calls return InvalidOperation.
    unsafe {
        if !core::ptr::read(core::ptr::addr_of!(INITIALIZED)) {
            let state_ptr = core::ptr::addr_of_mut!(STATE);
            (*state_ptr).write(State::new());
            core::ptr::addr_of_mut!(INITIALIZED).write(true);
            println!("Initializing Rustine library...");
            Status::Ok as i32
        } else {
            eprintln!("Rustine initialize() called more than once; ignoring.");
            Status::InvalidOperation as i32
        }
    }
}