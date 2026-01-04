#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Status codes for operations
typedef enum rwebp_status {
	RWEBP_STATUS_OK = 0,
	RWEBP_STATUS_INVALID_ARGUMENT = 1,
	RWEBP_STATUS_ALLOCATION_FAILED = 2,
	RWEBP_STATUS_DECODE_FAILED = 3,
	RWEBP_STATUS_END_OF_STREAM = 4,
} rwebp_status;

// Opaque decoder state
typedef struct rwebp_decoder rwebp_decoder;

// Create a decoder from in-memory WebP data.
// Returns metadata via out parameters when successful.
rwebp_status rwebpDecoderCreate(const uint8_t* data, size_t size, rwebp_decoder** out_decoder,
							  uint32_t* out_width, uint32_t* out_height, uint32_t* out_frame_count,
							  uint32_t* out_loop_count);

// Fetch the next RGBA frame. The buffer must be at least width*height*4 bytes.
rwebp_status rwebpDecoderNext(rwebp_decoder* decoder, uint8_t* rgba_out, size_t rgba_capacity,
							uint32_t* out_timestamp_ms);

// Reset the decoder to the first frame.
rwebp_status rwebpDecoderReset(rwebp_decoder* decoder);

// Destroy the decoder and free resources.
void rwebpDecoderDestroy(rwebp_decoder* decoder);

#ifdef __cplusplus
}
#endif
