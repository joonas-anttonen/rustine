use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

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
    let use_ninja = should_use_ninja();
    let cmake_profile = if use_ninja { "ninja" } else { "native" };

    if cfg!(target_os = "windows") {
        if let Ok(vulkan_sdk) = env::var("VULKAN_SDK") {
            let vulkan_lib_dir = PathBuf::from(vulkan_sdk).join("Lib");
            if vulkan_lib_dir.exists() {
                link_search(vulkan_lib_dir);
            }
        }
        link_dynamic("vulkan-1");
    } else {
        link_dynamic("vulkan");
    }

    let shaders_dir = project_dir.join("src/gfx/shaders");
    rustinesc::build_shaders_emit_directives(&shaders_dir, &shaders_dir.join("shaders.toml"), &out_dir)
        .expect("Shader compilation failed");

    let fonts_dir = project_dir.join("src/gfx/fonts");
    let fonts_config = fonts_dir.join("fonts.toml");
    rustinefc::build_bitmap_fonts(&fonts_dir, &fonts_config, &out_dir)
        .expect("Font generation failed");
    build_rustine_vma(&project_dir, &out_dir, use_ninja, cmake_profile);

    if !cfg!(target_os = "windows") {
        build_rustine_webp(&project_dir, &out_dir, use_ninja, cmake_profile);
        build_rustine_ffmpeg(&project_dir, &out_dir, use_ninja, cmake_profile);
    }

    build_glfw(&project_dir, &out_dir, use_ninja, cmake_profile);

    build_luajit(&project_dir, &out_dir);
}

fn build_luajit(project_dir: &Path, out_dir: &Path) {
    let source_dir = project_dir.join("ext").join("luajit").join("src");

    if cfg!(target_os = "windows") {
        let script = source_dir.join("msvcbuild.bat");
        if script.exists() {
            let vcvars = find_vcvars64().unwrap_or_else(|| {
                panic!(
                    "Unable to locate vcvars64.bat. Install Visual Studio C++ tools or run from a Developer Command Prompt."
                )
            });
            let wrapper = out_dir.join("rustine-luajit-build.bat");
            let wrapper_content = format!(
                "@echo off\r\ncall \"{}\" >nul\r\nif errorlevel 1 exit /b 1\r\ncall msvcbuild.bat static\r\n",
                vcvars.display()
            );
            fs::write(&wrapper, wrapper_content).expect("Failed to write LuaJIT wrapper script");

            let status = Command::new("cmd")
                .arg("/C")
                .arg(&wrapper)
                .current_dir(&source_dir)
                .status()
                .expect("Failed to build LuaJIT");

            if !status.success() {
                panic!("LuaJIT build failed");
            }

            link_search(&source_dir);
            link_static("lua51");
            rerun_if_changed(source_dir);
            return;
        }

        panic!(
            "LuaJIT source is missing at '{}'. Initialize submodules before building on Windows.",
            source_dir.display()
        );
    }

    let status = Command::new("make")
        .current_dir(&source_dir)
        .arg("BUILDMODE=static")
        .arg("-j4")
        .status()
        .expect("Failed to build LuaJIT");

    if !status.success() {
        panic!("LuaJIT build failed");
    }

    link_search(&source_dir);
    link_static("luajit");

    rerun_if_changed(source_dir);
}

fn build_rustine_webp(project_dir: &Path, out_dir: &Path, use_ninja: bool, cmake_profile: &str) {
    let mut cmake_config = cmake::Config::new(project_dir.join("ext").join("rustine-webp"));
    cmake_config
        .out_dir(out_dir.join("rustine-webp").join(cmake_profile))
        .always_configure(true);
    if cfg!(target_env = "msvc") {
        cmake_config.profile("Release");
    }
    if use_ninja {
        cmake_config.generator("Ninja");
    }
    let destination_dir = cmake_config.build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine_webp");
    // Use system-provided libwebp and libwebpdemux as dynamic libraries.
    // Link webpdemux before webp so the linker resolves symbols from webp
    // which may be referenced by the static webpdemux archive.
    link_dynamic("webpdemux");
    link_dynamic("webp");

    let rustine_webp_dir = project_dir.join("ext").join("rustine-webp");
    rerun_if_changed(rustine_webp_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_webp_dir.join("rustine-webp.cpp"));
    rerun_if_changed(rustine_webp_dir.join("rustine-webp.hpp"));
}

