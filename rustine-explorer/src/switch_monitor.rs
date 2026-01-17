use std::sync::{Arc, Mutex};

/// Represents a switch event detected via udev
#[derive(Debug, Clone)]
pub enum SwitchEvent {
    /// A switch state changed
    Changed {
        /// Switch type (lid, tablet mode, etc.)
        switch_type: SwitchType,
        /// Current state
        state: SwitchState,
        /// Device properties
        properties: SwitchProperties,
    },
    /// A switch device was added
    Added {
        /// Switch type
        switch_type: SwitchType,
        /// Device properties
        properties: SwitchProperties,
    },
    /// A switch device was removed
    Removed {
        /// Switch type
        switch_type: SwitchType,
        /// System path
        syspath: String,
    },
}

/// Type of switch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchType {
    /// Laptop lid switch
    Lid,
    /// Tablet mode switch (2-in-1 devices)
    TabletMode,
    /// Headphone jack
    Headphone,
    /// Microphone jack
    Microphone,
    /// Line out
    LineOut,
    /// Video out
    VideoOut,
    /// Camera lens cover
    CameraLensCover,
    /// Keypad slide (old phones)
    KeypadSlide,
    /// Front proximity sensor
    FrontProximity,
    /// Rotate lock
    RotateLock,
    /// Linein jack
    LineIn,
    /// Mute switch
    Mute,
    /// Unknown/other switch type
    Unknown(String),
}

/// Switch state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchState {
    /// Switch is off/open
    Off,
    /// Switch is on/closed
    On,
    /// Unknown state
    Unknown,
}

/// Properties of a switch from udev
#[derive(Debug, Clone)]
pub struct SwitchProperties {
    /// System path
    pub syspath: String,
    /// Device name
    pub name: Option<String>,
    /// Physical path
    pub phys: Option<String>,
    /// Device node (e.g., /dev/input/event*)
    pub devnode: Option<String>,
    /// All switch states (bitmask representation)
    pub switch_states: Option<String>,
}

/// Callback function type for handling switch events
pub type SwitchEventCallback = Box<dyn Fn(SwitchEvent) + Send + 'static>;

/// Monitor for switch events via udev
pub struct UdevSwitchMonitor {
    callback: Arc<Mutex<Option<SwitchEventCallback>>>,
}

