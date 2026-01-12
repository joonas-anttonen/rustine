// Example demonstrating system information gathering
//
// This example shows how to:
// 1. Query all mounted filesystems
// 2. List all block devices (disks and partitions)
// 3. Find unmounted drives that could potentially be mounted

use rustine_desktop::sysinfo;

fn main() {
    println!("=== System Drive Information ===\n");

    // Get mounted drives
    match sysinfo::get_mounted_drives() {
        Ok(mounted) => {
            println!("Mounted Filesystems:");
            for drive in mounted {
                println!(
                    "  {} mounted at {} (type: {})",
                    drive.device, drive.mount_point, drive.fs_type
                );
            }
            println!();
        }
        Err(e) => eprintln!("Error reading mounted drives: {}", e),
    }

    // Get all block devices
    match sysinfo::get_block_devices() {
        Ok(devices) => {
            println!("All Block Devices:");
            for device in devices {
                let size_str = if let Some(size) = device.size {
                    format!("{:.2} GB", size as f64 / 1_000_000_000.0)
                } else {
                    "unknown".to_string()
                };
                let type_str = if device.is_partition {
                    "partition"
                } else {
                    "disk"
                };
                let id_str = device
                    .model
                    .as_ref()
                    .or(device.vendor.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let serial_str = device
                    .serial
                    .as_ref()
                    .map(|s| format!(" serial={}", s))
                    .unwrap_or_default();
                let label_str = device
                    .label
                    .as_ref()
                    .map(|s| format!(" label=\"{}\"", s))
                    .unwrap_or_default();
                let fs_str = device
                    .fs_type
                    .as_ref()
                    .map(|s| format!(" fs={}", s))
                    .unwrap_or_default();
                println!(
                    "  {} - {} ({}) {}{}{}{}",
                    device.path,
                    size_str,
                    type_str,
                    id_str,
                    serial_str,
                    label_str,
                    fs_str
                );
            }
            println!();
        }
        Err(e) => eprintln!("Error reading block devices: {}", e),
    }

    // Get unmounted drives
    match sysinfo::get_unmounted_drives() {
        Ok(unmounted) => {
            println!("Unmounted Block Devices:");
            if unmounted.is_empty() {
                println!("  (all block devices are mounted)");
            } else {
                for drive in unmounted {
                    let size_str = if let Some(size) = drive.size {
                        format!("{:.2} GB", size as f64 / 1_000_000_000.0)
                    } else {
                        "unknown".to_string()
                    };
                    let id_str = drive
                        .model
                        .as_ref()
                        .or(drive.vendor.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let serial_str = drive
                        .serial
                        .as_ref()
                        .map(|s| format!(" serial={}", s))
                        .unwrap_or_default();
                    let label_str = drive
                        .label
                        .as_ref()
                        .map(|s| format!(" label=\"{}\"", s))
                        .unwrap_or_default();
                    let fs_str = drive
                        .fs_type
                        .as_ref()
                        .map(|s| format!(" fs={}", s))
                        .unwrap_or_default();
                    println!(
                        "  {} - {} {}{}{}{}",
                        drive.path,
                        size_str,
                        id_str,
                        serial_str,
                        label_str,
                        fs_str
                    );
                }
            }
        }
        Err(e) => eprintln!("Error finding unmounted drives: {}", e),
    }
}
