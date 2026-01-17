use fontdue::Font;
use rustinesc::Compiler;
use std::{collections::HashMap, env, fs, path::Path, path::PathBuf};

/// Parse codepoint specification strings like "0x1234-0x5678" or "0xabcd"
fn parse_codepoint_spec(spec: &str, codepoints: &mut Vec<char>) {
    let spec = spec.trim();
    if spec.contains('-') {
        // Range specification
        let parts: Vec<&str> = spec.split('-').collect();
        if parts.len() == 2
            && let (Ok(start), Ok(end)) = (
                u32::from_str_radix(parts[0].trim_start_matches("0x"), 16),
                u32::from_str_radix(parts[1].trim_start_matches("0x"), 16),
            )
        {
            for cp in start..=end {
                if let Some(ch) = char::from_u32(cp) {
                    codepoints.push(ch);
                }
            }
        }
    } else {
        // Single codepoint
        if let Ok(cp) = u32::from_str_radix(spec.trim_start_matches("0x"), 16)
            && let Some(ch) = char::from_u32(cp)
        {
            codepoints.push(ch);
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

    let shaders_dir = project_dir.join("src/gfx/shaders");
    build_shaders(&shaders_dir, &out_dir);

    build_bitmap_fonts(&project_dir, &out_dir);
    build_rustine_vma(&project_dir, &out_dir, generator);
    build_rustine_webp(&project_dir, &out_dir, generator);
    build_rustine_ffmpeg(&project_dir, &out_dir, generator);
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

fn build_rustine_ffmpeg(project_dir: &Path, out_dir: &Path, generator: &'static str) {
    let destination_dir = cmake::Config::new(project_dir.join("ext").join("rustine-ffmpeg"))
        .generator(generator)
        .out_dir(out_dir.join("rustine-ffmpeg"))
        .always_configure(true)
        .build();

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

    // Allow multiple definitions to resolve fractional-scale symbol conflict with GLFW
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");

    let rustine_wl_dir = project_dir.join("ext").join("rustine-wl");
    rerun_if_changed(rustine_wl_dir.join("CMakeLists.txt"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.cpp"));
    rerun_if_changed(rustine_wl_dir.join("rustine-wl.hpp"));
}

fn build_shaders(shaders_dir: &Path, out_dir: &Path) {
    // *.hlsl
    let mut shader_files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(shaders_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && ext.to_string_lossy().to_lowercase() == "hlsl"
            {
                shader_files.push(path);
            }
        }
    }

    // Sort for deterministic ordering
    shader_files.sort();

    // Compile shaders
    let config_path = shaders_dir.join("shaders.toml");

    // If there's no shaders.toml, nothing to do
    if !config_path.exists() {
        for shader in &shader_files {
            rerun_if_changed(shader);
        }
        return;
    }

    let config_content = fs::read_to_string(&config_path).expect("Failed to read shaders.toml");
    let config =
        toml::from_str::<toml::Value>(&config_content).expect("Failed to parse shaders.toml");

    let mut module_code = String::new();
    module_code.push_str("// Auto-generated shader modules\n\n");
    module_code.push_str("// Generated bytecode blobs and accessor\n\n");

    // Generate shader IDs starting from u32::MAX - 2
    let mut shader_id = u32::MAX - 2u32;
    let mut shader_info: Vec<(String, u32, Vec<String>)> = Vec::new();

    // Create a compiler instance (build-dependency)
    let compiler = Compiler::new().expect("Failed to create shader compiler");

    if let Some(shaders_table) = config.get("shaders").and_then(|v| v.as_table()) {
        for (name, shader_config) in shaders_table {
            let mut stages_vec: Vec<String> = Vec::new();
            if let Some(table) = shader_config.as_table()
                && let Some(stages) = table.get("stages").and_then(|v| v.as_array())
            {
                for s in stages {
                    if let Some(s_str) = s.as_str() {
                        stages_vec.push(s_str.to_string());
                    }
                }
            }

            // Attempt to compile each requested stage
            let shader_path = shaders_dir.join(format!("{}.hlsl", name));
            let source = fs::read_to_string(&shader_path).unwrap_or_else(|_| {
                panic!("Failed to read shader source: {}", shader_path.display())
            });

            // For each stage, compile and emit a byte array
            for stage_name in &stages_vec {
                let stage_enum = match stage_name.as_str() {
                    "vertex" => rustinesc::Stage::VERTEX,
                    "fragment" => rustinesc::Stage::FRAGMENT,
                    "compute" => rustinesc::Stage::COMPUTE,
                    other => panic!("Unknown shader stage '{}' for shader '{}'", other, name),
                };

                let compiled = compiler
                    .compile(stage_enum, &source)
                    .unwrap_or_else(|e| match e {
                        rustinesc::CompilerStatus::CompilationFailed(msg) => panic!(
                            "Shader compilation failed for {}:{} => {}",
                            name, stage_name, msg
                        ),
                        _ => panic!("Shader compilation error for {}:{}", name, stage_name),
                    });

                // Emit a const byte array
                let const_name = format!("{}_{}", name.to_uppercase(), stage_name.to_uppercase());
                module_code.push_str(&format!("pub static {}: &[u8] = &[\n", const_name));
                if compiled.bytecode.is_empty() {
                    module_code.push_str("];\n");
                } else {
                    // write in hex for readability
                    for chunk in compiled.bytecode.chunks(16) {
                        module_code.push_str("    ");
                        for b in chunk {
                            module_code.push_str(&format!("0x{:02x}, ", b));
                        }
                        module_code.push('\n');
                    }
                    module_code.push_str("];\n");
                }
                module_code.push('\n');
                // mark shader source for rerun
                rerun_if_changed(&shader_path);
            }

            // store info for registry generation
            shader_info.push((name.clone(), shader_id, stages_vec));
            shader_id -= 1;
        }
    }

    // Generate constants and get_shaderprogram function
    for (name, id, _stages) in &shader_info {
        let const_name = format!("{}_SHADER_ID", name.to_uppercase());
        module_code.push_str(&format!(
            "pub const {}: u32 = {};// shader id\n",
            const_name, id
        ));
    }

    module_code.push_str("\nuse std::vec::Vec;\n\n");
    module_code
        .push_str("pub fn get_shaderprogram(id: u32) -> Option<::rustinesc::ShaderProgram> {\n");
    module_code.push_str("    match id {\n");
    for (name, id, stages) in &shader_info {
        module_code.push_str(&format!("        {} => {{\n", id));
        module_code.push_str("            let mut stages = Vec::new();\n");
        for stage_name in stages {
            let const_name = format!("{}_{}", name.to_uppercase(), stage_name.to_uppercase());
            let stage_enum = match stage_name.as_str() {
                "vertex" => "rustinesc::Stage::VERTEX",
                "fragment" => "rustinesc::Stage::FRAGMENT",
                "compute" => "rustinesc::Stage::COMPUTE",
                _ => panic!("unsupported stage"),
            };
            module_code.push_str(&format!(
                "            if !{}.is_empty() {{\n                stages.push(rustinesc::Shader {{ stage: {}, entry_point: \"{}\".to_string(), bytecode: {}.to_vec() }});\n            }}\n",
                const_name, stage_enum, stage_name, const_name
            ));
        }
        module_code.push_str(&format!(
            "            Some(rustinesc::ShaderProgram {{ name: \"{}\".to_string(), stages }})\n",
            name
        ));
        module_code.push_str("        },\n");
    }
    module_code.push_str("        _ => None,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n");

    // write generated file
    rerun_if_changed(&config_path);
    let output_path = out_dir.join("shaders.rs");
    fs::write(&output_path, module_code).expect("Failed to write generated shaders file");

    // Tell Cargo to rerun build script if any shader changes
    for shader in &shader_files {
        rerun_if_changed(shader);
    }
}

/// Character set specification for a font
#[derive(Clone)]
struct CharsetSpec {
    charset_type: CharsetType,
}

#[derive(Clone)]
enum CharsetType {
    /// Latin-1: characters 32-255
    Latin1,
    /// Unicode: specific codepoints defined by ranges or individual values
    Unicode(Vec<char>),
}

fn build_bitmap_fonts(project_dir: &Path, out_dir: &Path) {
    let fonts_dir = project_dir.join("src/gfx/fonts");
    let config_path = fonts_dir.join("fonts.toml");

    // Load font configuration
    let mut font_configs: HashMap<String, (f32, CharsetSpec)> = HashMap::new();
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path).expect("Failed to read fonts.toml");
        if let Ok(config) = toml::from_str::<toml::Value>(&config_content)
            && let Some(fonts_table) = config.get("fonts").and_then(|v| v.as_table())
        {
            for (name, font_config) in fonts_table {
                if let Some(table) = font_config.as_table() {
                    let size = table.get("size").and_then(|v| v.as_float()).unwrap_or(16.0) as f32;
                    let charset_str = table
                        .get("charset")
                        .and_then(|v| v.as_str())
                        .unwrap_or("latin1");

                    let charset_spec = match charset_str {
                        "latin1" => CharsetSpec {
                            charset_type: CharsetType::Latin1,
                        },
                        "unicode" => {
                            let mut codepoints = Vec::new();
                            if let Some(codepoints_array) =
                                table.get("codepoints").and_then(|v| v.as_array())
                            {
                                for cp_value in codepoints_array {
                                    if let Some(cp_str) = cp_value.as_str() {
                                        parse_codepoint_spec(cp_str, &mut codepoints);
                                    }
                                }
                            }
                            CharsetSpec {
                                charset_type: CharsetType::Unicode(codepoints),
                            }
                        }
                        _ => CharsetSpec {
                            charset_type: CharsetType::Latin1,
                        },
                    };

                    font_configs.insert(name.clone(), (size, charset_spec));
                } else if let Some(size) = font_config.as_float() {
                    // Backwards compatibility: simple float value defaults to latin1
                    font_configs.insert(
                        name.clone(),
                        (
                            size as f32,
                            CharsetSpec {
                                charset_type: CharsetType::Latin1,
                            },
                        ),
                    );
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

    module_code.push_str("pub struct FontMetrics {\n");
    module_code.push_str("    pub ascender: f32,\n");
    module_code.push_str("    pub descender: f32,\n");
    module_code.push_str("    pub line_gap: f32,\n");
    module_code.push_str("}\n\n");

    // Generate font IDs starting from u32::MAX - 2 and going down
    let mut font_id = u32::MAX - 2u32;
    let mut font_info = Vec::new();

    for font_path in &font_paths {
        let font_name = font_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("Invalid font filename");

        // Get font config from config, default to latin1 at 16.0
        let (font_size, charset_spec) = font_configs.get(font_name).cloned().unwrap_or({
            (
                16.0,
                CharsetSpec {
                    charset_type: CharsetType::Latin1,
                },
            )
        });

        build_bitmap_font(font_path, font_name, font_size, &charset_spec, out_dir);

        module_code.push_str(&format!(
            "include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\"));\n",
            font_name
        ));

        // Store font info for the registry
        font_info.push((font_name.to_string(), font_id));
        font_id -= 1;

        rerun_if_changed(font_path);
    }

    // Generate font constants and registry
    module_code.push_str("\n// Auto-generated font IDs\n");
    for (font_name, id) in font_info.iter() {
        let const_name = format!("{}_FONT_ID", font_name.to_uppercase());
        module_code.push_str(&format!("pub const {}: u32 = {};\n", const_name, id));
    }

    // Generate font size constants
    module_code.push_str("\n// Auto-generated font sizes\n");
    for (font_name, _id) in font_info.iter() {
        let (font_size, _) = font_configs.get(font_name).cloned().unwrap_or({
            (
                16.0,
                CharsetSpec {
                    charset_type: CharsetType::Latin1,
                },
            )
        });
        let const_name = format!("{}_FONT_SIZE", font_name.to_uppercase());
        module_code.push_str(&format!(
            "pub const {}: f32 = {}f32;\n",
            const_name, font_size
        ));
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
            "        {} => Some(FontAtlasData {{\n            texture_id: {},\n            atlas_data: {},\n            width: {},\n            height: {},\n        }}),\n",
            id, id, atlas_const, width_const, height_const
        ));
    }
    module_code.push_str("        _ => None,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n\n");

    // Generate glyph metrics accessor function using HashMap lookups
    module_code.push_str(
        "pub fn get_glyph_metrics(font_id: u32, ch: char) -> Option<&'static GlyphMetrics> {\n",
    );
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

    // Generate function to get all metrics for a font
    module_code.push_str(
        "pub fn get_all_metrics(font_id: u32) -> Option<&'static [(char, GlyphMetrics)]> {\n",
    );
    module_code.push_str("    match font_id {\n");
    for (font_name, id) in font_info.iter() {
        let metrics_const = format!("{}_METRICS", font_name.to_uppercase());
        module_code.push_str(&format!("        {} => Some({}),\n", id, metrics_const));
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

    // Generate font metrics accessor function
    module_code.push_str("pub fn get_font_metrics(font_id: u32) -> Option<FontMetrics> {\n");
    module_code.push_str("    match font_id {\n");
    for (font_name, id) in font_info.iter() {
        let ascender_const = format!("{}_ASCENDER", font_name.to_uppercase());
        let descender_const = format!("{}_DESCENDER", font_name.to_uppercase());
        let line_gap_const = format!("{}_LINE_GAP", font_name.to_uppercase());
        module_code.push_str(&format!(
            "        {} => Some(FontMetrics {{ ascender: {}, descender: {}, line_gap: {} }}),\n",
            id, ascender_const, descender_const, line_gap_const
        ));
    }
    module_code.push_str("        _ => None,\n");
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

fn build_bitmap_font(
    font_path: &Path,
    font_name: &str,
    font_size: f32,
    charset_spec: &CharsetSpec,
    out_dir: &Path,
) {
    let font_data = fs::read(font_path).expect("Failed to read font file");
    let font =
        Font::from_bytes(font_data.as_slice(), Default::default()).expect("Failed to load font");

    // Capture vertical metrics from the font
    let horizontal_line_metrics = font
        .horizontal_line_metrics(font_size)
        .expect("Failed to get horizontal line metrics");
    let ascender = horizontal_line_metrics.ascent;
    let descender = horizontal_line_metrics.descent;
    let line_gap = horizontal_line_metrics.line_gap;

    // Generate glyph metrics and bitmaps based on charset
    let mut glyph_code = String::new();
    glyph_code.push_str("// Auto-generated bitmap font data\n\n");

    // Determine which characters to rasterize
    let chars_to_rasterize: Vec<char> = match &charset_spec.charset_type {
        CharsetType::Latin1 => {
            // Skip control characters (0-31) to save space
            (32u8..=255u8).map(|b| b as char).collect()
        }
        CharsetType::Unicode(codepoints) => codepoints.clone(),
    };

    // Rasterize characters
    let mut metrics = Vec::new();
    let mut bitmaps = Vec::new();
    let mut max_height = 0u32;

    for ch in chars_to_rasterize {
        let (metrics_local, bitmap) = font.rasterize(ch, font_size);

        // Include all characters, even those with empty bitmaps (like space)
        max_height = max_height.max(metrics_local.height as u32);
        metrics.push((ch, metrics_local));
        bitmaps.push(bitmap);
    }

    // Pack glyphs into a single atlas (simple row layout)
    let mut current_x = 0u32;
    let mut current_y = 0u32;
    let mut glyph_positions = Vec::new();
    let mut atlas_width = 0u32;

    for (ch, metrics_local) in metrics.iter() {
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
    let ascender_const = format!("{}_ASCENDER", font_name.to_uppercase());
    let descender_const = format!("{}_DESCENDER", font_name.to_uppercase());
    let line_gap_const = format!("{}_LINE_GAP", font_name.to_uppercase());

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

    // Generate vertical metrics constants
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&ascender_const);
    glyph_code.push_str(": f32 = ");
    glyph_code.push_str(&format!("{}f32", ascender));
    glyph_code.push_str(";\n");
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&descender_const);
    glyph_code.push_str(": f32 = ");
    glyph_code.push_str(&format!("{}f32", descender));
    glyph_code.push_str(";\n");
    glyph_code.push_str("pub const ");
    glyph_code.push_str(&line_gap_const);
    glyph_code.push_str(": f32 = ");
    glyph_code.push_str(&format!("{}f32", line_gap));
    glyph_code.push_str(";\n\n");

    glyph_code.push_str("pub static ");
    glyph_code.push_str(&atlas_data_const);
    glyph_code.push_str(": &[u8] = &[\n");
    for chunk in atlas.chunks(16) {
        glyph_code.push_str("    ");
        for byte in chunk {
            glyph_code.push_str(&format!("{}, ", byte));
        }
        glyph_code.push('\n');
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
            met.xmin,
            met.ymin,
            u0,
            v0,
            u1,
            v1,
        ));
    }
    glyph_code.push_str("];\n\n");

    // Generate HashMap lookup for O(1) glyph access
    let lookup_const = format!("{}_LOOKUP", font_name.to_uppercase());
    glyph_code.push_str(&format!(
        "static {}: OnceLock<HashMap<char, &'static GlyphMetrics>> = OnceLock::new();\n\n",
        lookup_const
    ));

    let get_lookup_fn = format!("get_{}_lookup", font_name.to_lowercase());
    glyph_code.push_str(&format!(
        "fn {}() -> &'static HashMap<char, &'static GlyphMetrics> {{\n",
        get_lookup_fn
    ));
    glyph_code.push_str(&format!("    {}.get_or_init(|| {{\n", lookup_const));
    glyph_code.push_str("        let mut map = HashMap::new();\n");
    glyph_code.push_str(&format!(
        "        for (ch, metrics) in {}.iter() {{\n",
        metrics_const
    ));
    glyph_code.push_str("            map.insert(*ch, metrics);\n");
    glyph_code.push_str("        }\n");
    glyph_code.push_str("        map\n");
    glyph_code.push_str("    })\n");
    glyph_code.push_str("}\n\n");

    let output_path = out_dir.join(format!("{}.rs", font_name));
    fs::write(&output_path, glyph_code).expect("Failed to write generated font file");
}