impl UdevSwitchMonitor {
    /// Create a new udev switch monitor
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(UdevSwitchMonitor {
            callback: Arc::new(Mutex::new(None)),
        })
    }

    /// Set a callback to be called when switch events occur
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(SwitchEvent) + Send + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(callback));
    }

    /// Start monitoring for switch events
    /// This will block until an error occurs
    pub fn monitor(&self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("input")?
            .listen()?;

        let callback = Arc::clone(&self.callback);

        loop {
            if let Some(event) = socket.iter().next() {
                let event_type = event.event_type();
                let device = event.device();

                // Only process switch devices
                if !is_switch_device(&device) {
                    continue;
                }

                match event_type {
                    udev::EventType::Add => {
                        if let Some(switch_type) = detect_switch_type(&device) {
                            let properties = extract_switch_properties(&event);

                            if let Ok(cb_lock) = callback.lock()
                                && let Some(cb) = cb_lock.as_ref()
                            {
                                cb(SwitchEvent::Added {
                                    switch_type,
                                    properties,
                                });
                            }
                        }
                    }
                    udev::EventType::Remove => {
                        if let Some(switch_type) = detect_switch_type(&device) {
                            let syspath = event.syspath().to_string_lossy().to_string();

                            if let Ok(cb_lock) = callback.lock()
                                && let Some(cb) = cb_lock.as_ref()
                            {
                                cb(SwitchEvent::Removed {
                                    switch_type,
                                    syspath,
                                });
                            }
                        }
                    }
                    udev::EventType::Change => {
                        if let Some(switch_type) = detect_switch_type(&device) {
                            let properties = extract_switch_properties(&event);
                            let state = read_switch_state(&device, &switch_type);

                            if let Ok(cb_lock) = callback.lock()
                                && let Some(cb) = cb_lock.as_ref()
                            {
                                cb(SwitchEvent::Changed {
                                    switch_type,
                                    state,
                                    properties,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Default for UdevSwitchMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create UdevSwitchMonitor")
    }
}

/// Check if a device is a switch device
fn is_switch_device(device: &udev::Device) -> bool {
    // Check for EV_SW capability
    if let Some(capabilities) = device.attribute_value("capabilities/sw")
        && let Some(cap_str) = capabilities.to_str()
    {
        // If it has any switch capabilities, it's a switch device
        return !cap_str.trim().is_empty() && cap_str.trim() != "0";
    }

    // Also check if name contains "lid" or other switch keywords
    if let Some(name) = device.property_value("NAME")
        && let Some(name_str) = name.to_str()
    {
        let name_lower = name_str.to_lowercase();
        return name_lower.contains("lid")
            || name_lower.contains("switch")
            || name_lower.contains("tablet");
    }

    false
}

/// Detect the type of switch from device properties
fn detect_switch_type(device: &udev::Device) -> Option<SwitchType> {
    // Check device name first
    if let Some(name) = device.property_value("NAME")
        && let Some(name_str) = name.to_str()
    {
        let name_lower = name_str.to_lowercase();

        if name_lower.contains("lid") {
            return Some(SwitchType::Lid);
        }
        if name_lower.contains("tablet") || name_lower.contains("tablet mode") {
            return Some(SwitchType::TabletMode);
        }
        if name_lower.contains("headphone") {
            return Some(SwitchType::Headphone);
        }
        if name_lower.contains("microphone") || name_lower.contains("mic") {
            return Some(SwitchType::Microphone);
        }
        if name_lower.contains("lineout") || name_lower.contains("line out") {
            return Some(SwitchType::LineOut);
        }
        if name_lower.contains("linein") || name_lower.contains("line in") {
            return Some(SwitchType::LineIn);
        }
        if name_lower.contains("video") {
            return Some(SwitchType::VideoOut);
        }
        if name_lower.contains("camera") || name_lower.contains("lens") {
            return Some(SwitchType::CameraLensCover);
        }
        if name_lower.contains("rotate") {
            return Some(SwitchType::RotateLock);
        }
        if name_lower.contains("mute") {
            return Some(SwitchType::Mute);
        }

        return Some(SwitchType::Unknown(name_str.to_string()));
    }

    // If we have switch capabilities but unknown type
    if is_switch_device(device) {
        return Some(SwitchType::Unknown("unidentified".to_string()));
    }

    None
}

/// Read the current state of a switch
fn read_switch_state(device: &udev::Device, switch_type: &SwitchType) -> SwitchState {
    // Try to read state from sysfs attributes
    if let Some(state_attr) = device.attribute_value("state")
        && let Some(state_str) = state_attr.to_str()
    {
        return match state_str.trim() {
            "0" => SwitchState::Off,
            "1" => SwitchState::On,
            _ => SwitchState::Unknown,
        };
    }

    // For lid switches, try to read from ACPI
    if matches!(switch_type, SwitchType::Lid) {
        // The actual state might be in /proc/acpi/button/lid/*/state
        // but we can't easily read that from udev context
        // The change event itself usually indicates the state changed
    }

    SwitchState::Unknown
}

/// Extract properties from a switch udev event
fn extract_switch_properties(event: &udev::Event) -> SwitchProperties {
    let device = event.device();

    let name = device
        .property_value("NAME")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());

    let phys = device
        .property_value("PHYS")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());

    let devnode = device
        .devnode()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string());

    let switch_states = device
        .attribute_value("capabilities/sw")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());

    SwitchProperties {
        syspath: device.syspath().to_string_lossy().to_string(),
        name,
        phys,
        devnode,
        switch_states,
    }
}
