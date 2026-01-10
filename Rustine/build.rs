use fontdue::Font;
use std::{collections::HashMap, env, fs, path::Path, path::PathBuf};

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

    build_bitmap_fonts(&project_dir, &out_dir);
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

fn build_bitmap_fonts(project_dir: &Path, out_dir: &Path) {
    let fonts_dir = project_dir.join("src/gfx/fonts");
    let config_path = fonts_dir.join("fonts.toml");

    // Load font configuration
    let mut font_sizes: HashMap<String, f32> = HashMap::new();
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .expect("Failed to read fonts.toml");
        if let Ok(config) = toml::from_str::<toml::Value>(&config_content) {
            if let Some(fonts_table) = config.get("fonts").and_then(|v| v.as_table()) {
                for (name, value) in fonts_table {
                    if let Some(size) = value.as_float() {
                        font_sizes.insert(name.clone(), size as f32);
                    }
                }
            }
        }
    }

    // Discover all font files (TTF and OTF)
    let mut font_paths = Vec::new();
    if let Ok(entries) = fs::read_dir(&fonts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "ttf" || ext_str == "otf" {
                    font_paths.push(path);
                }
            }
        }
    }

    font_paths.sort();

    // Generate a module file that includes all font modules
    let mut module_code = String::new();
    module_code.push_str("// Auto-generated font modules\n\n");
    module_code.push_str("use std::collections::HashMap;\n");
    module_code.push_str("use std::sync::OnceLock;\n\n");
    module_code.push_str("pub struct GlyphMetrics {\n");
    module_code.push_str("    pub x: u32,\n");
    module_code.push_str("    pub y: u32,\n");
    module_code.push_str("    pub width: u32,\n");
    module_code.push_str("    pub height: u32,\n");
    module_code.push_str("    pub advance_width: i32,\n");
    module_code.push_str("    pub offset_x: i32,\n");
    module_code.push_str("    pub offset_y: i32,\n");
    module_code.push_str("    pub u0: f32,\n");
    module_code.push_str("    pub v0: f32,\n");
    module_code.push_str("    pub u1: f32,\n");
    module_code.push_str("    pub v1: f32,\n");
    module_code.push_str("}\n\n");

    // Generate font IDs starting from u32::MAX - 2 and going down
    let mut font_id = u32::MAX - 2u32;
    let mut font_info = Vec::new();

    for font_path in &font_paths {
        let font_name = font_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("Invalid font filename");

        // Get font size from config, default to 16.0
        let font_size = font_sizes.get(font_name).copied().unwrap_or(16.0);

        build_bitmap_font(&font_path, font_name, font_size, out_dir);
        
        module_code.push_str(&format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\"));\n", font_name));
        
        // Store font info for the registry
        font_info.push((font_name.to_string(), font_id));
        font_id -= 1;

        rerun_if_changed(&font_path);
    }

    // Generate font constants and registry
    module_code.push_str("\n// Auto-generated font IDs\n");
    for (_idx, (font_name, id)) in font_info.iter().enumerate() {
        let const_name = format!("{}_FONT_ID", font_name.to_uppercase());
        module_code.push_str(&format!("pub const {}: u32 = {};\n", const_name, id));
    }

    // Generate font size constants
    module_code.push_str("\n// Auto-generated font sizes\n");
    for (font_name, _id) in font_info.iter() {
        let font_size = font_sizes.get(font_name).copied().unwrap_or(16.0);
        let const_name = format!("{}_FONT_SIZE", font_name.to_uppercase());
        module_code.push_str(&format!("pub const {}: f32 = {}f32;\n", const_name, font_size));
    }

    // Generate font data structure and registry
    module_code.push_str("\npub struct FontAtlasData {\n");
    module_code.push_str("    pub texture_id: u32,\n");
    module_code.push_str("    pub atlas_data: &'static [u8],\n");
    module_code.push_str("    pub width: u32,\n");
    module_code.push_str("    pub height: u32,\n");
    module_code.push_str("}\n\n");

    module_code.push_str("pub fn get_font_atlas(id: u32) -> Option<FontAtlasData> {\n");
    module_code.push_str("    match id {\n");
    for (font_name, id) in font_info.iter() {
        let atlas_const = format!("{}_ATLAS", font_name.to_uppercase());
        let width_const = format!("{}_ATLAS_WIDTH", font_name.to_uppercase());
        let height_const = format!("{}_ATLAS_HEIGHT", font_name.to_uppercase());
        module_code.push_str(&format!(
            "        {} => Some(FontAtlasData {{\n            texture_id: {},\n            atlas_data: &{},\n            width: {},\n            height: {},\n        }}),\n",
            id, id, atlas_const, width_const, height_const
        ));
    }
    module_code.push_str("        _ => None,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n\n");

    // Generate glyph metrics accessor function using HashMap lookups
    module_code.push_str("pub fn get_glyph_metrics(font_id: u32, ch: char) -> Option<&'static GlyphMetrics> {\n");
    module_code.push_str("    match font_id {\n");
    for (font_name, id) in font_info.iter() {
        let get_lookup_fn = format!("get_{}_lookup", font_name.to_lowercase());
        module_code.push_str(&format!(
            "        {} => {}().get(&ch).copied(),\n",
            id, get_lookup_fn
        ));
    }
    module_code.push_str("        _ => None,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n\n");

    // Generate font size accessor function
    module_code.push_str("pub fn get_font_size(font_id: u32) -> f32 {\n");
    module_code.push_str("    match font_id {\n");
    for (font_name, id) in font_info.iter() {
        let size_const = format!("{}_FONT_SIZE", font_name.to_uppercase());
        module_code.push_str(&format!("        {} => {},\n", id, size_const));
    }
    module_code.push_str("        _ => 16.0,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n\n");

    // Generate array of all font IDs
    module_code.push_str("pub const ALL_FONT_IDS: &[u32] = &[");
    for (id_idx, (_, id)) in font_info.iter().enumerate() {
        if id_idx > 0 {
            module_code.push_str(", ");
        }
        module_code.push_str(&id.to_string());
    }
    module_code.push_str("];\n");

    rerun_if_changed(&config_path);
    let output_path = out_dir.join("fonts.rs");
    fs::write(&output_path, module_code).expect("Failed to write generated fonts file");
}

