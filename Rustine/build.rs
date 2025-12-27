use std::{env, path::Path, path::PathBuf};

fn main() {
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Unable to get target OS");
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    //let target_dir = project_dir.join("target").join(&profile);

    let vulkan_sdk_path = env::var("VULKAN_SDK")
        .expect("VULKAN_SDK environment variable is not set. Please install the Vulkan SDK.");

    let lib_path = match target_os.as_str() {
        "windows" => PathBuf::from(&vulkan_sdk_path).join("Lib"),
        _ => panic!("Unsupported OS for Vulkan SDK linking"),
    };
    println!("cargo:rustc-link-search=native={}", lib_path.display());

    build_glfw(&project_dir, &target_os);
    build_vma_interop(&project_dir, &target_os);

    // If on Windows, link the appropriate CRT libraries
    if target_os == "windows" {
        match profile.as_str() {
            "debug" => {
                println!("cargo:rustc-link-lib=dylib=vcruntimed");
                println!("cargo:rustc-link-lib=dylib=msvcrtd");
            }
            _ => {
                println!("cargo:rustc-link-lib=dylib=vcruntime");
                println!("cargo:rustc-link-lib=dylib=msvcrt");
            }
        };
    }
}

fn build_vma_interop(project_dir: &Path, target_os: &str) {
    let generator = match target_os {
        "windows" => "Ninja",
        _ => "Ninja",
    };

    // Build
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("vma_interop"))
        .generator(generator)
        .always_configure(true)
        .build();

    // Link
    println!(
        "cargo:rustc-link-search=native={}",
        destination_dir.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static={}", "rustine_vma");
}

fn build_glfw(project_dir: &Path, target_os: &str) {
    let generator = match target_os {
        "windows" => "Ninja",
        _ => "Ninja",
    };

    // Build
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("glfw"))
        .generator(generator)
        .define("GLFW_BUILD_EXAMPLES", "OFF")
        .define("GLFW_BUILD_TESTS", "OFF")
        .define("GLFW_BUILD_DOCS", "OFF")
        .always_configure(true)
        .build();

    // Link
    println!(
        "cargo:rustc-link-search=native={}",
        destination_dir.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static={}", "glfw3");

    // Link platform-specific dependencies
    match target_os {
        "windows" => {
            println!("cargo:rustc-link-lib=dylib=user32");
            println!("cargo:rustc-link-lib=dylib=gdi32");
            println!("cargo:rustc-link-lib=dylib=shell32");
            println!("cargo:rustc-link-lib=dylib=kernel32");
            println!("cargo:rustc-link-lib=dylib=opengl32");
        }
        _ => {}
    };
}