fn build_rustine_ffmpeg(
    project_dir: &Path,
    out_dir: &Path,
    use_ninja: bool,
    cmake_profile: &str,
) {
    let mut cmake_config = cmake::Config::new(project_dir.join("ext").join("rustine-ffmpeg"));
    cmake_config
        .out_dir(out_dir.join("rustine-ffmpeg").join(cmake_profile))
        .always_configure(true);
    if cfg!(target_env = "msvc") {
        cmake_config.profile("Release");
    }
    if use_ninja {
        cmake_config.generator("Ninja");
    }
    let destination_dir = cmake_config.build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-ffmpeg");

    // Link FFmpeg libraries
    link_dynamic("avcodec");
    link_dynamic("avformat");
    link_dynamic("avutil");
    link_dynamic("swscale");

    let rustine_ffmpeg_dir = project_dir.join("ext").join("rustine-ffmpeg");
    rerun_if_changed(rustine_ffmpeg_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_ffmpeg_dir.join("rustine-ffmpeg.cpp"));
    rerun_if_changed(rustine_ffmpeg_dir.join("rustine-ffmpeg.hpp"));
}

fn build_rustine_vma(project_dir: &Path, out_dir: &Path, use_ninja: bool, cmake_profile: &str) {
    let mut cmake_config = cmake::Config::new(project_dir.join("ext").join("rustine-vma"));
    cmake_config
        .out_dir(out_dir.join("rustine-vma").join(cmake_profile))
        .always_configure(true);
    if cfg!(target_env = "msvc") {
        cmake_config.profile("Release");
    }
    if use_ninja {
        cmake_config.generator("Ninja");
    }
    let destination_dir = cmake_config.build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("rustine-vma");
}

fn build_glfw(project_dir: &Path, out_dir: &Path, use_ninja: bool, cmake_profile: &str) {
    let mut cmake_config = cmake::Config::new(project_dir.join("ext").join("glfw"));
    cmake_config
        .out_dir(out_dir.join("glfw").join(cmake_profile))
        .define("GLFW_BUILD_EXAMPLES", "OFF")
        .define("GLFW_BUILD_TESTS", "OFF")
        .define("GLFW_BUILD_DOCS", "OFF")
        .always_configure(true);

    if cfg!(target_os = "windows") {
        cmake_config
            .define("GLFW_BUILD_X11", "OFF")
            .define("GLFW_BUILD_WAYLAND", "OFF");
    } else {
        cmake_config
            .define("GLFW_BUILD_X11", "ON")
            .define("GLFW_BUILD_WAYLAND", "ON");
    }

    if cfg!(target_env = "msvc") {
        cmake_config.profile("Release");
    }
    if use_ninja {
        cmake_config.generator("Ninja");
    }

    let destination_dir = cmake_config.build();

    let lib_dir = prefer_lib64(&destination_dir);
    link_search(&lib_dir);
    link_static("glfw3");

    if cfg!(target_os = "windows") {
        link_dynamic("gdi32");
        link_dynamic("user32");
        link_dynamic("shell32");
    } else {
        link_dynamic("wayland-client");
        link_dynamic("wayland-cursor");
        link_dynamic("wayland-egl");
        link_dynamic("xkbcommon");
        link_dynamic("m");
        link_dynamic("dl");
        link_dynamic("pthread");
    }

    let glfw_dir = project_dir.join("ext").join("glfw");
    rerun_if_changed(glfw_dir.join("CMakeLists.txt"));
    rerun_if_changed(glfw_dir.join("src"));
    rerun_if_changed(glfw_dir.join("include"));
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

fn find_vcvars64() -> Option<PathBuf> {
    if let Ok(vcinstalldir) = env::var("VCINSTALLDIR") {
        let candidate = PathBuf::from(vcinstalldir)
            .join("Auxiliary")
            .join("Build")
            .join("vcvars64.bat");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Some(cl_path) = find_cl_path()
        && let Some(install_root) = cl_path
            .parent() // x64
            .and_then(|p| p.parent()) // HostX64
            .and_then(|p| p.parent()) // <toolset version>
            .and_then(|p| p.parent()) // MSVC
            .and_then(|p| p.parent()) // Tools
            .and_then(|p| p.parent()) // VC
            .and_then(|p| p.parent()) // <VS install root>
    {
        let candidate = install_root
            .join("VC")
            .join("Auxiliary")
            .join("Build")
            .join("vcvars64.bat");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let output = Command::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe")
        .arg("-latest")
        .arg("-products")
        .arg("*")
        .arg("-requires")
        .arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
        .arg("-property")
        .arg("installationPath")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let install_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if install_path.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(install_path)
        .join("VC")
        .join("Auxiliary")
        .join("Build")
        .join("vcvars64.bat");

    if candidate.exists() {
        return Some(candidate);
    }

    let fallback_candidates = [
        r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\18\Professional\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
    ];

    for path in fallback_candidates {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn find_cl_path() -> Option<PathBuf> {
    let output = Command::new("where").arg("cl").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    if first_line.is_empty() {
        None
    } else {
        Some(PathBuf::from(first_line))
    }
}
