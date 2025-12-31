use crate::log::{LogListener, Severity};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct StartupParameters {
    pub callback: Option<unsafe extern "C" fn(event: *const Event)>,
    pub enable_debugging: u32,
    pub physical_device_id: u64,
    pub host_platform: u32,
    pub host_version: crate::version::Version,
    pub host_name: *const u8,
}

// FFI version of `Event`.
#[repr(C)]
pub struct Event {
    pub severity: i32,
    pub timestamp_secs: u64,
    pub timestamp_nanos: u32,
    pub message: *const u8,
    pub message_length: u32,
    pub origin: *const u8,
    pub origin_length: u32,
    pub thread: *const u8,
    pub thread_length: u32,
}

/// A log listener that invokes a foreign function interface (FFI) callback.
pub struct CallbackListener {
    pub callback: unsafe extern "C" fn(event: *const Event),
}

impl LogListener for CallbackListener {
    fn append(&self, event: &crate::log::Event) {
        let duration = event
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let ffi_event = Event {
            severity: match event.severity {
                Severity::Debug => 0,
                Severity::Info => 1,
                Severity::Warning => 2,
                Severity::Error => 3,
            },
            timestamp_secs: duration.as_secs(),
            timestamp_nanos: duration.subsec_nanos(),
            message: event.message.as_ptr(),
            message_length: event.message.len() as u32,
            origin: event.origin.as_ptr(),
            origin_length: event.origin.len() as u32,
            thread: event.thread.as_ptr(),
            thread_length: event.thread.len() as u32,
        };
        unsafe {
            (self.callback)(&ffi_event as *const Event);
        }
    }

    fn flush(&self) {}
}
