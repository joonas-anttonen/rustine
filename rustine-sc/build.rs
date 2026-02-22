use std::{env, path::Path, path::PathBuf, process::Command};

fn main() {
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let rustine_dxc_dir = project_dir.join("rustine-dxc");
    let dxc_include_dir = resolve_dxc_include_dir();
    let use_ninja = should_use_ninja();
    let cmake_profile = if use_ninja { "ninja" } else { "native" };

    let mut cmake_config = cmake::Config::new(&rustine_dxc_dir);
    cmake_config
        .out_dir(out_dir.join("rustine-dxc").join(cmake_profile))
        .always_configure(true)
        .env("DXC_INCLUDE_DIR", &dxc_include_dir);

    if cfg!(target_env = "msvc") {
        cmake_config.profile("Release");
    }

    if use_ninja {
        cmake_config.generator("Ninja");
    }

    let destination_dir = cmake_config.build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-dxc");

    if cfg!(target_os = "linux") {
        link_dynamic("dl");
    }

    if cfg!(target_env = "gnu") {
        link_dynamic("stdc++");
    }

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

fn resolve_dxc_include_dir() -> String {
    if let Ok(dir) = env::var("DXC_INCLUDE_DIR") {
        return dir;
    }

    if let Ok(lib_dir) = env::var("DXC_LIB_DIR") {
        let lib_path = PathBuf::from(&lib_dir);
        let parent_include = lib_path.parent().map(|p| p.join("include"));
        let sibling_include = lib_path.join("..").join("include");

        let candidates = [
            parent_include,
            Some(sibling_include),
            Some(lib_path.clone()),
        ];

        for candidate in candidates.into_iter().flatten() {
            if has_dxc_header(&candidate) {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }

    panic!(
        "Set DXC_INCLUDE_DIR to a folder containing dxc/dxcapi.h or dxcapi.h. \
        Alternatively set DXC_LIB_DIR and keep headers under ../include relative to it."
    );
}

fn has_dxc_header(dir: &Path) -> bool {
    dir.join("dxc").join("dxcapi.h").exists() || dir.join("dxcapi.h").exists()
}

fn should_use_ninja() -> bool {
    if cfg!(target_os = "windows") {
        command_exists("ninja")
    } else {
        true
    }
}

fn command_exists(command: &str) -> bool {
    let probe = if cfg!(target_os = "windows") { "where" } else { "which" };
    Command::new(probe)
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
