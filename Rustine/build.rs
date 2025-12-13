use std::{env, path::PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Unable to get target OS");

    let vulkan_sdk_path = env::var("VULKAN_SDK")
        .expect("VULKAN_SDK environment variable is not set. Please install the Vulkan SDK.");

    let lib_path = match target_os.as_str() {
        "windows" => PathBuf::from(&vulkan_sdk_path).join("Lib"),
        _ => panic!("Unsupported OS for Vulkan SDK linking"),
    };
    println!("cargo:rustc-link-search=native={}", lib_path.display());
}
