use std::{env, path::Path, path::PathBuf};

fn main() {
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let generator = "Ninja";

    let rustine_dxc_dir = project_dir.join("rustine-dxc");
    let dxc_include_dir = env::var("DXC_INCLUDE_DIR")
            .expect("DXC_INCLUDE_DIR environment variable must be set to the DXC include directory (containing dxcapi.h)");

    let destination_dir = cmake::Config::new(&rustine_dxc_dir)
        .generator(generator)
        .out_dir(out_dir.join("rustine-dxc"))
        .always_configure(true)
        .env("DXC_INCLUDE_DIR", &dxc_include_dir)
        .build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-dxc");
    link_dynamic("dl");
    link_dynamic("stdc++");

    rerun_if_changed(rustine_dxc_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.cpp"));
    rerun_if_changed(rustine_dxc_dir.join("rustine-dxc.hpp"));
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

/// Returns the preferred library directory, favoring "lib64" if it exists.
fn prefer_lib64(destination_dir: &Path) -> PathBuf {
    let lib64 = destination_dir.join("lib64");
    if lib64.exists() {
        lib64
    } else {
        destination_dir.join("lib")
    }
}
