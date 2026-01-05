use std::{env, fs, path::Path, path::PathBuf, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
    Windows,
    Linux,
}

impl Os {
    fn from_env() -> Self {
        let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Unable to get target OS");
        match target_os.as_str() {
            "windows" => Os::Windows,
            "linux" => Os::Linux,
            os => panic!("Unsupported target OS: {}", os),
        }
    }
}

/// println!("cargo:rerun-if-changed={}"...
fn rerun_if_changed<P: AsRef<Path>>(path: P) {
    println!("cargo:rerun-if-changed={}", path.as_ref().display());
}

/// println!("cargo:rustc-link-lib=static={}"...
fn link_static(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

/// println!("cargo:rustc-link-lib=dylib={}"...
fn link_dynamic(lib: &str) {
    println!("cargo:rustc-link-lib=dylib={}", lib);
}

/// println!("cargo:rustc-link-search=native={}"...
fn link_search<P: AsRef<Path>>(path: P) {
    println!("cargo:rustc-link-search=native={}", path.as_ref().display());
}

fn choose_generator(target_os: Os) -> &'static str {
    if target_os == Os::Windows {
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

fn main() {
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    let target_os = Os::from_env();
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let generator = choose_generator(target_os);

    match target_os {
        Os::Windows => {
            let vulkan_sdk_path = env::var("VULKAN_SDK").expect(
                "VULKAN_SDK environment variable is not set. Please install the Vulkan SDK.",
            );
            let lib_path = PathBuf::from(&vulkan_sdk_path).join("Lib");
            link_search(&lib_path);
        }
        Os::Linux => {
            if let Ok(vulkan_sdk_path) = env::var("VULKAN_SDK") {
                let lib_path = PathBuf::from(&vulkan_sdk_path).join("lib");
                link_search(&lib_path);
            }
            link_dynamic("vulkan");
        }
    }

    build_glfw(&project_dir, &out_dir, target_os, generator);
    build_rustine_vma(&project_dir, &out_dir, target_os, generator);
    build_rustine_webp(&project_dir, &out_dir, target_os, generator);
    build_rustine_dxc(&project_dir, &out_dir, target_os, generator);

    if target_os == Os::Linux {
        build_rustine_wl(&project_dir, &out_dir, generator);
    }

    // If on Windows, link the appropriate CRT libraries
    if target_os == Os::Windows {
        match profile.as_str() {
            "debug" => {
                link_dynamic("vcruntimed");
                link_dynamic("msvcrtd");
            }
            _ => {
                link_dynamic("vcruntime");
                link_dynamic("msvcrt");
            }
        };
    }
}

fn build_rustine_webp(project_dir: &Path, out_dir: &Path, _target_os: Os, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-webp"))
        .generator(generator)
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("WEBP_ENABLE_SIMD", "ON")
        .define("WEBP_USE_THREAD", "ON")
        .out_dir(out_dir.join("rustine-webp"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine_webp");
    link_static("webp");
    link_static("webpdemux");

    let rustine_webp_dir = project_dir.join("ext").join("rustine-webp");
    rerun_if_changed(rustine_webp_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_webp_dir.join("rustine-webp.cpp"));
    rerun_if_changed(rustine_webp_dir.join("rustine-webp.hpp"));
    rerun_if_changed(project_dir.join("ext").join("libwebp"));
}

fn build_rustine_vma(project_dir: &Path, out_dir: &Path, _target_os: Os, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-vma"))
        .generator(generator)
        .out_dir(out_dir.join("rustine-vma"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-vma");
}

fn build_glfw(project_dir: &Path, out_dir: &Path, target_os: Os, generator: &'static str) {
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
    link_search(&lib_dir);
    link_static("glfw3");

    // Link platform-specific dependencies
    match target_os {
        Os::Windows => {
            link_dynamic("user32");
            link_dynamic("gdi32");
            link_dynamic("shell32");
            link_dynamic("kernel32");
            link_dynamic("opengl32");
        }
        Os::Linux => {
            link_dynamic("wayland-client");
            link_dynamic("wayland-egl");
            link_dynamic("wayland-cursor");
            link_dynamic("xkbcommon");
            link_dynamic("udev");
            link_dynamic("dl");
            link_dynamic("pthread");
            link_dynamic("m");
            link_dynamic("stdc++");
        }
    }
}

fn build_rustine_wl(project_dir: &Path, out_dir: &Path, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-wl"))
        .generator(generator)
        .out_dir(out_dir.join("rustine-wl"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-wl");
    link_dynamic("wayland-client");

    // Allow multiple definitions to resolve fractional-scale symbol conflict with GLFW
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");

    let rustine_wl_dir = project_dir.join("ext").join("rustine-wl");
    rerun_if_changed(rustine_wl_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.cpp"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.hpp"));
}

fn build_rustine_dxc(project_dir: &Path, out_dir: &Path, target_os: Os, generator: &'static str) {
    let dxc_include_dir = env::var("DXC_INCLUDE_DIR")
        .expect("DXC_INCLUDE_DIR environment variable must be set to the DXC include directory (containing dxcapi.h)");
    let dxc_lib_dir = env::var("DXC_LIB_DIR")
        .expect("DXC_LIB_DIR environment variable must be set to the DXC library directory (containing libdxcompiler.so or dxcompiler.dll)");

    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-dxc"))
        .generator(generator)
        .out_dir(out_dir.join("rustine-dxc"))
        .always_configure(true)
        .env("DXC_INCLUDE_DIR", &dxc_include_dir)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-dxc");

    // Get the target directory where the binary will be located
    let target_dir = PathBuf::from(
        env::var("CARGO_TARGET_DIR")
            .unwrap_or_else(|_| project_dir.join("target").to_string_lossy().into_owned()),
    );
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    let bin_dir = target_dir.join(&profile);

    // Copy DXC libraries to both OUT_DIR (for linking) and the binary output directory (for runtime)
    match target_os {
        Os::Windows => {
            let src_compiler = PathBuf::from(&dxc_lib_dir).join("dxcompiler.dll");
            let src_dxil = PathBuf::from(&dxc_lib_dir).join("dxil.dll");
            let dst_compiler_out = out_dir.join("dxcompiler.dll");
            let dst_dxil_out = out_dir.join("dxil.dll");
            let dst_compiler_bin = bin_dir.join("dxcompiler.dll");
            let dst_dxil_bin = bin_dir.join("dxil.dll");

            if src_compiler.exists() {
                fs::copy(&src_compiler, &dst_compiler_out)
                    .expect("Failed to copy dxcompiler.dll to out directory");
                fs::copy(&src_compiler, &dst_compiler_bin)
                    .expect("Failed to copy dxcompiler.dll to bin directory");
            }
            if src_dxil.exists() {
                fs::copy(&src_dxil, &dst_dxil_out)
                    .expect("Failed to copy dxil.dll to out directory");
                fs::copy(&src_dxil, &dst_dxil_bin)
                    .expect("Failed to copy dxil.dll to bin directory");
            }
        }
        Os::Linux => {
            let src_compiler = PathBuf::from(&dxc_lib_dir).join("libdxcompiler.so");

            if src_compiler.exists() {
                let dst_compiler_out = out_dir.join("libdxcompiler.so");
                // Try to copy to out_dir, but don't fail if we can't (e.g., read-only Nix store)
                if let Err(e) = fs::copy(&src_compiler, &dst_compiler_out) {
                    eprintln!("Warning: Could not copy libdxcompiler.so to out_dir: {}", e);
                }

                let dst_compiler_bin = bin_dir.join("libdxcompiler.so");
                // Try to copy to bin_dir, but don't fail if we can't (e.g., read-only Nix store)
                if let Err(e) = fs::copy(&src_compiler, &dst_compiler_bin) {
                    eprintln!("Warning: Could not copy libdxcompiler.so to bin_dir: {}", e);
                }
            } else {
                eprintln!(
                    "Warning: libdxcompiler.so not found at {}",
                    src_compiler.display()
                );
            }

            // Add the DXC library directory to the search path so the dynamic linker can find it
            link_search(&dxc_lib_dir);
            link_dynamic("dl");
            // Use $ORIGIN to find libdxcompiler.so relative to the binary
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN:{}", dxc_lib_dir);
        }
    }

    let rustine_dxc_dir = project_dir.join("ext").join("rustine-dxc");
    rerun_if_changed(rustine_dxc_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.cpp"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.hpp"));
}
