#include "rustine-webp.hpp"

#include <cstring>

#include "webp/decode.h"
#include "webp/encode.h"
#include "webp/demux.h"

struct rwebp_decoder {
	WebPData data;
	WebPAnimDecoder* decoder;
	uint8_t* owned_buffer;
	uint32_t width;
	uint32_t height;
	uint32_t frame_count;
	uint32_t loop_count;
};

static void rwp_free_decoder_internal(rwebp_decoder* dec) {
	if (!dec) {
		return;
	}
	if (dec->decoder) {
		WebPAnimDecoderDelete(dec->decoder);
		dec->decoder = nullptr;
	}
	// Don't free dec->data.bytes here - it points to owned_buffer which is freed separately
	dec->data.bytes = nullptr;
	dec->data.size = 0;
	if (dec->owned_buffer) {
		WebPFree(dec->owned_buffer);
		dec->owned_buffer = nullptr;
	}
}

extern "C" rwebp_status rwebpDecoderCreate(const uint8_t* data, size_t size, rwebp_decoder** out_decoder,
										   uint32_t* out_width, uint32_t* out_height,
										   uint32_t* out_frame_count, uint32_t* out_loop_count) {
	if (!data || size == 0 || !out_decoder || !out_width || !out_height || !out_frame_count || !out_loop_count) {
		return RWEBP_STATUS_INVALID_ARGUMENT;
	}

	rwebp_decoder* dec = reinterpret_cast<rwebp_decoder*>(WebPMalloc(sizeof(rwebp_decoder)));
	if (!dec) {
		return RWEBP_STATUS_ALLOCATION_FAILED;
	}
	
	// Initialize all fields
	dec->decoder = nullptr;
	dec->owned_buffer = nullptr;
	dec->data.bytes = nullptr;
	dec->data.size = 0;
	dec->width = 0;
	dec->height = 0;
	dec->frame_count = 0;
	dec->loop_count = 0;
	
	dec->owned_buffer = reinterpret_cast<uint8_t*>(WebPMalloc(size));
	if (!dec->owned_buffer) {
		WebPFree(dec);
		return RWEBP_STATUS_ALLOCATION_FAILED;
	}

	std::memcpy(dec->owned_buffer, data, size);
	dec->data.bytes = dec->owned_buffer;
	dec->data.size = size;

	WebPAnimDecoderOptions opts;
	WebPAnimDecoderOptionsInit(&opts);
	opts.color_mode = MODE_RGBA;
	opts.use_threads = 0;

	dec->decoder = WebPAnimDecoderNew(&dec->data, &opts);
	if (!dec->decoder) {
		rwp_free_decoder_internal(dec);
		WebPFree(dec);
		return RWEBP_STATUS_DECODE_FAILED;
	}

	WebPAnimInfo info;
	if (!WebPAnimDecoderGetInfo(dec->decoder, &info)) {
		rwp_free_decoder_internal(dec);
		WebPFree(dec);
		return RWEBP_STATUS_DECODE_FAILED;
	}

	dec->width = info.canvas_width;
	dec->height = info.canvas_height;
	dec->frame_count = info.frame_count;
	dec->loop_count = info.loop_count;

	*out_width = dec->width;
	*out_height = dec->height;
	*out_frame_count = dec->frame_count;
	*out_loop_count = dec->loop_count;
	*out_decoder = dec;

	return RWEBP_STATUS_OK;
}

extern "C" rwebp_status rwebpDecoderNext(rwebp_decoder* decoder, uint8_t* rgba_out, size_t rgba_capacity,
										 uint32_t* out_timestamp_ms) {
	if (!decoder || !rgba_out || rgba_capacity == 0 || !out_timestamp_ms) {
		return RWEBP_STATUS_INVALID_ARGUMENT;
	}

	const size_t required = static_cast<size_t>(decoder->width) * static_cast<size_t>(decoder->height) * 4u;
	if (rgba_capacity < required) {
		return RWEBP_STATUS_INVALID_ARGUMENT;
	}

	if (!WebPAnimDecoderHasMoreFrames(decoder->decoder)) {
		return RWEBP_STATUS_END_OF_STREAM;
	}

	uint8_t* frame = nullptr;
	int timestamp = 0;
	if (!WebPAnimDecoderGetNext(decoder->decoder, &frame, &timestamp)) {
		return RWEBP_STATUS_DECODE_FAILED;
	}

	std::memcpy(rgba_out, frame, required);
	*out_timestamp_ms = static_cast<uint32_t>(timestamp);
	return RWEBP_STATUS_OK;
}

extern "C" rwebp_status rwebpDecoderReset(rwebp_decoder* decoder) {
	if (!decoder) {
		return RWEBP_STATUS_INVALID_ARGUMENT;
	}
	WebPAnimDecoderReset(decoder->decoder);
	return RWEBP_STATUS_OK;
}

extern "C" void rwebpDecoderDestroy(rwebp_decoder* decoder) {
	if (!decoder) {
		return;
	}
	rwp_free_decoder_internal(decoder);
	WebPFree(decoder);
}