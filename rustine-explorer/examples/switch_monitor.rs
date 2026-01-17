use rustine_explorer::switch_monitor::{SwitchEvent, SwitchType, SwitchState, UdevSwitchMonitor};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Switch Monitor");
    println!("Listening for switch events (lid, tablet mode, etc.)...");
    println!("Press Ctrl+C to stop\n");

    let mut monitor = UdevSwitchMonitor::new()?;

    // Set up the event callback
    monitor.on_event(|event| match event {
        SwitchEvent::Added {
            switch_type,
            properties,
        } => {
            println!("\n➕ Switch Added: {:?}", switch_type);
            if let Some(name) = &properties.name {
                println!("   Name:        {}", name);
            }
            if let Some(devnode) = &properties.devnode {
                println!("   Device:      {}", devnode);
            }
            if let Some(phys) = &properties.phys {
                println!("   Physical:    {}", phys);
            }
            if let Some(caps) = &properties.switch_states {
                println!("   Capabilities: {}", caps);
            }
            println!("   Syspath:     {}", properties.syspath);
            io::stdout().flush().ok();
        }
        SwitchEvent::Removed {
            switch_type,
            syspath,
        } => {
            println!("\n➖ Switch Removed: {:?}", switch_type);
            println!("   Syspath:     {}", syspath);
            io::stdout().flush().ok();
        }
        SwitchEvent::Changed {
            switch_type,
            state,
            properties,
        } => {
            let state_icon = match state {
                SwitchState::On => "🔒",
                SwitchState::Off => "🔓",
                SwitchState::Unknown => "❓",
            };
            
            let state_text = match state {
                SwitchState::On => "ON/CLOSED",
                SwitchState::Off => "OFF/OPEN",
                SwitchState::Unknown => "UNKNOWN",
            };
            
            println!("\n{} Switch Changed: {:?}", state_icon, switch_type);
            println!("   State:       {}", state_text);
            if let Some(name) = &properties.name {
                println!("   Name:        {}", name);
            }
            if let Some(devnode) = &properties.devnode {
                println!("   Device:      {}", devnode);
            }
            
            // Special messages for common switches
            match switch_type {
                SwitchType::Lid => {
                    match state {
                        SwitchState::On => println!("   💤 Laptop lid CLOSED"),
                        SwitchState::Off => println!("   ☀️  Laptop lid OPEN"),
                        _ => {}
                    }
                }
                SwitchType::TabletMode => {
                    match state {
                        SwitchState::On => println!("   📱 Tablet mode ENABLED"),
                        SwitchState::Off => println!("   💻 Laptop mode ENABLED"),
                        _ => {}
                    }
                }
                _ => {}
            }
            
            io::stdout().flush().ok();
        }
    });

    // Start monitoring (this will block)
    monitor.monitor()?;

    Ok(())
}
