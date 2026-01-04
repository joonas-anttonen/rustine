#![allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]

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

    /// Fetch the next RGBA frame. The buffer must be at least width*height*4 bytes.
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