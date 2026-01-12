use std::sync::{Arc, Mutex};

/// Represents a device change event detected via udev
#[derive(Debug, Clone)]
pub enum DriveEvent {
    /// A block device was added
    Added {
        /// Device node path (e.g., /dev/sda1)
        device_node: String,
        /// Device properties
        properties: DriveProperties,
    },
    /// A block device was removed
    Removed {
        /// Device node path
        device_node: String,
        /// System path
        syspath: String,
    },
}

/// Properties of a drive device from udev
#[derive(Debug, Clone)]
pub struct DriveProperties {
    /// System path (e.g., /sys/devices/...)
    pub syspath: String,
    /// Device name (e.g., sda1)
    pub devname: Option<String>,
    /// Device type (disk, partition)
    pub devtype: Option<String>,
    /// Filesystem type if available
    pub fs_type: Option<String>,
    /// Filesystem label if available
    pub fs_label: Option<String>,
    /// Size in bytes if available
    pub size: Option<u64>,
    /// ID_BUS (usb, ata, etc.)
    pub id_bus: Option<String>,
    /// Whether this is removable
    pub removable: bool,
    /// Partition number if this is a partition
    pub partition: Option<u32>,
}

/// Callback function type for handling drive events
pub type DriveEventCallback = Box<dyn Fn(DriveEvent) + Send + 'static>;

/// Monitor for block device events via udev
pub struct UdevDriveMonitor {
    callback: Arc<Mutex<Option<DriveEventCallback>>>,
}

impl UdevDriveMonitor {
    /// Create a new udev drive monitor
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(UdevDriveMonitor {
            callback: Arc::new(Mutex::new(None)),
        })
    }

    /// Set a callback to be called when drive events occur
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(DriveEvent) + Send + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(callback));
    }

    /// Start monitoring for drive events
    /// This will block until an error occurs
    pub fn monitor(&self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem("block")?
            .listen()?;

        let callback = Arc::clone(&self.callback);

        loop {
            if let Some(event) = socket.iter().next() {
                let event_type = event.event_type();
                
                match event_type {
                    udev::EventType::Add => {
                        if let Some(device_node) = event.devnode() {
                            let device_node_str = device_node.to_string_lossy().to_string();
                            let properties = extract_properties(&event);
                            
                            if let Ok(cb_lock) = callback.lock() {
                                if let Some(cb) = cb_lock.as_ref() {
                                    cb(DriveEvent::Added {
                                        device_node: device_node_str,
                                        properties,
                                    });
                                }
                            }
                        }
                    }
                    udev::EventType::Remove => {
                        let syspath = event.syspath().to_string_lossy().to_string();
                        let device_node = event
                            .devnode()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| extract_devname_from_syspath(&syspath));
                        
                        if let Ok(cb_lock) = callback.lock() {
                            if let Some(cb) = cb_lock.as_ref() {
                                cb(DriveEvent::Removed {
                                    device_node,
                                    syspath,
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

impl Default for UdevDriveMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create UdevDriveMonitor")
    }
}

/// Extract properties from a udev event
fn extract_properties(event: &udev::Event) -> DriveProperties {
    let device = event.device();
    
    let devname = device.devnode()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());
    
    let devtype = device.property_value("DEVTYPE")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let fs_type = device.property_value("ID_FS_TYPE")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let fs_label = device.property_value("ID_FS_LABEL")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let id_bus = device.property_value("ID_BUS")
        .and_then(|v| v.to_str())
        .map(|s| s.to_string());
    
    let size = device.attribute_value("size")
        .and_then(|v| v.to_str())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|blocks| blocks * 512); // Convert 512-byte blocks to bytes
    
    let removable = device.attribute_value("removable")
        .and_then(|v| v.to_str())
        .map(|s| s == "1")
        .unwrap_or(false);
    
    let partition = device.property_value("PARTN")
        .and_then(|v| v.to_str())
        .and_then(|s| s.parse::<u32>().ok());
    
    DriveProperties {
        syspath: device.syspath().to_string_lossy().to_string(),
        devname,
        devtype,
        fs_type,
        fs_label,
        size,
        id_bus,
        removable,
        partition,
    }
}

/// Extract device name from syspath
fn extract_devname_from_syspath(syspath: &str) -> String {
    syspath
        .split('/')
        .last()
        .map(|s| format!("/dev/{}", s))
        .unwrap_or_else(|| syspath.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_devname_from_syspath() {
        assert_eq!(
            extract_devname_from_syspath("/sys/devices/pci0000:00/0000:00:14.0/usb1/1-3/1-3:1.0/host2/target2:0:0/2:0:0:0/block/sdb"),
            "/dev/sdb"
        );
        assert_eq!(
            extract_devname_from_syspath("/sys/devices/virtual/block/loop0"),
            "/dev/loop0"
        );
    }
}
