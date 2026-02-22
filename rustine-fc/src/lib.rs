use fontdue::Font;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Configuration for font rasterization and atlas generation.
#[derive(Clone)]
pub struct FontConfig {
    /// Font size in pixels
    pub size: f32,
    /// Character set to rasterize
    pub charset: Charset,
}

/// Charset specification for a font
#[derive(Clone)]
pub enum Charset {
    /// Latin-1: characters 32-255
    Latin1,
    /// Unicode: specific codepoints
    Unicode(Vec<char>),
}

/// Runtime-generated glyph metrics
#[derive(Clone, Debug)]
pub struct GlyphMetrics {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub advance_width: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// Runtime-generated font metrics
#[derive(Clone, Debug)]
pub struct FontMetrics {
    pub ascender: f32,
    pub descender: f32,
    pub line_gap: f32,
}

/// Runtime-generated font atlas data
#[derive(Clone)]
pub struct FontAtlas {
    /// RGBA8 bitmap data
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Glyph metrics indexed by character
    pub glyphs: HashMap<char, GlyphMetrics>,
    /// Font vertical metrics
    pub metrics: FontMetrics,
}

/// Parse codepoint specification strings like "0x1234-0x5678" or "0xabcd"
pub fn parse_codepoint_spec(spec: &str, codepoints: &mut Vec<char>) {
    let spec = spec.trim();
    if spec.contains('-') {
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
    } else if let Ok(cp) = u32::from_str_radix(spec.trim_start_matches("0x"), 16)
        && let Some(ch) = char::from_u32(cp)
    {
        codepoints.push(ch);
    }
}



/// Generate a font atlas from font data and configuration.
/// Takes raw font bytes and returns a runtime FontAtlas structure with bitmap and glyph metrics.
pub fn generate_font_atlas(font_data: &[u8], config: &FontConfig) -> Result<FontAtlas, Box<dyn Error>> {
    let font = Font::from_bytes(font_data, Default::default())
        .map_err(|_| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to load font")) as Box<dyn Error>)?;

    let horizontal_line_metrics = font
        .horizontal_line_metrics(config.size)
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to get horizontal line metrics")) as Box<dyn Error>)?;
    let ascender = horizontal_line_metrics.ascent;
    let descender = horizontal_line_metrics.descent;
    let line_gap = horizontal_line_metrics.line_gap;

    let chars_to_rasterize: Vec<char> = match &config.charset {
        Charset::Latin1 => (32u8..=255u8).map(|b| b as char).collect(),
        Charset::Unicode(codepoints) => codepoints.clone(),
    };

    let mut metrics = Vec::new();
    let mut bitmaps = Vec::new();

    for ch in chars_to_rasterize {
        let (metrics_local, bitmap) = font.rasterize(ch, config.size);
        metrics.push((ch, metrics_local));
        bitmaps.push(bitmap);
    }

    let glyph_padding = 2u32;
    let mut pack_order: Vec<GlyphCandidate> = metrics
        .iter()
        .enumerate()
        .map(|(bitmap_index, (ch, metrics_local))| GlyphCandidate {
            ch: *ch,
            metrics: *metrics_local,
            bitmap_index,
        })
        .collect();

    pack_order.sort_by(|a, b| {
        b.metrics
            .height
            .cmp(&a.metrics.height)
            .then_with(|| b.metrics.width.cmp(&a.metrics.width))
    });

    let max_glyph_dim = pack_order
        .iter()
        .map(|g| g.metrics.width.max(g.metrics.height))
        .max()
        .unwrap_or(0)
        .max(1) as u32;
    let mut side = max_glyph_dim.max(16).next_power_of_two();

    let positions = loop {
        if let Some(positions) = try_pack_square(side, &pack_order, glyph_padding) {
            break positions;
        }
        side *= 2;
    };

    let atlas_width = side;
    let atlas_height = side;
    let mut atlas_bitmap = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    let mut glyph_metrics = HashMap::new();

    for (x, y, ch, metrics_data, bitmap_index) in positions {
        let bitmap = &bitmaps[bitmap_index];
        let width = metrics_data.width as u32;
        let height = metrics_data.height as u32;

        for row in 0..height {
            for col in 0..width {
                let src_idx = (row * width + col) as usize;
                if src_idx < bitmap.len() {
                    let dst_x = x + col;
                    let dst_y = y + row;
                    let dst_idx = ((dst_y * atlas_width + dst_x) * 4) as usize;
                    if dst_idx + 3 < atlas_bitmap.len() {
                        let alpha = bitmap[src_idx];
                        atlas_bitmap[dst_idx] = 255;
                        atlas_bitmap[dst_idx + 1] = 255;
                        atlas_bitmap[dst_idx + 2] = 255;
                        atlas_bitmap[dst_idx + 3] = alpha;
                    }
                }
            }
        }

        let u0 = x as f32 / atlas_width as f32;
        let v0 = y as f32 / atlas_height as f32;
        let u1 = (x + width) as f32 / atlas_width as f32;
        let v1 = (y + height) as f32 / atlas_height as f32;

        glyph_metrics.insert(
            ch,
            GlyphMetrics {
                x,
                y,
                width,
                height,
                advance_width: metrics_data.advance_width as i32,
                offset_x: metrics_data.xmin as i32,
                offset_y: metrics_data.ymin as i32,
                u0,
                v0,
                u1,
                v1,
            },
        );
    }

    Ok(FontAtlas {
        bitmap: atlas_bitmap,
        width: atlas_width,
        height: atlas_height,
        glyphs: glyph_metrics,
        metrics: FontMetrics {
            ascender,
            descender,
            line_gap,
        },
    })
}

#[derive(Clone, Copy)]
struct GlyphCandidate {
    ch: char,
    metrics: fontdue::Metrics,
    bitmap_index: usize,
}

fn try_pack_square(
    side: u32,
    glyphs: &[GlyphCandidate],
    padding: u32,
) -> Option<Vec<(u32, u32, char, fontdue::Metrics, usize)>> {
    let mut current_x = 0u32;
    let mut current_y = 0u32;
    let mut row_height = 0u32;
    let mut positions = Vec::with_capacity(glyphs.len());

    for glyph in glyphs {
        let width = glyph.metrics.width as u32;
        let height = glyph.metrics.height as u32;

        if width > side || height > side {
            return None;
        }

        if current_x > 0 && current_x + width > side {
            current_x = 0;
            current_y += row_height + padding;
            row_height = 0;
        }

        if current_y + height > side {
            return None;
        }

        positions.push((
            current_x,
            current_y,
            glyph.ch,
            glyph.metrics,
            glyph.bitmap_index,
        ));

        current_x += width + padding;
        row_height = row_height.max(height);
    }

    Some(positions)
}

/// Core font generation for build-time: reads fonts and config, generates Rust code.
/// Writes individual font .rs files to out_dir and returns the main module code.
pub fn generate_fonts(fonts_dir: &Path, config_path: &Path, out_dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut font_configs: HashMap<String, (f32, Charset)> = HashMap::new();
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)?;
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

                    let charset = match charset_str {
                        "latin1" => Charset::Latin1,
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
                            Charset::Unicode(codepoints)
                        }
                        _ => Charset::Latin1,
                    };

