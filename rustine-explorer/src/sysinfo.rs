use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

/// Information about a mounted filesystem
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount point (e.g., /home)
    pub mount_point: String,
    /// Filesystem type (e.g., ext4, ntfs, btrfs)
    pub fs_type: String,
    /// Mount options
    pub options: Vec<String>,
}

/// Represents a block device
#[derive(Debug, Clone)]
pub struct BlockDevice {
    /// Device name (e.g., sda, sda1, nvme0n1, nvme0n1p1)
    pub _name: String,
    /// Full device path (e.g., /dev/sda1)
    pub path: String,
    /// Size in bytes
    pub size: Option<u64>,
    /// Whether this is a partition (true) or a whole disk (false)
    pub is_partition: bool,
    /// Probed filesystem type (best-effort, may be None)
    pub fs_type: Option<String>,
    /// Vendor string (if available)
    pub vendor: Option<String>,
    /// Model string (if available)
    pub model: Option<String>,
    /// Serial number (if available)
    pub serial: Option<String>,
    /// World-wide identifier (if available)
    pub wwid: Option<String>,
    /// Volume label for partitions (if available)
    pub label: Option<String>,
    /// Mount information if this device is currently mounted
    pub mount_info: Option<MountInfo>,
}

/// Returns a map of device paths to their mount information
fn get_mount_map() -> io::Result<std::collections::HashMap<String, MountInfo>> {
    let mut mounts = std::collections::HashMap::new();
    let file = fs::File::open("/proc/mounts")?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 4 {
            let device = parts[0];
            let mount_point = parts[1];
            let fs_type = parts[2];
            let options_str = parts[3];

            // Skip pseudo-filesystems typically
            if device.starts_with('/') {
                mounts.insert(
                    device.to_string(),
                    MountInfo {
                        mount_point: unescape_mount_point(mount_point),
                        fs_type: fs_type.to_string(),
                        options: options_str.split(',').map(|s| s.to_string()).collect(),
                    },
                );
            }
        }
    }

    Ok(mounts)
}

/// Returns all block devices on the system
pub fn get_block_devices() -> io::Result<Vec<BlockDevice>> {
    let mut devices = Vec::new();
    let sys_block_path = Path::new("/sys/block");

    let label_map = collect_partition_labels();
    let mount_map = get_mount_map()?;

    if !sys_block_path.exists() {
        return Ok(devices);
    }

    for entry in fs::read_dir(sys_block_path)? {
        let entry = entry?;
        let device_name = entry.file_name();
        let device_name_str = device_name.to_string_lossy();

        // Skip loop devices and ram devices by default
        if device_name_str.starts_with("loop") || device_name_str.starts_with("ram") {
            continue;
        }

        let device_path = entry.path();
        let size = read_block_device_size(&device_path);
        let metadata = read_device_metadata(&device_path);
        let dev_path = format!("/dev/{}", device_name_str);
        let fs_type = probe_filesystem(&dev_path);
        let mount_info = mount_map.get(&dev_path).cloned();

        devices.push(BlockDevice {
            _name: device_name_str.to_string(),
            path: dev_path,
            size,
            is_partition: false,
            fs_type,
            vendor: metadata.vendor,
            model: metadata.model,
            serial: metadata.serial,
            wwid: metadata.wwid,
            label: None,
            mount_info,
        });

        // Check for partitions
        if let Ok(partitions) =
            collect_partitions(&device_path, &device_name_str, &label_map, &mount_map)
        {
            devices.extend(partitions);
        }
    }

    Ok(devices)
}

/// Returns only block devices that are currently mounted
pub fn get_mounted_devices() -> io::Result<Vec<BlockDevice>> {
    let all_devices = get_block_devices()?;
    Ok(all_devices
        .into_iter()
        .filter(|dev| dev.mount_info.is_some())
        .collect())
}

/// Returns only block devices that are not currently mounted
pub fn get_unmounted_devices() -> io::Result<Vec<BlockDevice>> {
    let all_devices = get_block_devices()?;
    Ok(all_devices
        .into_iter()
        .filter(|dev| dev.mount_info.is_none())
        .collect())
}

