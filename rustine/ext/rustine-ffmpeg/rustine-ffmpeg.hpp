#pragma once

#include <cstdint>
#include <cstddef>

#ifdef __cplusplus
extern "C" {
#endif

/// Status codes for FFmpeg decoder operations
typedef enum {
	RFFMPEG_STATUS_OK = 0,
	RFFMPEG_STATUS_INVALID_ARGUMENT = 1,
	RFFMPEG_STATUS_ALLOCATION_FAILED = 2,
	RFFMPEG_STATUS_DECODE_FAILED = 3,
	RFFMPEG_STATUS_ENCODE_FAILED = 4,
	RFFMPEG_STATUS_END_OF_STREAM = 5,
} rffmpeg_status;

typedef enum {
	RFFMPEG_PIXFMT_BAYER_RGGB8 = 0,
	RFFMPEG_PIXFMT_BAYER_BGGR8 = 1,
	RFFMPEG_PIXFMT_BAYER_GBRG8 = 2,
	RFFMPEG_PIXFMT_BAYER_GRBG8 = 3,
	RFFMPEG_PIXFMT_BAYER_RGGB16 = 4,
	RFFMPEG_PIXFMT_BAYER_BGGR16 = 5,
	RFFMPEG_PIXFMT_BAYER_GBRG16 = 6,
	RFFMPEG_PIXFMT_BAYER_GRBG16 = 7,
	RFFMPEG_PIXFMT_MONO8 = 8,
	RFFMPEG_PIXFMT_MONO10 = 9,
	RFFMPEG_PIXFMT_MONO12 = 10,
	RFFMPEG_PIXFMT_MONO16 = 11,
	RFFMPEG_PIXFMT_BAYER_RGGB10 = 12,
	RFFMPEG_PIXFMT_BAYER_BGGR10 = 13,
	RFFMPEG_PIXFMT_BAYER_GBRG10 = 14,
	RFFMPEG_PIXFMT_BAYER_GRBG10 = 15,
	RFFMPEG_PIXFMT_BAYER_RGGB12 = 16,
	RFFMPEG_PIXFMT_BAYER_BGGR12 = 17,
	RFFMPEG_PIXFMT_BAYER_GBRG12 = 18,
	RFFMPEG_PIXFMT_BAYER_GRBG12 = 19,
	RFFMPEG_PIXFMT_MONO10_PACKED = 20,
	RFFMPEG_PIXFMT_MONO12_PACKED = 21,
	RFFMPEG_PIXFMT_BAYER_RGGB10_PACKED = 22,
	RFFMPEG_PIXFMT_BAYER_BGGR10_PACKED = 23,
	RFFMPEG_PIXFMT_BAYER_GBRG10_PACKED = 24,
	RFFMPEG_PIXFMT_BAYER_GRBG10_PACKED = 25,
	RFFMPEG_PIXFMT_BAYER_RGGB12_PACKED = 26,
	RFFMPEG_PIXFMT_BAYER_BGGR12_PACKED = 27,
	RFFMPEG_PIXFMT_BAYER_GBRG12_PACKED = 28,
	RFFMPEG_PIXFMT_BAYER_GRBG12_PACKED = 29,
} rffmpeg_pixel_format;

/// Opaque decoder handle
typedef struct rffmpeg_decoder rffmpeg_decoder;
/// Opaque encoder handle
typedef struct rffmpeg_encoder rffmpeg_encoder;
/// Opaque JPEG encoder handle
typedef struct rffmpeg_jpeg_encoder rffmpeg_jpeg_encoder;

/// Create a video decoder from in-memory data (e.g., MP4 file).
/// Returns metadata via out parameters when successful.
rffmpeg_status rffmpegDecoderCreate(const uint8_t* data, size_t size, rffmpeg_decoder** out_decoder,
									uint32_t* out_width, uint32_t* out_height, uint32_t* out_frame_count,
									double* out_fps, double* out_duration_sec);

/// Create a video decoder from a file or URL path (e.g., "/path/to/video.mp4" or "http://example.com/video.mp4").
/// Returns metadata via out parameters when successful.
rffmpeg_status rffmpegDecoderCreateFromPath(const char* path, rffmpeg_decoder** out_decoder,
											uint32_t* out_width, uint32_t* out_height, uint32_t* out_frame_count,
											double* out_fps, double* out_duration_sec);

/// Fetch the next RGBA frame. The buffer must be at least width * height * 4 bytes.
/// Returns the timestamp in milliseconds via out_timestamp_ms.
rffmpeg_status rffmpegDecoderNext(rffmpeg_decoder* decoder, uint8_t* rgba_out, size_t rgba_capacity,
								  uint32_t* out_timestamp_ms);

/// Reset the decoder to the first frame.
rffmpeg_status rffmpegDecoderReset(rffmpeg_decoder* decoder);

/// Destroy the decoder and free resources.
void rffmpegDecoderDestroy(rffmpeg_decoder* decoder);

/// Create an encoder that writes a video file at `path` (e.g. "out.mp4").
/// `fps` selects the output framerate. `bitrate` may be 0 to use a reasonable default.
rffmpeg_status rffmpegEncoderCreateToPath(const char* path,
	uint32_t width,
 	uint32_t height,
 	double fps,
 	int64_t bitrate,
 	rffmpeg_encoder** out_encoder);

/// Feed a single RGBA frame (tightly-packed width*height*4 bytes) into the encoder.
/// Frames are written into the file specified at create time. This function encodes
/// and muxes packets as they become available.
rffmpeg_status rffmpegEncoderEncode(rffmpeg_encoder* encoder,
	const uint8_t* rgba_in,
 	size_t rgba_size);

/// Finish encoding: flushes delayed packets, writes the trailer, and ensures the
/// output file is finalized. After this call you should call `rffmpegEncoderDestroy`.
rffmpeg_status rffmpegEncoderFinish(rffmpeg_encoder* encoder);

/// Destroy the encoder and free resources.
void rffmpegEncoderDestroy(rffmpeg_encoder* encoder);

/// Create a JPEG encoder for in-memory frames.
rffmpeg_status rffmpegJpegEncoderCreate(uint32_t width,
	uint32_t height,
	int quality,
	rffmpeg_jpeg_encoder** out_encoder);

/// Encode a single RGBA frame to a JPEG buffer.
/// The returned buffer must be freed with `rffmpegJpegFreeBuffer`.
rffmpeg_status rffmpegJpegEncode(rffmpeg_jpeg_encoder* encoder,
	const uint8_t* rgba_in,
	size_t rgba_size,
	uint8_t** out_buf,
	size_t* out_size);

/// Free a JPEG buffer allocated by `rffmpegJpegEncode`.
void rffmpegJpegFreeBuffer(uint8_t* buffer);

/// Destroy the JPEG encoder and free resources.
void rffmpegJpegEncoderDestroy(rffmpeg_jpeg_encoder* encoder);

/// Demosaic a Bayer RGGB8 image to RGBA using swscale.
/// Input buffer should be width * height bytes.
/// Output buffer must be at least width * height * 4 bytes.
rffmpeg_status rffmpegDemosaicBayerRG8(const uint8_t* bayer_in,
	size_t bayer_size,
	uint32_t width,
	uint32_t height,
	uint8_t* rgba_out,
	size_t rgba_capacity);

/// Convert a Bayer/mono image to RGBA using swscale.
/// Input buffer size depends on the format (e.g. width*height for 8-bit, width*height*2 for 10/12/16-bit).
/// Output buffer must be at least width * height * 4 bytes.
rffmpeg_status rffmpegDemosaicToRgba(const uint8_t* input,
	size_t input_size,
	uint32_t width,
	uint32_t height,
	rffmpeg_pixel_format input_format,
	uint8_t* rgba_out,
	size_t rgba_capacity);

#ifdef __cplusplus
}
#endif
