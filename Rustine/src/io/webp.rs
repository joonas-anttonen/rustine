#![allow(dead_code)]

use super::webp_ffi as rwebp;
use std::ptr;

/// A safe wrapper around the native WebP decoder.
/// Automatically frees resources when dropped.
pub struct WebPDecoder {
    decoder: *mut rwebp::RwpDecoder,
    width: u32,
    height: u32,
    frame_count: u32,
    loop_count: u32,
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
        let mut decoder: *mut rwebp::RwpDecoder = ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut frame_count: u32 = 0;
        let mut loop_count: u32 = 0;

        let status = unsafe {
            rwebp::rwebpDecoderCreate(
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
            rwebp::RwpStatus::Ok => Ok(WebPDecoder {
                decoder,
                width,
                height,
                frame_count,
                loop_count,
            }),
            rwebp::RwpStatus::InvalidArgument => Err("Invalid arguments provided".to_string()),
            rwebp::RwpStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            rwebp::RwpStatus::DecodeFailed => Err("Failed to decode WebP data".to_string()),
            rwebp::RwpStatus::EndOfStream => Err("Unexpected end of stream".to_string()),
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
            rwebp::rwebpDecoderNext(
                self.decoder,
                rgba_out.as_mut_ptr(),
                rgba_out.len(),
                &mut timestamp_ms,
            )
        };

        match status {
            rwebp::RwpStatus::Ok => Ok(timestamp_ms),
            rwebp::RwpStatus::InvalidArgument => Err("Invalid arguments".to_string()),
            rwebp::RwpStatus::AllocationFailed => Err("Memory allocation failed".to_string()),
            rwebp::RwpStatus::DecodeFailed => Err("Failed to decode frame".to_string()),
            rwebp::RwpStatus::EndOfStream => Err("End of stream reached".to_string()),
        }
    }

    /// Reset the decoder to the first frame
    pub fn reset(&mut self) -> Result<(), String> {
        let status = unsafe { rwebp::rwebpDecoderReset(self.decoder) };

        match status {
            rwebp::RwpStatus::Ok => Ok(()),
            rwebp::RwpStatus::InvalidArgument => Err("Invalid decoder".to_string()),
            _ => Err("Failed to reset decoder".to_string()),
        }
    }
}

impl Drop for WebPDecoder {
    fn drop(&mut self) {
        if !self.decoder.is_null() {
            unsafe {
                rwebp::rwebpDecoderDestroy(self.decoder);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data() {
        let invalid_data = b"not a webp file";
        assert!(WebPDecoder::new(invalid_data).is_err());
    }
}
