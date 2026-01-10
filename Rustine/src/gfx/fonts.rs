#![allow(dead_code)]

// Bitmap font renderer using embedded glyph atlas
// Generated at build time from ProggyClean.ttf

include!(concat!(env!("OUT_DIR"), "/ProggyClean.rs"));

/// Get glyph metrics for a character
pub fn get_glyph(ch: char) -> Option<&'static GlyphMetrics> {
    GLYPH_METRICS
        .iter()
        .find(|(c, _)| *c == ch)
        .map(|(_, metrics)| metrics)
}

/// Get the texture data for the glyph atlas (RGBA8)
pub fn get_atlas_data() -> &'static [u8] {
    GLYPH_ATLAS
}

/// Get atlas dimensions
pub fn get_atlas_dimensions() -> (u32, u32) {
    (GLYPH_ATLAS_WIDTH, GLYPH_ATLAS_HEIGHT)
}
