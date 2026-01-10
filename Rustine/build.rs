use fontdue::Font;
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

    build_bitmap_font(&project_dir, &out_dir);
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

fn build_bitmap_font(project_dir: &Path, out_dir: &Path) {
    let font_path = project_dir.join("src/gfx/fonts").join("ProggyClean.ttf");

    let font_data = fs::read(&font_path).expect("Failed to read font file");
    let font =
        Font::from_bytes(font_data.as_slice(), Default::default()).expect("Failed to load font");

    // Generate glyph metrics and bitmaps
    let mut glyph_code = String::new();
    glyph_code.push_str("// Auto-generated bitmap font data\n\n");

    glyph_code.push_str("pub struct GlyphMetrics {\n");
    glyph_code.push_str("    pub x: u32,\n");
    glyph_code.push_str("    pub y: u32,\n");
    glyph_code.push_str("    pub width: u32,\n");
    glyph_code.push_str("    pub height: u32,\n");
    glyph_code.push_str("    pub advance_width: i32,\n");
    glyph_code.push_str("    pub offset_x: i32,\n");
    glyph_code.push_str("    pub offset_y: i32,\n");
    glyph_code.push_str("    pub u0: f32,\n");
    glyph_code.push_str("    pub v0: f32,\n");
    glyph_code.push_str("    pub u1: f32,\n");
    glyph_code.push_str("    pub v1: f32,\n");
    glyph_code.push_str("}\n\n");

    // Rasterize full Latin-1 character set (0-255)
    // Skip control characters (0-31) to save space
    let mut metrics = Vec::new();
    let mut bitmaps = Vec::new();
    let mut max_height = 0u32;

    for ch in 32u8..=255u8 {
        let (metrics_local, bitmap) = font.rasterize(ch as char, 16.0);

        // Include all characters, even those with empty bitmaps (like space)
        max_height = max_height.max(metrics_local.height as u32);
        metrics.push((ch as char, metrics_local));
        bitmaps.push(bitmap);
    }

    // Pack glyphs into a single atlas (simple row layout)
    let mut current_x = 0u32;
    let mut current_y = 0u32;
    let mut glyph_positions = Vec::new();
    let mut atlas_width = 0u32;

    for (_i, (ch, metrics_local)) in metrics.iter().enumerate() {
        let width = metrics_local.width as u32;

        // Simple row layout with wrap at 2048px
        if current_x + width > 2048 {
            current_x = 0;
            current_y += max_height + 2;
        }

        glyph_positions.push((current_x, current_y, *ch, *metrics_local));
        current_x += width + 2; // 2px padding
        atlas_width = atlas_width.max(current_x);
    }

    let atlas_height = current_y + max_height;

    // Create atlas bitmap (RGBA8)
    let mut atlas = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    for (i, (px, py, _ch, met)) in glyph_positions.iter().enumerate() {
        let bitmap = &bitmaps[i];
        let width = met.width as u32;
        let height = met.height as u32;

        for y in 0..height {
            for x in 0..width {
                let bitmap_idx = (y * width + x) as usize;
                if bitmap_idx < bitmap.len() {
                    let alpha = bitmap[bitmap_idx];
                    let atlas_idx = ((py + y) * atlas_width + (px + x)) as usize * 4;
                    atlas[atlas_idx] = 255;
                    atlas[atlas_idx + 1] = 255;
                    atlas[atlas_idx + 2] = 255;
                    atlas[atlas_idx + 3] = alpha;
                }
            }
        }
    }

    // Generate Rust code
    glyph_code.push_str("pub const GLYPH_ATLAS_WIDTH: u32 = ");
    glyph_code.push_str(&atlas_width.to_string());
    glyph_code.push_str(";\n");
    glyph_code.push_str("pub const GLYPH_ATLAS_HEIGHT: u32 = ");
    glyph_code.push_str(&atlas_height.to_string());
    glyph_code.push_str(";\n\n");

    glyph_code.push_str("pub static GLYPH_ATLAS: &[u8] = &[\n");
    for chunk in atlas.chunks(16) {
        glyph_code.push_str("    ");
        for byte in chunk {
            glyph_code.push_str(&format!("{}, ", byte));
        }
        glyph_code.push_str("\n");
    }
    glyph_code.push_str("];\n\n");

    glyph_code.push_str("pub static GLYPH_METRICS: &[(char, GlyphMetrics)] = &[\n");
    let atlas_w = atlas_width as f32;
    let atlas_h = atlas_height as f32;
    for (px, py, ch, met) in &glyph_positions {
        let ch_escaped = format!("{}", ch.escape_default());
        let u0 = *px as f32 / atlas_w;
        let v0 = *py as f32 / atlas_h;
        let u1 = (*px as f32 + met.width as f32) / atlas_w;
        let v1 = (*py as f32 + met.height as f32) / atlas_h;
        glyph_code.push_str(&format!(
            "    ('{}', GlyphMetrics {{ x: {}, y: {}, width: {}, height: {}, advance_width: {}, offset_x: {}, offset_y: {}, u0: {}f32, v0: {}f32, u1: {}f32, v1: {}f32 }}),\n",
            ch_escaped,
            px,
            py,
            met.width,
            met.height,
            met.advance_width as i32,
            met.xmin as i32,
            met.ymin as i32,
            u0,
            v0,
            u1,
            v1,
        ));
    }
    glyph_code.push_str("];\n\n");

    // Generate full Latin-1 character set for testing
    glyph_code.push_str("/// Full Latin-1 character set (characters 32-255)\n");
    glyph_code.push_str("pub const LATIN1_CHARSET: &str = \"\\\n");
    for ch in 32u8..=255u8 {
        let c = ch as char;
        match c {
            '\\' => glyph_code.push_str("\\\\"),
            '"' => glyph_code.push_str("\\\""),
            _ => glyph_code.push(c),
        }
    }
    glyph_code.push_str("\";\n");

    let output_path = out_dir.join("ProggyClean.rs");
    fs::write(&output_path, glyph_code).expect("Failed to write generated font file");

    rerun_if_changed(&font_path);
}
