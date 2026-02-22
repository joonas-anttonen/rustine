#![allow(dead_code)]

pub enum WebPResult {
    Ok(u32),
    EndOfStream,
    Error(String),
}

pub struct WebPDecoder {
    width: u32,
    height: u32,
    frame_count: u32,
    loop_count: u32,
}

impl WebPDecoder {
    pub fn new(_data: &[u8]) -> Result<Self, String> {
        Err("WebP backend is not available on Windows in this build".to_string())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn loop_count(&self) -> u32 {
        self.loop_count
    }

    pub fn next_frame(&mut self, _rgba_out: &mut [u8]) -> WebPResult {
        WebPResult::Error("WebP backend is not available on Windows in this build".to_string())
    }

    pub fn reset(&mut self) -> Result<(), String> {
        Err("WebP backend is not available on Windows in this build".to_string())
    }
}
