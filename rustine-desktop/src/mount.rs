use std::{ffi::CString, fs};

/// Mount a filesystem using libc mount syscall
pub fn mount(device: &str, target: &str, fstype: &str) -> Result<(), MountError> {
    // Create mount point if it doesn't exist
    fs::create_dir_all(target)
        .map_err(|e| MountError::InvalidPath(format!("Failed to create mount point: {}", e)))?;

    let device_c = CString::new(device)
        .map_err(|_| MountError::InvalidPath("device path contains null byte".into()))?;
    let target_c = CString::new(target)
        .map_err(|_| MountError::InvalidPath("target path contains null byte".into()))?;
    let fstype_c = CString::new(fstype)
        .map_err(|_| MountError::InvalidPath("fstype contains null byte".into()))?;

    /*

    let mut child = Command::new("sudo")
        .args(&["-S", "mount", "-t", fstype, device, target])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| MountError::MountFailed(e))?;
     */

    unsafe {
        let ret = libc::mount(
            device_c.as_ptr(),
            target_c.as_ptr(),
            fstype_c.as_ptr(),
            0,  // flags
            std::ptr::null(),  // data
        );

        if ret != 0 {
            return Err(MountError::MountFailed(std::io::Error::last_os_error()));
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
            return Err(MountError::UmountFailed(std::io::Error::last_os_error()));
        }
    }

    Ok(())
}

/// Errors that can occur during mount operations
#[derive(Debug)]
pub enum MountError {
    /// Invalid path provided (contains null bytes)
    InvalidPath(String),
    /// Mount syscall failed
    MountFailed(std::io::Error),
    /// Unmount syscall failed
    UmountFailed(std::io::Error),
}

impl std::fmt::Display for MountError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MountError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            MountError::MountFailed(err) => write!(f, "Mount failed: {}", err),
            MountError::UmountFailed(err) => write!(f, "Unmount failed: {}", err),
        }
    }
}

impl std::error::Error for MountError {}
