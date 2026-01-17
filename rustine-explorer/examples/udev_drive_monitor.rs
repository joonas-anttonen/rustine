use rustine_explorer::udev_monitor::{DriveEvent, UdevDriveMonitor};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting udev Drive Monitor");
    println!("Listening for block device add/remove events...");
    println!("Press Ctrl+C to stop\n");

    let mut monitor = UdevDriveMonitor::new()?;

    // Set up the event callback
    monitor.on_event(|event| match event {
        DriveEvent::Added {
            device_node,
            properties,
        } => {
            println!("\n📱 Device Added: {}", device_node);
            if let Some(devname) = &properties.devname {
                println!("   Name:        {}", devname);
            }
            if let Some(devtype) = &properties.devtype {
                println!("   Type:        {}", devtype);
            }
            if let Some(id_bus) = &properties.id_bus {
                println!("   Bus:         {}", id_bus);
            }
            println!("   Removable:   {}", properties.removable);
            if let Some(size) = properties.size {
                println!("   Size:        {} bytes ({:.2} GB)", size, size as f64 / 1_000_000_000.0);
            }
            if let Some(fs_type) = &properties.fs_type {
                println!("   FS Type:     {}", fs_type);
            }
            if let Some(fs_label) = &properties.fs_label {
                println!("   FS Label:    {}", fs_label);
            }
            if let Some(partition) = properties.partition {
                println!("   Partition:   {}", partition);
            }
            println!("   Syspath:     {}", properties.syspath);
            io::stdout().flush().ok();
        }
        DriveEvent::Removed {
            device_node,
            syspath,
        } => {
            println!("\n🗑️  Device Removed: {}", device_node);
            println!("   Syspath:     {}", syspath);
            io::stdout().flush().ok();
        }
    });

    // Start monitoring (this will block)
    monitor.monitor()?;

    Ok(())
}
