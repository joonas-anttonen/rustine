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
	RFFMPEG_STATUS_END_OF_STREAM = 4,
} rffmpeg_status;

/// Opaque decoder handle
typedef struct rffmpeg_decoder rffmpeg_decoder;

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

#ifdef __cplusplus
}
#endif