fn collect_partitions(
    device_path: &Path,
    device_name: &str,
    label_map: &std::collections::HashMap<String, String>,
    mount_map: &std::collections::HashMap<String, MountInfo>,
) -> io::Result<Vec<BlockDevice>> {
    let mut partitions = Vec::new();

    for entry in fs::read_dir(device_path)? {
        let entry = entry?;
        let partition_name = entry.file_name();
        let partition_name_str = partition_name.to_string_lossy();

        // Partitions start with the device name
        if partition_name_str.starts_with(device_name) {
            let partition_path = entry.path();
            let size = read_block_device_size(&partition_path);
            let metadata = read_device_metadata(&partition_path);
            let dev_path = format!("/dev/{}", partition_name_str);
            let label = label_map.get(&dev_path).cloned();
            let fs_type = probe_filesystem(&dev_path);
            let mount_info = mount_map.get(&dev_path).cloned();

            partitions.push(BlockDevice {
                _name: partition_name_str.to_string(),
                path: dev_path,
                size,
                is_partition: true,
                fs_type,
                vendor: metadata.vendor,
                model: metadata.model,
                serial: metadata.serial,
                wwid: metadata.wwid,
                label,
                mount_info,
            });
        }
    }

    Ok(partitions)
}

fn read_block_device_size(device_path: &Path) -> Option<u64> {
    let size_file = device_path.join("size");
    let size_str = fs::read_to_string(size_file).ok()?;
    let blocks = size_str.trim().parse::<u64>().ok()?;
    // Block size in /sys/block is 512 bytes
    Some(blocks * 512)
}

struct DeviceMetadata {
    vendor: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    wwid: Option<String>,
}

fn read_device_metadata(device_path: &Path) -> DeviceMetadata {
    let device_dir = device_path.join("device");

    DeviceMetadata {
        vendor: read_trimmed(device_dir.join("vendor")),
        model: read_trimmed(device_dir.join("model")),
        serial: read_trimmed(device_dir.join("serial")),
        wwid: read_trimmed(device_dir.join("wwid")),
    }
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn collect_partition_labels() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let by_label_path = Path::new("/dev/disk/by-label");

    if !by_label_path.exists() {
        return map;
    }

    if let Ok(entries) = fs::read_dir(by_label_path) {
        for entry in entries.flatten() {
            let label = entry.file_name().to_string_lossy().to_string();
            if let Ok(target) = fs::read_link(entry.path()) {
                let mut full_path = by_label_path.to_path_buf();
                full_path.pop(); // remove by-label
                full_path.push(target);
                if let Ok(canonical) = full_path.canonicalize()
                    && let Some(dev_name) = canonical.file_name().and_then(|n| n.to_str())
                {
                    let dev_path = format!("/dev/{}", dev_name);
                    map.insert(dev_path, label);
                }
            }
        }
    }

    map
}

fn probe_filesystem(dev_path: &str) -> Option<String> {
    use std::process::Command;

    // Preferred: lsblk (usually non-root)
    if let Ok(output) = Command::new("lsblk")
        .args(["-d", "-no", "FSTYPE", dev_path])
        .output() && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(first) = s.lines().map(str::trim).find(|l| !l.is_empty()) {
            return Some(first.to_string());
        }
    }

    None
}

/// Unescapes octal sequences in mount point paths from /proc/mounts
fn unescape_mount_point(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Try to read three octal digits
            let mut octal = String::new();
            for _ in 0..3 {
                if let Some(digit) = chars.next() {
                    if digit.is_ascii_digit() {
                        octal.push(digit);
                    } else {
                        result.push(digit);
                        break;
                    }
                } else {
                    break;
                }
            }

            if octal.len() == 3 {
                if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                    result.push(byte as char);
                } else {
                    result.push('\\');
                    result.push_str(&octal);
                }
            } else if !octal.is_empty() {
                result.push_str(&octal);
            }
        } else {
            result.push(c);
        }
    }

    result
}
