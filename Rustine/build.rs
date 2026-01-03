use std::{env, path::Path, path::PathBuf, process::Command};

fn main() {
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Unable to get target OS");
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    println!("cargo:rerun-if-env-changed=VULKAN_SDK");

    match target_os.as_str() {
        "windows" => {
            let vulkan_sdk_path = env::var("VULKAN_SDK").expect(
                "VULKAN_SDK environment variable is not set. Please install the Vulkan SDK.",
            );
            let lib_path = PathBuf::from(&vulkan_sdk_path).join("Lib");
            println!("cargo:rustc-link-search=native={}", lib_path.display());
        }
        "linux" => {
            if let Ok(vulkan_sdk_path) = env::var("VULKAN_SDK") {
                let lib_path = PathBuf::from(&vulkan_sdk_path).join("lib");
                println!("cargo:rustc-link-search=native={}", lib_path.display());
            }
            println!("cargo:rustc-link-lib=dylib=vulkan");
        }
        _ => panic!("Unsupported OS for Vulkan SDK linking"),
    };

    build_glfw(&project_dir, &out_dir, &target_os);
    build_vma_interop(&project_dir, &out_dir, &target_os);
    
    if target_os == "linux" {
        build_rustine_wl(&project_dir, &out_dir);
    }

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

fn build_vma_interop(project_dir: &Path, out_dir: &Path, target_os: &str) {
    let generator = choose_generator(target_os);

    // Build
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("vma_interop"))
        .generator(generator)
        .out_dir(out_dir.join("vma_interop"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);

    // Link
    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.display()
    );
    println!("cargo:rustc-link-lib=static={}", "rustine_vma");
}

fn build_glfw(project_dir: &Path, out_dir: &Path, target_os: &str) {
    let generator = choose_generator(target_os);

    // Build
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("glfw"))
        .generator(generator)
        .define("GLFW_BUILD_EXAMPLES", "OFF")
        .define("GLFW_BUILD_TESTS", "OFF")
        .define("GLFW_BUILD_DOCS", "OFF")
        .define("GLFW_BUILD_WAYLAND", "ON")
        .define("GLFW_BUILD_X11", "OFF")
        .out_dir(out_dir.join("glfw"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);

    // Link
    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.display()
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
        "linux" => {
            println!("cargo:rustc-link-lib=dylib=wayland-client");
            println!("cargo:rustc-link-lib=dylib=wayland-egl");
            println!("cargo:rustc-link-lib=dylib=wayland-cursor");
            println!("cargo:rustc-link-lib=dylib=xkbcommon");
            println!("cargo:rustc-link-lib=dylib=udev");
            println!("cargo:rustc-link-lib=dylib=dl");
            println!("cargo:rustc-link-lib=dylib=pthread");
            println!("cargo:rustc-link-lib=dylib=m");
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
        _ => {}
    };
}

fn choose_generator(target_os: &str) -> &'static str {
    if target_os == "windows" {
        return "Ninja";
    }

    if Command::new("ninja").arg("--version").output().is_ok() {
        "Ninja"
    } else {
        "Unix Makefiles"
    }
}

fn prefer_lib64(destination_dir: &Path) -> PathBuf {
    let lib64 = destination_dir.join("lib64");
    if lib64.exists() {
        lib64
    } else {
        destination_dir.join("lib")
    }
}

fn build_rustine_wl(project_dir: &Path, out_dir: &Path) {
    let source_dir = project_dir.join("ext").join("rustine-wl");
    let build_dir = out_dir.join("rustine-wl");

    // Setup meson build
    let setup_status = Command::new("meson")
        .arg("setup")
        .arg(&build_dir)
        .arg("--wipe")
        .current_dir(&source_dir)
        .status()
        .expect("Failed to run meson setup for rustine-wl");

    if !setup_status.success() {
        panic!("meson setup failed for rustine-wl");
    }

    // Compile with meson
    let compile_status = Command::new("meson")
        .arg("compile")
        .arg("-C")
        .arg(&build_dir)
        .status()
        .expect("Failed to run meson compile for rustine-wl");

    if !compile_status.success() {
        panic!("meson compile failed for rustine-wl");
    }

    // Link the static library
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.display()
    );
    println!("cargo:rustc-link-lib=static=rustine-wl");

    // Add wayland-client dependency (required by rustine-wl)
    println!("cargo:rustc-link-lib=dylib=wayland-client");

    // Tell cargo to rerun if the source changes
    println!("cargo:rerun-if-changed={}", source_dir.join("rustine-wl.cpp").display());
    println!("cargo:rerun-if-changed={}", source_dir.join("rustine-wl.hpp").display());
    println!("cargo:rerun-if-changed={}", source_dir.join("meson.build").display());
}