                    font_configs.insert(name.clone(), (size, charset));
                } else if let Some(size) = font_config.as_float() {
                    font_configs.insert(
                        name.clone(),
                        (size as f32, Charset::Latin1),
                    );
                }
            }
        }
    }

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

    let mut font_id = u32::MAX - 2u32;
    let mut font_info = Vec::new();

    for font_path in &font_paths {
        let font_name = font_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("Invalid font filename");

        let (font_size, charset) = font_configs.get(font_name).cloned().unwrap_or((16.0, Charset::Latin1));

        build_bitmap_font_codegen(font_path, font_name, font_size, &charset, out_dir)?;

        module_code.push_str(&format!(
            "include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\"));\n",
            font_name
        ));

        font_info.push((font_name.to_string(), font_id));
        font_id -= 1;
    }

    module_code.push_str("\n// Auto-generated font IDs\n");
    for (font_name, id) in font_info.iter() {
        let const_name = format!("{}_FONT_ID", font_name.to_uppercase());
        module_code.push_str(&format!("pub const {}: u32 = {};\n", const_name, id));
    }

    module_code.push_str("\n// Auto-generated font sizes\n");
    for (font_name, _id) in font_info.iter() {
        let (font_size, _) = font_configs.get(font_name).cloned().unwrap_or((16.0, Charset::Latin1));
        let const_name = format!("{}_FONT_SIZE", font_name.to_uppercase());
        module_code.push_str(&format!(
            "pub const {}: f32 = {}f32;\n",
            const_name, font_size
        ));
    }

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

    module_code.push_str("pub fn get_font_size(font_id: u32) -> f32 {\n");
    module_code.push_str("    match font_id {\n");
    for (font_name, id) in font_info.iter() {
        let size_const = format!("{}_FONT_SIZE", font_name.to_uppercase());
        module_code.push_str(&format!("        {} => {},\n", id, size_const));
    }
    module_code.push_str("        _ => 16.0,\n");
    module_code.push_str("    }\n");
    module_code.push_str("}\n\n");

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

    module_code.push_str("pub const ALL_FONT_IDS: &[u32] = &[");
    for (id_idx, (_, id)) in font_info.iter().enumerate() {
        if id_idx > 0 {
            module_code.push_str(", ");
        }
        module_code.push_str(&id.to_string());
    }
    module_code.push_str("];\n");

    let output_path = out_dir.join("fonts.rs");
    fs::write(&output_path, &module_code)?;

    Ok(module_code)
}

