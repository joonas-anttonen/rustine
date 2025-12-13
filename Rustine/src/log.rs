//! A logging module providing event-based logging with pluggable listeners.
//!
//! This module provides an event logging system where log messages are captured
//! as `Event` objects and distributed to registered `LogListener` implementations.

#![allow(dead_code)] // Come on, this is a library!

use std::{fmt, time, sync, thread};

/// A log event containing severity level, timestamp, and contextual information.
///
/// Events are created with a severity level, message, type name, method name, and
/// thread identifier. They can be formatted as short or full strings for output.
#[derive(Clone, Debug)]
pub struct Event {
    pub severity: Severity,
    pub timestamp: time::SystemTime,
    pub message: String,
    pub r#type: String,
    pub method: String,
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
        r#type: impl Into<String>,
        method: impl Into<String>,
        thread: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            timestamp: time::SystemTime::now(),
            message: message.into(),
            r#type: r#type.into(),
            method: method.into(),
            thread: thread.into(),
        }
    }

    /// Formats the event as a short string with thread, origin, and message.
    pub fn to_short_string(&self) -> String {
        format!(
            "[{}] {} {}",
            self.thread,
            origin_string(&self.r#type, &self.method),
            self.message
        )
    }

    /// Formats the event as a full string including timestamp and severity.
    pub fn to_string_full(&self) -> String {
        format!(
            "[{}] [{}] [{}] {} {}",
            self.thread,
            datetime_iso8601(self.timestamp),
            self.severity,
            origin_string(&self.r#type, &self.method),
            self.message
        )
    }
}

/// Log severity levels.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Debug,
    Information,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Information => write!(f, "INFO"),
            Severity::Debug => write!(f, "DEBUG"),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_full())
    }
}

fn origin_string(r#type: &str, method: &str) -> String {
    format!("{}::{}", r#type, method)
}

fn datetime_iso8601(tp: time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = tp.into();
    datetime.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()
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
pub struct ConsoleLogListener;

impl ConsoleLogListener {
    /// Creates a new console log listener.
    pub fn new() -> Self {
        Self
    }

    fn color_code(sev: Severity) -> &'static str {
        match sev {
            Severity::Debug => "\u{1b}[34m",
            Severity::Information => "\u{1b}[0m",
            Severity::Warning => "\u{1b}[33m",
            Severity::Error => "\u{1b}[31m",
        }
    }
}

impl LogListener for ConsoleLogListener {
    fn append(&self, event: &Event) {
        let color = Self::color_code(event.severity);
        let reset = "\u{1b}[0m";
        println!("{}{}{}", color, event.to_short_string(), reset);
    }

    fn flush(&self) {
    }
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
    pub fn add_listener<L: LogListener + 'static>(&self, listener: L) -> sync::Arc<dyn LogListener> {
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
    pub fn append(&self, severity: Severity, message: &str, type_name: &str, method_name: &str) {
        let thread_name = {
            let map = self.thread_names.lock().unwrap();
            map.get(&thread::current().id())
                .cloned()
                .unwrap_or_else(|| "unknown".to_string())
        };

        let ev = Event::new(severity, message, type_name, method_name, thread_name);
        for l in self.listeners.read().unwrap().iter() {
            l.append(&ev);
        }
    }

    /// Creates a Logger instance with the specified type and caller_name context.
    ///
    /// Returns a Logger that will include the provided type and caller_name
    /// information in all subsequent log messages.
    pub fn logger<'a>(&'a self, type_name: impl Into<String>, caller_name: impl Into<String>) -> Logger<'a> {
        Logger {
            log: self,
            type_name: type_name.into(),
            caller_name: caller_name.into(),
        }
    }
}

/// A logger instance that captures type and caller_name context for logging.
///
/// Created by calling `Log::logger()`, this struct stores contextual
/// information and provides methods to log at different severity levels.
pub struct Logger<'a> {
    log: &'a Log,
    type_name: String,
    caller_name: String,
}

impl Logger<'_> {
    /// Logs an error message.
    pub fn error(&self, message: impl AsRef<str>) {
        self.log.append(
            Severity::Error,
            message.as_ref(),
            &self.type_name,
            &self.caller_name,
        );
    }

    /// Logs a warning message.
    pub fn warning(&self, message: impl AsRef<str>) {
        self.log.append(
            Severity::Warning,
            message.as_ref(),
            &self.type_name,
            &self.caller_name,
        );
    }

    /// Logs an informational message.
    pub fn info(&self, message: impl AsRef<str>) {
        self.log.append(
            Severity::Information,
            message.as_ref(),
            &self.type_name,
            &self.caller_name,
        );
    }

    /// Logs a debug message.
    pub fn debug(&self, message: impl AsRef<str>) {
        self.log.append(
            Severity::Debug,
            message.as_ref(),
            &self.type_name,
            &self.caller_name,
        );
    }

    /// Logs a function call entry as a debug message with empty message.
    pub fn func(&self) {
        self.debug("");
    }
}
