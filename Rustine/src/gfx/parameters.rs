#![allow(dead_code)]

use super::Version;

#[derive(Debug)]
pub enum Platform {
    Windows,
    Wayland,
    X11,
    MacOS,
    Unknown,
}

/// Parameters for initializing the graphics API.
#[derive(Debug)]
pub struct ApiParameters {
    pub enable_debugging: bool,
    pub platform: Platform,
    pub required_api_version: Version,
    pub app_version: Version,
    pub app_engine_version: Version,
    pub app_name: String,
    pub app_engine_name: String,
}