/// Build-time wrapper for generate_fonts that emits cargo directives.
/// Use this in build.rs scripts.
pub fn build_bitmap_fonts(fonts_dir: &Path, config_path: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let _module_code = generate_fonts(fonts_dir, config_path, out_dir)?;

    println!("cargo:rerun-if-changed={}", config_path.display());
    if let Ok(entries) = fs::read_dir(fonts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "ttf" || ext_str == "otf" {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }

    Ok(())
}

fn build_bitmap_font_codegen(
    font_path: &Path,
    font_name: &str,
    font_size: f32,
    charset: &Charset,
    out_dir: &Path,
) -> Result<(), Box<dyn Error>> {


    let font_data = fs::read(font_path)?;
    let font = Font::from_bytes(font_data.as_slice(), Default::default())
        .map_err(|_| "Failed to load font")?;

    let horizontal_line_metrics = font
        .horizontal_line_metrics(font_size)
        .ok_or("Failed to get horizontal line metrics")?;
    let ascender = horizontal_line_metrics.ascent;
    let descender = horizontal_line_metrics.descent;
    let line_gap = horizontal_line_metrics.line_gap;

    let mut glyph_code = String::new();
    glyph_code.push_str("// Auto-generated bitmap font data\n\n");

    let chars_to_rasterize: Vec<char> = match charset {
        Charset::Latin1 => (32u8..=255u8).map(|b| b as char).collect(),
        Charset::Unicode(codepoints) => codepoints.clone(),
    };

    let mut metrics = Vec::new();
    let mut bitmaps = Vec::new();

    for ch in chars_to_rasterize {
        let (metrics_local, bitmap) = font.rasterize(ch, font_size);
        metrics.push((ch, metrics_local));
        bitmaps.push(bitmap);
    }

    let glyph_padding = 2u32;
    let mut pack_order: Vec<GlyphCandidate> = metrics
        .iter()
        .enumerate()
        .map(|(bitmap_index, (ch, metrics_local))| GlyphCandidate {
            ch: *ch,
            metrics: *metrics_local,
            bitmap_index,
        })
        .collect();

    pack_order.sort_by(|a, b| {
        b.metrics
            .height
            .cmp(&a.metrics.height)
            .then_with(|| b.metrics.width.cmp(&a.metrics.width))
    });

    let max_glyph_dim = pack_order
        .iter()
        .map(|g| (g.metrics.width as u32).max(g.metrics.height as u32))
        .max()
        .unwrap_or(0);
    let total_padded_area: u64 = pack_order
        .iter()
        .map(|g| {
            let width = g.metrics.width as u64 + glyph_padding as u64;
            let height = g.metrics.height as u64 + glyph_padding as u64;
            width * height
        })
        .sum();
    let area_hint = (total_padded_area as f64).sqrt().ceil() as u32;

    let min_atlas_side = 64u32;
    let max_atlas_side = 8192u32;
    let mut low = max_glyph_dim.max(area_hint).max(1);
    let mut high = low;
    if high > max_atlas_side {
        return Err(format!(
            "Font '{}' requires atlas side {} which exceeds maximum {}",
            font_name, high, max_atlas_side
        ).into());
    }
    while try_pack_square(high, &pack_order, glyph_padding).is_none() {
        high = high.saturating_mul(2);
        if high == 0 || high > max_atlas_side {
            return Err(format!(
                "Font '{}' cannot fit glyphs into max atlas {}x{}",
                font_name, max_atlas_side, max_atlas_side
            ).into());
        }
    }

    while low < high {
        let mid = low + (high - low) / 2;
        if try_pack_square(mid, &pack_order, glyph_padding).is_some() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }

    let atlas_side = low.next_power_of_two().max(min_atlas_side);
    if atlas_side > max_atlas_side {
        return Err(format!(
            "Font '{}' requires atlas {}x{} which exceeds max {}x{}",
            font_name, atlas_side, atlas_side, max_atlas_side, max_atlas_side
        ).into());
    }
    let atlas_width = atlas_side;
    let atlas_height = atlas_side;
    let glyph_positions = try_pack_square(atlas_width, &pack_order, glyph_padding)
        .ok_or("Failed to pack glyphs into atlas")?;

    let mut atlas = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    for (px, py, _ch, met, bitmap_index) in glyph_positions.iter() {
        let bitmap = &bitmaps[*bitmap_index];
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

    let atlas_width_const = format!("{}_ATLAS_WIDTH", font_name.to_uppercase());
    let atlas_height_const = format!("{}_ATLAS_HEIGHT", font_name.to_uppercase());
    let atlas_data_const = format!("{}_ATLAS", font_name.to_uppercase());
    let metrics_const = format!("{}_METRICS", font_name.to_uppercase());
    let ascender_const = format!("{}_ASCENDER", font_name.to_uppercase());
    let descender_const = format!("{}_DESCENDER", font_name.to_uppercase());
    let line_gap_const = format!("{}_LINE_GAP", font_name.to_uppercase());

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
    for (px, py, ch, met, _bitmap_index) in &glyph_positions {
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
    fs::write(&output_path, glyph_code)?;

    Ok(())
}
