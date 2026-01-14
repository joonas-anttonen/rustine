#![allow(dead_code)]

use std::ptr;

pub enum WebPResult {
    Ok(u32),
    EndOfStream,
    Error(String),
}

/// A safe wrapper around the native WebP decoder.
/// Automatically frees resources when dropped.
pub struct WebPDecoder {
    decoder: *mut ffi::RwpDecoder,
    width: u32,
    height: u32,
    frame_count: u32,
    loop_count: u32,
}

impl Drop for WebPDecoder {
    fn drop(&mut self) {
        if !self.decoder.is_null() {
            unsafe {
                ffi::rwebpDecoderDestroy(self.decoder);
            }
        }
    }
}

impl WebPDecoder {
    /// Create a WebP decoder from raw byte data.
    ///
    /// # Arguments
    /// * `data` - The WebP file data
    ///
    /// # Returns
    /// * `Ok(WebPDecoder)` - Successfully created decoder
    /// * `Err(String)` - Error description
    pub fn new(data: &[u8]) -> Result<Self, String> {
        let mut decoder: *mut ffi::RwpDecoder = ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut frame_count: u32 = 0;
        let mut loop_count: u32 = 0;

        let status = unsafe {
            ffi::rwebpDecoderCreate(
                data.as_ptr(),
                data.len(),
                &mut decoder,
                &mut width,
                &mut height,
                &mut frame_count,
                &mut loop_count,
            )
        };

        match status {
            ffi::RwpStatus::Ok => Ok(WebPDecoder {
                decoder,
                width,
                height,
                frame_count,
                loop_count,
            }),
            ffi::RwpStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            ffi::RwpStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            ffi::RwpStatus::DecodeFailed => Err("Failed to decode WebP data".to_string()),
            ffi::RwpStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
        }
    }

    /// Get the width of the WebP animation canvas
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the WebP animation canvas
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the number of frames in the animation
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Get the animation loop count (0 means infinite)
    pub fn loop_count(&self) -> u32 {
        self.loop_count
    }

    /// Fetch the next RGBA frame.
    ///
    /// # Arguments
    /// * `rgba_out` - Output buffer for RGBA frame data (must be at least width*height*4 bytes)
    ///
    /// # Returns
    /// * `Ok(timestamp_ms)` - Frame timestamp in milliseconds
    /// * `Err(String)` - Error description or end of stream
    pub fn next_frame(&mut self, rgba_out: &mut [u8]) -> WebPResult {
        let required_size = (self.width as usize) * (self.height as usize) * 4;
        if rgba_out.len() < required_size {
            return WebPResult::Error(format!(
                "Output buffer too small: {} bytes, need {}",
                rgba_out.len(),
                required_size
            ));
        }

        let mut timestamp_ms: u32 = 0;
        let status = unsafe {
            ffi::rwebpDecoderNext(
                self.decoder,
                rgba_out.as_mut_ptr(),
                rgba_out.len(),
                &mut timestamp_ms,
            )
        };

        match status {
            ffi::RwpStatus::Ok => WebPResult::Ok(timestamp_ms),
            ffi::RwpStatus::InvalidArgument => WebPResult::Error("Invalid arguments".to_string()),
            ffi::RwpStatus::AllocationFailed => WebPResult::Error("Memory allocation failed".to_string()),
            ffi::RwpStatus::DecodeFailed => WebPResult::Error("Failed to decode frame".to_string()),
            ffi::RwpStatus::EndOfStream => WebPResult::EndOfStream,
        }
    }

    /// Reset the decoder to the first frame
    pub fn reset(&mut self) -> Result<(), String> {
        let status = unsafe { ffi::rwebpDecoderReset(self.decoder) };

        match status {
            ffi::RwpStatus::Ok => Ok(()),
            ffi::RwpStatus::InvalidArgument => Err("Invalid decoder".to_string()),
            _ => Err("Failed to reset decoder".to_string()),
        }
    }
}

mod ffi {
    /// Status codes for WebP decoder operations
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RwpStatus {
        Ok = 0,
        InvalidArgument = 1,
        AllocationFailed = 2,
        DecodeFailed = 3,
        EndOfStream = 4,
    }

    /// Opaque decoder state handle
    #[repr(C)]
    pub struct RwpDecoder {
        _private: [u8; 0],
    }

    unsafe extern "C" {
        /// Create a decoder from in-memory WebP data.
        /// Returns metadata via out parameters when successful.
        pub fn rwebpDecoderCreate(
            data: *const u8,
            size: usize,
            out_decoder: *mut *mut RwpDecoder,
            out_width: *mut u32,
            out_height: *mut u32,
            out_frame_count: *mut u32,
            out_loop_count: *mut u32,
        ) -> RwpStatus;

        /// Fetch the next RGBA frame. The buffer must be at least width * height * 4 bytes.
        pub fn rwebpDecoderNext(
            decoder: *mut RwpDecoder,
            rgba_out: *mut u8,
            rgba_capacity: usize,
            out_timestamp_ms: *mut u32,
        ) -> RwpStatus;

        /// Reset the decoder to the first frame.
        pub fn rwebpDecoderReset(decoder: *mut RwpDecoder) -> RwpStatus;

        /// Destroy the decoder and free resources.
        pub fn rwebpDecoderDestroy(decoder: *mut RwpDecoder);
    }
}
