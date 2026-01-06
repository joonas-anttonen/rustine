use std::{env, fs, path::Path, path::PathBuf};

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

/// Returns the preferred library directory, favoring "lib64" if it exists.
/// rustine-webp needs this at least
fn prefer_lib64(destination_dir: &Path) -> PathBuf {
    let lib64 = destination_dir.join("lib64");
    if lib64.exists() {
        lib64
    } else {
        destination_dir.join("lib")
    }
}

fn main() {
    let _profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let generator = "Ninja";

    link_dynamic("vulkan");

    build_rustine_vma(&project_dir, &out_dir, generator);
    build_rustine_webp(&project_dir, &out_dir, generator);
    build_rustine_dxc(&project_dir, &out_dir, generator);
    build_rustine_wl(&project_dir, &out_dir, generator);
}

fn build_rustine_webp(project_dir: &Path, out_dir: &Path, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-webp"))
        .generator(generator)
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

fn build_rustine_vma(project_dir: &Path, out_dir: &Path, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-vma"))
        .generator(generator)
        .out_dir(out_dir.join("rustine-vma"))
        .always_configure(true)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-vma");
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
    link_dynamic("xkbcommon");
    link_dynamic("stdc++");

    // GLFW
    //link_dynamic("wayland-egl");
    //link_dynamic("wayland-cursor");
    //link_dynamic("udev");
    //link_dynamic("dl");
    //link_dynamic("pthread");
    //link_dynamic("m");

    // Allow multiple definitions to resolve fractional-scale symbol conflict with GLFW
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");

    let rustine_wl_dir = project_dir.join("ext").join("rustine-wl");
    rerun_if_changed(rustine_wl_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.cpp"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.hpp"));
}

fn build_rustine_dxc(project_dir: &Path, out_dir: &Path, generator: &'static str) {
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

    let rustine_dxc_dir = project_dir.join("ext").join("rustine-dxc");
    rerun_if_changed(rustine_dxc_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.cpp"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.hpp"));
}
