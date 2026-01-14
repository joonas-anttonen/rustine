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
        let c_path = std::ffi::CString::new(path)
            .map_err(|_| "Path contains null bytes".to_string())?;
        
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
        EndOfStream = 4,
    }

    /// Opaque decoder state handle
    #[repr(C)]
    pub struct RffmpegDecoder {
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
            path: *const std::os::raw::c_char,
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
    }
}
