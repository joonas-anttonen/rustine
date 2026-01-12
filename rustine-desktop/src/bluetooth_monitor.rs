use std::sync::{Arc, Mutex};

/// Represents a Bluetooth device change event detected via udev
#[derive(Debug, Clone)]
pub enum BluetoothEvent {
    /// A Bluetooth device was added (connected/paired)
    Added {
        /// Device address (MAC address)
        address: String,
        /// Device properties
        properties: BluetoothProperties,
    },
    /// A Bluetooth device was removed (disconnected)
    Removed {
        /// Device address (MAC address)
        address: String,
        /// System path
        syspath: String,
    },
    /// A Bluetooth device state changed
    Changed {
        /// Device address (MAC address)
        address: String,
        /// Device properties
        properties: BluetoothProperties,
    },
}

/// Properties of a Bluetooth device from udev
#[derive(Debug, Clone)]
pub struct BluetoothProperties {
    /// System path (e.g., /sys/devices/...)
    pub syspath: String,
    /// Device name/alias
    pub name: Option<String>,
    /// Device type (e.g., input, audio)
    pub devtype: Option<String>,
    /// Subsystem
    pub subsystem: Option<String>,
    /// Driver
    pub driver: Option<String>,
    /// Modalias
    pub modalias: Option<String>,
    /// Whether device is connected
    pub connected: bool,
}

/// Callback function type for handling Bluetooth events
pub type BluetoothEventCallback = Box<dyn Fn(BluetoothEvent) + Send + 'static>;

/// Monitor for Bluetooth device events via udev
pub struct UdevBluetoothMonitor {
    callback: Arc<Mutex<Option<BluetoothEventCallback>>>,
}

impl UdevBluetoothMonitor {
    /// Create a new udev Bluetooth monitor
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(UdevBluetoothMonitor {
            callback: Arc::new(Mutex::new(None)),
        })
    }

    /// Set a callback to be called when Bluetooth events occur
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(BluetoothEvent) + Send + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(callback));
    }

    /// Start monitoring for Bluetooth events
    /// This will block until an error occurs
    pub fn monitor(&self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("bluetooth")?
            .match_subsystem("input")?
            .listen()?;

        let callback = Arc::clone(&self.callback);

        loop {
            if let Some(event) = socket.iter().next() {
                let event_type = event.event_type();
                let device = event.device();
                
                // Filter for Bluetooth-related devices
                let subsystem = device.subsystem().and_then(|s| s.to_str());
                if !is_bluetooth_related(subsystem) {
                    continue;
                }
                
                match event_type {
                    udev::EventType::Add => {
                        if let Some(address) = extract_bt_address(&event) {
                            let properties = extract_bt_properties(&event);
                            
                            if let Ok(cb_lock) = callback.lock() {
                                if let Some(cb) = cb_lock.as_ref() {
                                    cb(BluetoothEvent::Added {
                                        address,
                                        properties,
                                    });
                                }
                            }
                        }
                    }
                    udev::EventType::Remove => {
                        let syspath = event.syspath().to_string_lossy().to_string();
                        if let Some(address) = extract_bt_address(&event) {
                            if let Ok(cb_lock) = callback.lock() {
                                if let Some(cb) = cb_lock.as_ref() {
                                    cb(BluetoothEvent::Removed {
                                        address,
                                        syspath,
                                    });
                                }
                            }
                        }
                    }
                    udev::EventType::Change => {
                        if let Some(address) = extract_bt_address(&event) {
                            let properties = extract_bt_properties(&event);
                            
                            if let Ok(cb_lock) = callback.lock() {
                                if let Some(cb) = cb_lock.as_ref() {
                                    cb(BluetoothEvent::Changed {
                                        address,
                                        properties,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Default for UdevBluetoothMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create UdevBluetoothMonitor")
    }
}

/// Check if the subsystem is Bluetooth-related
fn is_bluetooth_related(subsystem: Option<&str>) -> bool {
    matches!(subsystem, Some("bluetooth") | Some("input"))
}

/// Extract Bluetooth address from a udev event
fn extract_bt_address(event: &udev::Event) -> Option<String> {
    let device = event.device();
    let syspath = device.syspath().to_string_lossy().to_string();
    
    // Look for MAC address pattern in syspath
    // Example: /sys/devices/virtual/input/input45/0005:046D:B023.0012
    // or /sys/devices/pci0000:00/.../bluetooth/hci0/hci0:256/0005:046D:B023.0012
    
    // Try to find a Bluetooth address in parent devices
    let mut current = Some(device);
    while let Some(dev) = current {
        if let Some(addr) = dev.property_value("UNIQ") {
            if let Some(addr_str) = addr.to_str() {
                if is_valid_bt_address(addr_str) {
                    return Some(addr_str.to_string());
                }
            }
        }
        
        // Check syspath for address pattern
        if let Some(sysname) = dev.sysname().to_str() {
            if is_valid_bt_address(sysname) {
                return Some(sysname.to_string());
            }
        }
        
        current = dev.parent();
    }
    
    // Fallback: extract from syspath
    for part in syspath.split('/') {
        if is_valid_bt_address(part) {
            return Some(part.to_string());
        }
    }
    
    // Last resort: use the last component of syspath
    Some(
        syspath
            .split('/')
            .last()
            .unwrap_or("unknown")
            .to_string()
    )
}

/// Check if a string looks like a Bluetooth MAC address
fn is_valid_bt_address(s: &str) -> bool {
    // XX:XX:XX:XX:XX:XX format
    if s.len() == 17 {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 6 {
            return parts.iter().all(|p| {
                p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit())
            });
        }
    }
    false
}

/// Extract properties from a Bluetooth udev event
fn extract_bt_properties(event: &udev::Event) -> BluetoothProperties {
    let device = event.device();
    
    let name = device.property_value("NAME")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string())
        .or_else(|| {
            device.sysname()
                .to_str()
                .map(|s| s.to_string())
        });
    
    let devtype = device.devtype()
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let subsystem = device.subsystem()
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let driver = device.driver()
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let modalias = device.property_value("MODALIAS")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let connected = device.attribute_value("connected")
        .and_then(|v| v.to_str())
        .map(|s| s == "1")
        .unwrap_or(false);
    
    BluetoothProperties {
        syspath: device.syspath().to_string_lossy().to_string(),
        name,
        devtype,
        subsystem,
        driver,
        modalias,
        connected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_bt_address() {
        assert!(is_valid_bt_address("00:1A:7D:DA:71:13"));
        assert!(is_valid_bt_address("A4:C1:38:F2:9D:8E"));
        assert!(!is_valid_bt_address("00:1A:7D:DA:71"));
        assert!(!is_valid_bt_address("not-a-mac"));
        assert!(!is_valid_bt_address("00:1A:7D:DA:71:ZZ"));
    }

    #[test]
    fn test_is_bluetooth_related() {
        assert!(is_bluetooth_related(Some("bluetooth")));
        assert!(is_bluetooth_related(Some("input")));
        assert!(!is_bluetooth_related(Some("block")));
        assert!(!is_bluetooth_related(None));
    }
}
