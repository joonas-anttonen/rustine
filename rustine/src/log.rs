//! A logging module providing event-based logging with pluggable listeners.
//!
//! This module provides an event logging system where log messages are captured
//! as `Event` objects and distributed to registered `LogListener` implementations.

#![allow(dead_code)] // Come on, this is a library!

use std::{fmt, sync, thread, time};

/// A log event containing severity level, timestamp, and contextual information.
///
/// Events are created with a severity level, message, type name, method name, and
/// thread identifier. They can be formatted as short or full strings for output.
#[derive(Clone, Debug)]
pub struct Event {
    pub severity: Severity,
    pub timestamp: time::SystemTime,
    pub message: String,
    pub origin: String,
    pub thread: String,
}

impl Event {
    /// Creates a new log event with the specified severity and context.
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity level of the event
    /// * `message` - The log message
    /// * `r#type` - The type or class name where the event originated
    /// * `method` - The method or function name
    /// * `thread` - The name of the thread generating the event
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        origin: impl Into<String>,
        thread: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            timestamp: time::SystemTime::now(),
            message: message.into(),
            origin: origin.into(),
            thread: thread.into(),
        }
    }

    /// Formats the event as a short string with thread, origin, and message.
    pub fn to_short_string(&self) -> String {
        format!("[{}] {} {}", self.thread, &self.origin, self.message)
    }

    /// Formats the event as a full string including timestamp and severity.
    pub fn to_full_string(&self) -> String {
        format!(
            "[{}] [{}] [{}] {} {}",
            Event::datetime_iso8601(self.timestamp),
            self.thread,
            self.severity,
            &self.origin,
            self.message
        )
    }

    /// Formats a `SystemTime` as an ISO 8601 string with milliseconds and timezone.
    fn datetime_iso8601(tp: time::SystemTime) -> String {
        let datetime: chrono::DateTime<chrono::Local> = tp.into();
        datetime.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string()
    }
}

/// Log severity levels.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "R"),
            Severity::Warning => write!(f, "W"),
            Severity::Info => write!(f, "I"),
            Severity::Debug => write!(f, "D"),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_full_string())
    }
}

/// A trait for receiving and handling log events.
///
/// Implementations of this trait receive log events and can handle them
/// (e.g., write to a file, console, or network). Must be `Send + Sync`
/// to work across threads.
pub trait LogListener: Send + Sync {
    /// Appends a log event for processing.
    fn append(&self, event: &Event);
    /// Flushes any buffered log data.
    fn flush(&self);
}

/// A log listener that outputs events to the console with color coding.
///
/// Events are formatted as short strings and colored based on severity
/// (blue for debug, white for info, yellow for warning, red for error).
pub struct ConsoleListener {
    use_short_display: bool,
    time_base: time::SystemTime,
}

impl ConsoleListener {
    /// Creates a new console log listener.
    pub fn new(use_short_display: bool) -> Self {
        Self {
            use_short_display,
            time_base: time::SystemTime::now(),
        }
    }

    fn color_code(sev: Severity) -> &'static str {
        match sev {
            Severity::Debug => "\u{1b}[34m",
            Severity::Info => "\u{1b}[0m",
            Severity::Warning => "\u{1b}[33m",
            Severity::Error => "\u{1b}[31m",
        }
    }
}

impl LogListener for ConsoleListener {
    fn append(&self, event: &Event) {
        let color = Self::color_code(event.severity);
        let reset = "\u{1b}[0m";
        let output = if self.use_short_display {
            let time_diff = event
                .timestamp
                .duration_since(self.time_base)
                .unwrap_or_default();
            format!(
                "[+{} ms] {}",
                time_diff.as_millis(),
                event.to_short_string()
            )
        } else {
            event.to_full_string()
        };
        println!("{}{}{}", color, output, reset);
    }

    fn flush(&self) {}
}

/// The main logging system that manages listeners and distributes log events.
///
/// Log events are collected with metadata (severity, timestamp, origin) and
/// distributed to all registered listeners. Thread names can be registered
/// for better context in log output.
pub struct Log {
    listeners: sync::RwLock<Vec<sync::Arc<dyn LogListener>>>,
    thread_names: sync::Mutex<std::collections::HashMap<thread::ThreadId, String>>,
}

impl Log {
    /// Creates a new empty logger with no listeners.
    pub fn new() -> Self {
        Self {
            listeners: sync::RwLock::new(Vec::new()),
            thread_names: sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Returns a reference to the global log instance.
    ///
    /// This creates a single shared logger instance on first call,
    /// returning the same instance on subsequent calls.
    pub fn global() -> &'static Self {
        use std::sync::OnceLock;
        static LOG: OnceLock<Log> = OnceLock::new();
        LOG.get_or_init(Self::new)
    }

    /// Registers a new listener to receive log events.
    ///
    /// Returns an `Arc` to the listener for potential later reference.
    pub fn add_listener<L: LogListener + 'static>(
        &self,
        listener: L,
    ) -> sync::Arc<dyn LogListener> {
        let arc_listener: sync::Arc<dyn LogListener> = sync::Arc::new(listener);
        self.listeners.write().unwrap().push(arc_listener.clone());
        arc_listener
    }

    /// Sets the name of the current thread for use in log events.
    pub fn set_current_thread_name(&self, name: impl Into<String>) {
        self.thread_names
            .lock()
            .unwrap()
            .insert(thread::current().id(), name.into());
    }

    /// Flushes all registered listeners.
    pub fn flush_all(&self) {
        for l in self.listeners.read().unwrap().iter() {
            l.flush();
        }
    }

    /// Appends a log event with the specified severity and context information.
    pub fn append(&self, severity: Severity, message: &str, type_name: &str) {
        let thread_name = {
            let map = self.thread_names.lock().unwrap();
            map.get(&thread::current().id())
                .cloned()
                .unwrap_or_else(|| "unknown".to_string())
        };

        let ev = Event::new(severity, message, type_name, thread_name);
        for l in self.listeners.read().unwrap().iter() {
            l.append(&ev);
        }
    }
}

/// Sets the name of the current thread to the global logger instance.
pub fn set_current_thread_name(name: impl Into<String>) {
    Log::global().set_current_thread_name(name);
}

/// Adds a new listener to the global logger instance.
pub fn add_listener<L: LogListener + 'static>(listener: L) -> sync::Arc<dyn LogListener> {
    Log::global().add_listener(listener)
}

// Re-export macros into this module's namespace
pub use crate::{debug, error, info, warning};

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::log::Log::global().append(
            $crate::log::Severity::Debug,
            &format!($($arg)*),
            module_path!()
        );
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::log::Log::global().append(
            $crate::log::Severity::Info,
            &format!($($arg)*),
            module_path!()
        );
    }};
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        $crate::log::Log::global().append(
            $crate::log::Severity::Warning,
            &format!($($arg)*),
            module_path!()
        );
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::log::Log::global().append(
            $crate::log::Severity::Error,
            &format!($($arg)*),
            module_path!()
        );
    }};
}
