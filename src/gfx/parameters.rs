#![allow(dead_code)]

use super::Version;

#[derive(Debug)]
pub struct ApiParameters {
    pub enable_debugging: bool,
    pub app_version: Version,
    pub app_engine_version: Version,
    pub required_api_version: Version,
    pub app_name: String,
    pub app_engine_name: String,
}