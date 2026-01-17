use rustine_explorer::bluetooth_monitor::{BluetoothEvent, UdevBluetoothMonitor};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Bluetooth Device Monitor");
    println!("Listening for Bluetooth device events...");
    println!("Press Ctrl+C to stop\n");

    let mut monitor = UdevBluetoothMonitor::new()?;

    // Set up the event callback
    monitor.on_event(|event| match event {
        BluetoothEvent::Added { address, properties } => {
            println!("\n🔵 Bluetooth Device Added");
            println!("   Address:     {}", address);
            if let Some(name) = &properties.name {
                println!("   Name:        {}", name);
            }
            if let Some(subsystem) = &properties.subsystem {
                println!("   Subsystem:   {}", subsystem);
            }
            if let Some(devtype) = &properties.devtype {
                println!("   Type:        {}", devtype);
            }
            if let Some(driver) = &properties.driver {
                println!("   Driver:      {}", driver);
            }
            if let Some(modalias) = &properties.modalias {
                println!("   Modalias:    {}", modalias);
            }
            println!("   Connected:   {}", properties.connected);
            println!("   Syspath:     {}", properties.syspath);
            io::stdout().flush().ok();
        }
        BluetoothEvent::Removed { address, syspath } => {
            println!("\n🔴 Bluetooth Device Removed");
            println!("   Address:     {}", address);
            println!("   Syspath:     {}", syspath);
            io::stdout().flush().ok();
        }
        BluetoothEvent::Changed { address, properties } => {
            println!("\n🟡 Bluetooth Device Changed");
            println!("   Address:     {}", address);
            if let Some(name) = &properties.name {
                println!("   Name:        {}", name);
            }
            println!("   Connected:   {}", properties.connected);
            println!("   Syspath:     {}", properties.syspath);
            io::stdout().flush().ok();
        }
    });

    // Start monitoring (this will block)
    monitor.monitor()?;

    Ok(())
}