fn build_bitmap_font(font_path: &Path, font_name: &str, font_size: f32, out_dir: &Path) {
    let font_data = fs::read(font_path).expect("Failed to read font file");
    let font =
        Font::from_bytes(font_data.as_slice(), Default::default()).expect("Failed to load font");

    // Generate glyph metrics and bitmaps
    let mut glyph_code = String::new();
    glyph_code.push_str("// Auto-generated bitmap font data\n\n");

    // Rasterize full Latin-1 character set (0-255)
    // Skip control characters (0-31) to save space
    let mut metrics = Vec::new();
    let mut bitmaps = Vec::new();
    let mut max_height = 0u32;

    for ch in 32u8..=255u8 {
        let (metrics_local, bitmap) = font.rasterize(ch as char, font_size);

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

    // Generate variable names based on font name
    let atlas_width_const = format!("{}_ATLAS_WIDTH", font_name.to_uppercase());
    let atlas_height_const = format!("{}_ATLAS_HEIGHT", font_name.to_uppercase());
    let atlas_data_const = format!("{}_ATLAS", font_name.to_uppercase());
    let metrics_const = format!("{}_METRICS", font_name.to_uppercase());
    let charset_const = format!("{}_CHARSET", font_name.to_uppercase());

    // Generate Rust code
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&atlas_width_const);
    glyph_code.push_str(": u32 = ");
    glyph_code.push_str(&atlas_width.to_string());
    glyph_code.push_str(";\n");
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&atlas_height_const);
    glyph_code.push_str(": u32 = ");
    glyph_code.push_str(&atlas_height.to_string());
    glyph_code.push_str(";\n\n");

    glyph_code.push_str("pub static ");
    glyph_code.push_str(&atlas_data_const);
    glyph_code.push_str(": &[u8] = &[\n");
    for chunk in atlas.chunks(16) {
        glyph_code.push_str("    ");
        for byte in chunk {
            glyph_code.push_str(&format!("{}, ", byte));
        }
        glyph_code.push_str("\n");
    }
    glyph_code.push_str("];\n\n");

    glyph_code.push_str("pub static ");
    glyph_code.push_str(&metrics_const);
    glyph_code.push_str(": &[(char, GlyphMetrics)] = &[\n");
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

    // Generate HashMap lookup for O(1) glyph access
    let lookup_const = format!("{}_LOOKUP", font_name.to_uppercase());
    glyph_code.push_str(&format!("static {}: OnceLock<HashMap<char, &'static GlyphMetrics>> = OnceLock::new();\n\n", lookup_const));
    
    let get_lookup_fn = format!("get_{}_lookup", font_name.to_lowercase());
    glyph_code.push_str(&format!("fn {}() -> &'static HashMap<char, &'static GlyphMetrics> {{\n", get_lookup_fn));
    glyph_code.push_str(&format!("    {}.get_or_init(|| {{\n", lookup_const));
    glyph_code.push_str("        let mut map = HashMap::new();\n");
    glyph_code.push_str(&format!("        for (ch, metrics) in {}.iter() {{\n", metrics_const));
    glyph_code.push_str("            map.insert(*ch, metrics);\n");
    glyph_code.push_str("        }\n");
    glyph_code.push_str("        map\n");
    glyph_code.push_str("    })\n");
    glyph_code.push_str("}\n\n");

    // Generate full Latin-1 character set for testing
    glyph_code.push_str("/// Full Latin-1 character set (characters 32-255)\n");
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&charset_const);
    glyph_code.push_str(": &str = \"\\\n");
    for ch in 32u8..=255u8 {
        let c = ch as char;
        match c {
            '\\' => glyph_code.push_str("\\\\"),
            '"' => glyph_code.push_str("\\\""),
            _ => glyph_code.push(c),
        }
    }
    glyph_code.push_str("\";\n");

    let output_path = out_dir.join(format!("{}.rs", font_name));
    fs::write(&output_path, glyph_code).expect("Failed to write generated font file");
}
