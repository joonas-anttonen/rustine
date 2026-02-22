#![allow(dead_code)]

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RffmpegPixelFormat {
    BayerRggb8,
    BayerBggr8,
    BayerGbrg8,
    BayerGrbg8,
    BayerRggb16,
    BayerBggr16,
    BayerGbrg16,
    BayerGrbg16,
    Mono8,
    Mono10,
    Mono12,
    Mono16,
    BayerRggb10,
    BayerBggr10,
    BayerGbrg10,
    BayerGrbg10,
    BayerRggb12,
    BayerBggr12,
    BayerGbrg12,
    BayerGrbg12,
    Mono10Packed,
    Mono12Packed,
    BayerRggb10Packed,
    BayerBggr10Packed,
    BayerGbrg10Packed,
    BayerGrbg10Packed,
    BayerRggb12Packed,
    BayerBggr12Packed,
    BayerGbrg12Packed,
    BayerGrbg12Packed,
}

pub struct VideoDecoder {
    width: u32,
    height: u32,
    frame_count: u32,
    fps: f64,
    duration_sec: f64,
}

impl VideoDecoder {
    pub fn new(_data: &[u8]) -> Result<Self, String> {
        Err("FFmpeg backend is not available on Windows in this build".to_string())
    }

    pub fn from_path(_path: &str) -> Result<Self, String> {
        Err("FFmpeg backend is not available on Windows in this build".to_string())
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

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn duration_sec(&self) -> f64 {
        self.duration_sec
    }

    pub fn next_frame(&mut self, _rgba_out: &mut [u8]) -> Result<u32, String> {
        Err("FFmpeg backend is not available on Windows in this build".to_string())
    }

    pub fn reset(&mut self) -> Result<(), String> {
        Err("FFmpeg backend is not available on Windows in this build".to_string())
    }
}

pub struct JpegEncoder {
    width: u32,
    height: u32,
    quality: u8,
}

impl JpegEncoder {
    pub fn new(width: u32, height: u32, quality: u8) -> Result<Self, String> {
        Ok(Self {
            width,
            height,
            quality,
        })
    }

    pub fn encode_rgba(&mut self, rgba_in: &[u8]) -> Result<Vec<u8>, String> {
        let expected = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(|| "Image dimensions are too large".to_string())?;

        if rgba_in.len() < expected {
            return Err(format!(
                "Input buffer too small: {} bytes, need {}",
                rgba_in.len(),
                expected
            ));
        }

        let _ = self.quality;
        Err("JPEG encoder backend is not available on Windows in this build".to_string())
    }
}

pub fn demosaic_to_rgba(
    _input: &[u8],
    _width: u32,
    _height: u32,
    _input_format: RffmpegPixelFormat,
) -> Result<Vec<u8>, String> {
    Err("Demosaic backend is not available on Windows in this build".to_string())
}
