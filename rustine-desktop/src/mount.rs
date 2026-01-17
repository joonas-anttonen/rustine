use std::{
    ffi::CString,
    fs,
    io::Write,
    process::{Command, Stdio},
};

/// Mount a filesystem using sudo with password
pub fn mount_with_sudo(
    device: &str,
    target: &str,
    fstype: &str,
    password: &str,
) -> Result<(), MountError> {
    // Create mount point directory using sudo
    let mut mkdir_status = Command::new("sudo")
        .arg("-S")
        .arg("mkdir")
        .arg("-p")
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(MountError::Other)?;

    if let Some(mut stdin) = mkdir_status.stdin.take() {
        writeln!(stdin, "{}", password).map_err(MountError::Other)?;
    }

    let mkdir_output = mkdir_status.wait_with_output().map_err(MountError::Other)?;

    if !mkdir_output.status.success() {
        return Err(MountError::DirectoryCreationFailed(std::io::Error::other(
            String::from_utf8_lossy(&mkdir_output.stderr).to_string(),
        )));
    }

    // Mount using sudo
    let mut mount_status = Command::new("sudo")
        .arg("-S")
        .arg("mount")
        .arg("-t")
        .arg(fstype)
        .arg(device)
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(MountError::Other)?;

    if let Some(mut stdin) = mount_status.stdin.take() {
        writeln!(stdin, "{}", password).map_err(MountError::Other)?;
    }

    let mount_output = mount_status.wait_with_output().map_err(MountError::Other)?;

    if !mount_output.status.success() {
        let stderr = String::from_utf8_lossy(&mount_output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("incorrect password") {
            return Err(MountError::PermissionDenied);
        }
        return Err(MountError::MountFailed(std::io::Error::other(
            stderr.to_string(),
        )));
    }

    Ok(())
}

/// Unmount a filesystem using sudo with password
pub fn umount_with_sudo(target: &str, password: &str) -> Result<(), MountError> {
    let mut umount_status = Command::new("sudo")
        .arg("-S")
        .arg("umount")
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(MountError::Other)?;

    if let Some(mut stdin) = umount_status.stdin.take() {
        writeln!(stdin, "{}", password).map_err(MountError::Other)?;
    }

    let umount_output = umount_status
        .wait_with_output()
        .map_err(MountError::Other)?;

    if !umount_output.status.success() {
        let stderr = String::from_utf8_lossy(&umount_output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("incorrect password") {
            return Err(MountError::PermissionDenied);
        }
        return Err(MountError::UmountFailed(std::io::Error::other(
            stderr.to_string(),
        )));
    }

    Ok(())
}

/// Mount a filesystem using libc mount syscall
pub fn mount(device: &str, target: &str, fstype: &str) -> Result<(), MountError> {
    // Create mount point if it doesn't exist
    fs::create_dir_all(target).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => MountError::PermissionDenied,
        _ => MountError::DirectoryCreationFailed(e),
    })?;

    let device_c = CString::new(device)
        .map_err(|_| MountError::InvalidPath("device path contains null byte".into()))?;
    let target_c = CString::new(target)
        .map_err(|_| MountError::InvalidPath("target path contains null byte".into()))?;
    let fstype_c = CString::new(fstype)
        .map_err(|_| MountError::InvalidPath("filesystem type contains null byte".into()))?;

    unsafe {
        let ret = libc::mount(
            device_c.as_ptr(),
            target_c.as_ptr(),
            fstype_c.as_ptr(),
            0,                // flags
            std::ptr::null(), // data
        );

        if ret != 0 {
            let io_error = std::io::Error::last_os_error();
            return Err(categorize_mount_error(io_error));
        }
    }

    Ok(())
}

/// Unmount a filesystem using libc umount syscall
pub fn umount(target: &str) -> Result<(), MountError> {
    let target_c = CString::new(target)
        .map_err(|_| MountError::InvalidPath("target path contains null byte".into()))?;

    unsafe {
        let ret = libc::umount(target_c.as_ptr());
        if ret != 0 {
            let io_error = std::io::Error::last_os_error();
            return Err(categorize_umount_error(io_error));
        }
    }

    Ok(())
}

/// Errors that can occur during mount operations
#[derive(Debug)]
pub enum MountError {
    /// Invalid path provided (contains null bytes)
    InvalidPath(String),
    /// Permission denied when performing mount/unmount
    PermissionDenied,
    /// Device not found
    DeviceNotFound,
    /// Mount point or target doesn't exist
    NotFound,
    /// Filesystem type not supported
    UnsupportedFilesystem,
    /// Directory creation failed
    DirectoryCreationFailed(std::io::Error),
    /// Mount syscall failed
    MountFailed(std::io::Error),
    /// Unmount syscall failed
    UmountFailed(std::io::Error),
    /// Other mount-related error
    Other(std::io::Error),
}

/// Categorize mount errors from OS error codes
fn categorize_mount_error(err: std::io::Error) -> MountError {
    use std::io::ErrorKind;
    match err.kind() {
        ErrorKind::PermissionDenied => MountError::PermissionDenied,
        ErrorKind::NotFound => MountError::DeviceNotFound,
        ErrorKind::Unsupported => MountError::UnsupportedFilesystem,
        _ => {
            // Check raw OS error code for more specific errors
            if let Some(code) = err.raw_os_error() {
                match code {
                    libc::ENODEV => MountError::DeviceNotFound,
                    libc::ENOENT => MountError::NotFound,
                    libc::EACCES | libc::EPERM => MountError::PermissionDenied,
                    libc::ENOTSUP => MountError::UnsupportedFilesystem,
                    _ => MountError::MountFailed(err),
                }
            } else {
                MountError::MountFailed(err)
            }
        }
    }
}

/// Categorize umount errors from OS error codes
fn categorize_umount_error(err: std::io::Error) -> MountError {
    use std::io::ErrorKind;
    match err.kind() {
        ErrorKind::PermissionDenied => MountError::PermissionDenied,
        ErrorKind::NotFound => MountError::NotFound,
        _ => {
            // Check raw OS error code for more specific errors
            if let Some(code) = err.raw_os_error() {
                match code {
                    libc::ENOENT => MountError::NotFound,
                    libc::EACCES | libc::EPERM => MountError::PermissionDenied,
                    _ => MountError::UmountFailed(err),
                }
            } else {
                MountError::UmountFailed(err)
            }
        }
    }
}

impl std::fmt::Display for MountError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MountError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            MountError::PermissionDenied => {
                write!(f, "Permission denied: requires root privileges")
            }
            MountError::DeviceNotFound => write!(f, "Device not found"),
            MountError::NotFound => write!(f, "Mount point not found"),
            MountError::UnsupportedFilesystem => {
                write!(f, "Unsupported or unrecognized filesystem type")
            }
            MountError::DirectoryCreationFailed(err) => {
                write!(f, "Failed to create mount point: {}", err)
            }
            MountError::MountFailed(err) => write!(f, "Mount failed: {}", err),
            MountError::UmountFailed(err) => write!(f, "Unmount failed: {}", err),
            MountError::Other(err) => write!(f, "Mount operation failed: {}", err),
        }
    }
}

impl std::error::Error for MountError {}
