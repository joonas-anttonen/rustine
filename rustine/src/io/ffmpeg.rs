#![allow(dead_code)]

use std::ptr;

/// A safe wrapper around the native FFmpeg video decoder.
/// Automatically frees resources when dropped.
pub struct VideoDecoder {
    decoder: *mut ffi::RffmpegDecoder,
    width: u32,
    height: u32,
    frame_count: u32,
    fps: f64,
    duration_sec: f64,
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        if !self.decoder.is_null() {
            unsafe {
                ffi::rffmpegDecoderDestroy(self.decoder);
            }
        }
    }
}

impl VideoDecoder {
    /// Create a video decoder from raw byte data (e.g., MP4 file).
    ///
    /// # Arguments
    /// * `data` - The video file data
    ///
    /// # Returns
    /// * `Ok(VideoDecoder)` - Successfully created decoder
    /// * `Err(String)` - Error description
    pub fn new(data: &[u8]) -> Result<Self, String> {
        let mut decoder: *mut ffi::RffmpegDecoder = ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut frame_count: u32 = 0;
        let mut fps: f64 = 0.0;
        let mut duration_sec: f64 = 0.0;

        let status = unsafe {
            ffi::rffmpegDecoderCreate(
                data.as_ptr(),
                data.len(),
                &mut decoder,
                &mut width,
                &mut height,
                &mut frame_count,
                &mut fps,
                &mut duration_sec,
            )
        };

        match status {
            ffi::RffmpegStatus::Ok => Ok(VideoDecoder {
                decoder,
                width,
                height,
                frame_count,
                fps,
                duration_sec,
            }),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Failed to decode video data".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
        }
    }

    /// Create a video decoder from a file or URL path.
    ///
    /// # Arguments
    /// * `path` - File path or URL (e.g., "/path/to/video.mp4" or "http://example.com/video.mp4")
    ///
    /// # Returns
    /// * `Ok(VideoDecoder)` - Successfully created decoder
    /// * `Err(String)` - Error description
    pub fn from_path(path: &str) -> Result<Self, String> {
        let c_path =
            std::ffi::CString::new(path).map_err(|_| "Path contains null bytes".to_string())?;

        let mut decoder: *mut ffi::RffmpegDecoder = ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut frame_count: u32 = 0;
        let mut fps: f64 = 0.0;
        let mut duration_sec: f64 = 0.0;

        let status = unsafe {
            ffi::rffmpegDecoderCreateFromPath(
                c_path.as_ptr(),
                &mut decoder,
                &mut width,
                &mut height,
                &mut frame_count,
                &mut fps,
                &mut duration_sec,
            )
        };

        match status {
            ffi::RffmpegStatus::Ok => Ok(VideoDecoder {
                decoder,
                width,
                height,
                frame_count,
                fps,
                duration_sec,
            }),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Failed to decode video at path".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
        }
    }

    /// Get the width of the video
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the video
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the number of frames in the video (may be 0 if unknown)
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Get the frames per second
    pub fn fps(&self) -> f64 {
        self.fps
    }

    /// Get the duration in seconds
    pub fn duration_sec(&self) -> f64 {
        self.duration_sec
    }

    /// Fetch the next RGBA frame.
    ///
    /// # Arguments
    /// * `rgba_out` - Output buffer for RGBA frame data (must be at least width*height*4 bytes)
    ///
    /// # Returns
    /// * `Ok(timestamp_ms)` - Frame timestamp in milliseconds
    /// * `Err(String)` - Error description or end of stream
    pub fn next_frame(&mut self, rgba_out: &mut [u8]) -> Result<u32, String> {
        let required_size = (self.width as usize) * (self.height as usize) * 4;
        if rgba_out.len() < required_size {
            return Err(format!(
                "Output buffer too small: {} bytes, need {}",
                rgba_out.len(),
                required_size
            ));
        }

        let mut timestamp_ms: u32 = 0;
        let status = unsafe {
            ffi::rffmpegDecoderNext(
                self.decoder,
                rgba_out.as_mut_ptr(),
                rgba_out.len(),
                &mut timestamp_ms,
            )
        };

        match status {
            ffi::RffmpegStatus::Ok => Ok(timestamp_ms),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Failed to decode frame".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("End of stream reached".to_string()),
        }
    }

    /// Reset the decoder to the first frame
    pub fn reset(&mut self) -> Result<(), String> {
        let status = unsafe { ffi::rffmpegDecoderReset(self.decoder) };

        match status {
            ffi::RffmpegStatus::Ok => Ok(()),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid decoder".to_string()),
            _ => Err("Failed to reset decoder".to_string()),
        }
    }
}

mod ffi {
    /// Status codes for FFmpeg decoder operations
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RffmpegStatus {
        Ok = 0,
        InvalidArgument = 1,
        AllocationFailed = 2,
        DecodeFailed = 3,
        EncodeFailed = 4,
        EndOfStream = 5,
    }

