#![allow(dead_code)]

#[cfg(not(target_os = "windows"))]
pub mod genicam;
#[cfg(not(target_os = "windows"))]
pub mod gige;
#[cfg(not(target_os = "windows"))]
pub mod network;
#[cfg(not(target_os = "windows"))]
pub mod mjpeg;

#[cfg(target_os = "windows")]
pub mod genicam {}
#[cfg(target_os = "windows")]
pub mod gige {}
#[cfg(target_os = "windows")]
pub mod network {}
#[cfg(target_os = "windows")]
pub mod mjpeg {}
