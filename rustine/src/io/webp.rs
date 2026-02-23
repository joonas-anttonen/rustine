#![allow(dead_code)]

use std::{ptr, slice};

pub enum WebPResult {
    Ok(u32),
    EndOfStream,
    Error(String),
}

/// A safe wrapper around the native WebP decoder.
/// Automatically frees resources when dropped.
pub struct WebPDecoder {
    decoder: *mut ffi::WebPAnimDecoder,
    _data: Vec<u8>,
    width: u32,
    height: u32,
    frame_count: u32,
    loop_count: u32,
}

impl Drop for WebPDecoder {
    fn drop(&mut self) {
        if !self.decoder.is_null() {
            unsafe {
                ffi::WebPAnimDecoderDelete(self.decoder);
            }
            self.decoder = ptr::null_mut();
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
        if data.is_empty() {
            return Err("Invalid arguments provided".to_string());
        }

        let owned_data = data.to_vec();
        let webp_data = ffi::WebPData {
            bytes: owned_data.as_ptr(),
            size: owned_data.len(),
        };
        let decoder = unsafe {
            ffi::WebPAnimDecoderNewInternal(
                &webp_data,
                ptr::null(),
                ffi::WEBP_DEMUX_ABI_VERSION,
            )
        };
        if decoder.is_null() {
            return Err("Failed to decode WebP data".to_string());
        }

        let mut info = ffi::WebPAnimInfo::default();
        let info_ok = unsafe { ffi::WebPAnimDecoderGetInfo(decoder, &mut info) } != 0;
        if !info_ok {
            unsafe {
                ffi::WebPAnimDecoderDelete(decoder);
            }
            Err("Failed to get WebP animation info".to_string())
        } else {
            Ok(WebPDecoder {
                decoder,
                _data: owned_data,
                width: info.canvas_width,
                height: info.canvas_height,
                frame_count: info.frame_count,
                loop_count: info.loop_count,
            })
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
        let Some(required_size) = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            return WebPResult::Error("Image dimensions overflow output size".to_string());
        };

        if rgba_out.len() < required_size {
            return WebPResult::Error(format!(
                "Output buffer too small: {} bytes, need {}",
                rgba_out.len(),
                required_size
            ));
        }

        if unsafe { ffi::WebPAnimDecoderHasMoreFrames(self.decoder) } == 0 {
            return WebPResult::EndOfStream;
        }

        let mut frame_data: *mut u8 = ptr::null_mut();
        let mut timestamp_ms: i32 = 0;
        let get_next_ok =
            unsafe { ffi::WebPAnimDecoderGetNext(self.decoder, &mut frame_data, &mut timestamp_ms) } != 0;
        if !get_next_ok {
            return WebPResult::Error("Failed to decode frame".to_string());
        }
        if frame_data.is_null() {
            return WebPResult::Error("Decoder returned null frame buffer".to_string());
        }

        let frame = unsafe { slice::from_raw_parts(frame_data as *const u8, required_size) };
        rgba_out[..required_size].copy_from_slice(frame);

        if timestamp_ms < 0 {
            WebPResult::Error("Decoder returned negative timestamp".to_string())
        } else {
            WebPResult::Ok(timestamp_ms as u32)
        }
    }

    /// Reset the decoder to the first frame
    pub fn reset(&mut self) -> Result<(), String> {
        if self.decoder.is_null() {
            return Err("Invalid decoder".to_string());
        }

        unsafe {
            ffi::WebPAnimDecoderReset(self.decoder);
        }
        Ok(())
    }
}

mod ffi {
    pub const WEBP_DEMUX_ABI_VERSION: i32 = 0x0107;

    #[repr(C)]
    pub struct WebPData {
        pub bytes: *const u8,
        pub size: usize,
    }

    #[repr(C)]
    pub struct WebPAnimDecoderOptions {
        pub color_mode: i32,
        pub use_threads: i32,
        pub padding: [u32; 7],
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct WebPAnimInfo {
        pub canvas_width: u32,
        pub canvas_height: u32,
        pub loop_count: u32,
        pub bgcolor: u32,
        pub frame_count: u32,
        pub pad: [u32; 4],
    }

    #[repr(C)]
    pub struct WebPAnimDecoder {
        _private: [u8; 0],
    }

    unsafe extern "C" {
        pub fn WebPAnimDecoderNewInternal(
            webp_data: *const WebPData,
            dec_options: *const WebPAnimDecoderOptions,
            abi_version: i32,
        ) -> *mut WebPAnimDecoder;
        pub fn WebPAnimDecoderGetInfo(dec: *const WebPAnimDecoder, info: *mut WebPAnimInfo) -> i32;
        pub fn WebPAnimDecoderGetNext(
            dec: *mut WebPAnimDecoder,
            buf: *mut *mut u8,
            timestamp: *mut i32,
        ) -> i32;
        pub fn WebPAnimDecoderHasMoreFrames(dec: *const WebPAnimDecoder) -> i32;
        pub fn WebPAnimDecoderReset(dec: *mut WebPAnimDecoder);
        pub fn WebPAnimDecoderDelete(dec: *mut WebPAnimDecoder);
    }
}