    /// Opaque decoder state handle
    #[repr(C)]
    pub struct RffmpegDecoder {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct RffmpegEncoder {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct RffmpegJpegEncoder {
        _private: [u8; 0],
    }

    unsafe extern "C" {
        /// Create a decoder from in-memory video data.
        /// Returns metadata via out parameters when successful.
        pub fn rffmpegDecoderCreate(
            data: *const u8,
            size: usize,
            out_decoder: *mut *mut RffmpegDecoder,
            out_width: *mut u32,
            out_height: *mut u32,
            out_frame_count: *mut u32,
            out_fps: *mut f64,
            out_duration_sec: *mut f64,
        ) -> RffmpegStatus;
        /// Create a decoder from a file or URL path.
        /// Returns metadata via out parameters when successful.
        pub fn rffmpegDecoderCreateFromPath(
            path: *const std::ffi::c_char,
            out_decoder: *mut *mut RffmpegDecoder,
            out_width: *mut u32,
            out_height: *mut u32,
            out_frame_count: *mut u32,
            out_fps: *mut f64,
            out_duration_sec: *mut f64,
        ) -> RffmpegStatus;
        /// Fetch the next RGBA frame. The buffer must be at least width * height * 4 bytes.
        pub fn rffmpegDecoderNext(
            decoder: *mut RffmpegDecoder,
            rgba_out: *mut u8,
            rgba_capacity: usize,
            out_timestamp_ms: *mut u32,
        ) -> RffmpegStatus;
        /// Reset the decoder to the first frame.
        pub fn rffmpegDecoderReset(decoder: *mut RffmpegDecoder) -> RffmpegStatus;
        /// Destroy the decoder and free resources.
        pub fn rffmpegDecoderDestroy(decoder: *mut RffmpegDecoder);

        pub fn rffmpegEncoderCreateToPath(
            path: *const std::ffi::c_char,
            width: u32,
            height: u32,
            fps: f64,
            bitrate: u64,
            out_encoder: *mut *mut RffmpegEncoder,
        ) -> RffmpegStatus;
        pub fn rffmpegEncoderEncode(
            encoder: *mut RffmpegEncoder,
            rgba_in: *const u8,
            rgba_size: usize,
        ) -> RffmpegStatus;
        pub fn rffmpegEncoderFinish(encoder: *mut RffmpegEncoder) -> RffmpegStatus;
        pub fn rffmpegEncoderDestroy(encoder: *mut RffmpegEncoder);

        pub fn rffmpegJpegEncoderCreate(
            width: u32,
            height: u32,
            quality: i32,
            out_encoder: *mut *mut RffmpegJpegEncoder,
        ) -> RffmpegStatus;
        pub fn rffmpegJpegEncode(
            encoder: *mut RffmpegJpegEncoder,
            rgba_in: *const u8,
            rgba_size: usize,
            out_buf: *mut *mut u8,
            out_size: *mut usize,
        ) -> RffmpegStatus;
        pub fn rffmpegJpegFreeBuffer(buffer: *mut u8);
        pub fn rffmpegJpegEncoderDestroy(encoder: *mut RffmpegJpegEncoder);

        /// Demosaic a Bayer RGGB8 image to RGBA using swscale.
        pub fn rffmpegDemosaicBayerRG8(
            bayer_in: *const u8,
            bayer_size: usize,
            width: u32,
            height: u32,
            rgba_out: *mut u8,
            rgba_capacity: usize,
        ) -> RffmpegStatus;
    }
}

/// A safe wrapper around the native FFmpeg video encoder.
/// Automatically frees resources when dropped.
pub struct VideoEncoder {
    encoder: *mut ffi::RffmpegEncoder,
    width: u32,
    height: u32,
    fps: f64,
}

/// A safe wrapper around an in-memory JPEG encoder.
pub struct JpegEncoder {
    encoder: *mut ffi::RffmpegJpegEncoder,
    width: u32,
    height: u32,
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        if !self.encoder.is_null() {
            unsafe {
                ffi::rffmpegEncoderDestroy(self.encoder);
            }
        }
    }
}

impl VideoEncoder {
    /// Create an encoder writing to `path` with the provided parameters.
    pub fn create_to_path(
        path: &str,
        width: u32,
        height: u32,
        fps: f64,
        bitrate: u64,
    ) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|_| "Path contains null bytes".to_string())?;

        let mut encoder: *mut ffi::RffmpegEncoder = std::ptr::null_mut();

        let status = unsafe {
            ffi::rffmpegEncoderCreateToPath(
                c_path.as_ptr(),
                width,
                height,
                fps,
                bitrate,
                &mut encoder,
            )
        };

        match status {
            ffi::RffmpegStatus::Ok => Ok(VideoEncoder {
                encoder,
                width,
                height,
                fps,
            }),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Decoder failure during encode".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to initialize encoder".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
        }
    }

    /// Encode a single RGBA frame. `rgba_in` must be exactly `width*height*4` bytes.
    pub fn encode_frame(&mut self, rgba_in: &[u8]) -> Result<(), String> {
        let required_size = (self.width as usize) * (self.height as usize) * 4;
        if rgba_in.len() < required_size {
            return Err(format!(
                "Input buffer too small: {} bytes, need {}",
                rgba_in.len(),
                required_size
            ));
        }

        let status = unsafe { ffi::rffmpegEncoderEncode(self.encoder, rgba_in.as_ptr(), rgba_in.len()) };

        match status {
            ffi::RffmpegStatus::Ok => Ok(()),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Decode failure during encode".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("End of stream".to_string()),
        }
    }

    /// Finalize the encoding and flush remaining packets to disk.
    pub fn finish(&mut self) -> Result<(), String> {
        let status = unsafe { ffi::rffmpegEncoderFinish(self.encoder) };

        match status {
            ffi::RffmpegStatus::Ok => Ok(()),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid encoder".to_string()),
            _ => Err("Failed to finish encoding".to_string()),
        }
    }
}

impl Drop for JpegEncoder {
    fn drop(&mut self) {
        if !self.encoder.is_null() {
            unsafe {
                ffi::rffmpegJpegEncoderDestroy(self.encoder);
            }
        }
    }
}

impl JpegEncoder {
    pub fn new(width: u32, height: u32, quality: i32) -> Result<Self, String> {
        let mut encoder: *mut ffi::RffmpegJpegEncoder = std::ptr::null_mut();
        let status = unsafe { ffi::rffmpegJpegEncoderCreate(width, height, quality, &mut encoder) };

        match status {
            ffi::RffmpegStatus::Ok => Ok(JpegEncoder {
                encoder,
                width,
                height,
            }),
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Decoder failure during encode".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to initialize encoder".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
        }
    }

    pub fn encode_rgba(&mut self, rgba_in: &[u8]) -> Result<Vec<u8>, String> {
        let required_size = (self.width as usize) * (self.height as usize) * 4;
        if rgba_in.len() < required_size {
            return Err(format!(
                "Input buffer too small: {} bytes, need {}",
                rgba_in.len(),
                required_size
            ));
        }

        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_size: usize = 0;
        let status = unsafe {
            ffi::rffmpegJpegEncode(
                self.encoder,
                rgba_in.as_ptr(),
                rgba_in.len(),
                &mut out_buf,
                &mut out_size,
            )
        };

        match status {
            ffi::RffmpegStatus::Ok => {
                if out_buf.is_null() || out_size == 0 {
                    return Err("Empty JPEG buffer".to_string());
                }
                let bytes = unsafe { std::slice::from_raw_parts(out_buf, out_size) }.to_vec();
                unsafe { ffi::rffmpegJpegFreeBuffer(out_buf) };
                Ok(bytes)
            }
            ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments".to_string()),
            ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RffmpegStatus::DecodeFailed => Err("Decoder failure during encode".to_string()),
            ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
            ffi::RffmpegStatus::EndOfStream => Err("End of stream".to_string()),
        }
    }
}

/// Demosaic a Bayer RGGB8 image to RGBA using FFmpeg's swscale.
///
/// # Arguments
/// * `bayer_in` - Input Bayer RGGB8 image data (width * height bytes)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
/// * `Ok(Vec<u8>)` - RGBA image data (width * height * 4 bytes)
/// * `Err(String)` - Error description
///
/// # Example
/// ```ignore
/// let bayer_data = vec![0u8; 1920 * 1080];
/// let rgba = demosaic_bayer_rg8(&bayer_data, 1920, 1080)?;
/// ```
pub fn demosaic_bayer_rg8(bayer_in: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let expected_size = (width as usize) * (height as usize);
    if bayer_in.len() < expected_size {
        return Err(format!(
            "Input buffer too small: {} bytes, need {}",
            bayer_in.len(),
            expected_size
        ));
    }

    let mut rgba_out = vec![0u8; expected_size * 4];

    let status = unsafe {
        ffi::rffmpegDemosaicBayerRG8(
            bayer_in.as_ptr(),
            bayer_in.len(),
            width,
            height,
            rgba_out.as_mut_ptr(),
            rgba_out.len(),
        )
    };

    match status {
        ffi::RffmpegStatus::Ok => Ok(rgba_out),
        ffi::RffmpegStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
        ffi::RffmpegStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
        ffi::RffmpegStatus::DecodeFailed => Err("Failed to demosaic image".to_string()),
        ffi::RffmpegStatus::EncodeFailed => Err("Failed to encode frame".to_string()),
        ffi::RffmpegStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
    }
}
