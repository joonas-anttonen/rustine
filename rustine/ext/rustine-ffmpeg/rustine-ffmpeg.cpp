#include "rustine-ffmpeg.hpp"

#include <cstring>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/imgutils.h>
#include <libswscale/swscale.h>
}

struct rffmpeg_decoder {
    AVFormatContext* format_ctx = nullptr;
    AVCodecContext* codec_ctx = nullptr;
    SwsContext* sws_ctx = nullptr;
    AVPacket* packet = nullptr;
    AVFrame* frame = nullptr;
    AVFrame* rgba_frame = nullptr;
    uint8_t* rgba_buffer = nullptr;

    int video_stream_index = -1;
    uint32_t width = 0;
    uint32_t height = 0;
    uint32_t frame_count = 0;
    double fps = 0.0;
    double duration_sec = 0.0;

    // For reading from memory
    AVIOContext* avio_ctx = nullptr;
    uint8_t* avio_buffer = nullptr;
    const uint8_t* data_ptr = nullptr;
    size_t data_size = 0;
    size_t data_pos = 0;
};

// Custom read callback for reading from memory
static int read_packet(void* opaque, uint8_t* buf, int buf_size) {
    rffmpeg_decoder* dec = static_cast<rffmpeg_decoder*>(opaque);

    if (dec->data_pos >= dec->data_size) {
        return AVERROR_EOF;
    }

    size_t bytes_to_read = buf_size;
    if (dec->data_pos + bytes_to_read > dec->data_size) {
        bytes_to_read = dec->data_size - dec->data_pos;
    }

    memcpy(buf, dec->data_ptr + dec->data_pos, bytes_to_read);
    dec->data_pos += bytes_to_read;

    return bytes_to_read;
}

// Custom seek callback for seeking in memory
static int64_t seek_packet(void* opaque, int64_t offset, int whence) {
    rffmpeg_decoder* dec = static_cast<rffmpeg_decoder*>(opaque);

    int64_t new_pos = 0;

    switch (whence) {
    case SEEK_SET:
        new_pos = offset;
        break;
    case SEEK_CUR:
        new_pos = dec->data_pos + offset;
        break;
    case SEEK_END:
        new_pos = dec->data_size + offset;
        break;
    case AVSEEK_SIZE:
        return dec->data_size;
    default:
        return -1;
    }

    if (new_pos < 0 || new_pos > static_cast<int64_t>(dec->data_size)) {
        return -1;
    }

    dec->data_pos = new_pos;
    return new_pos;
}

static void rffmpeg_free_decoder_internal(rffmpeg_decoder* dec) {
    if (!dec) {
        return;
    }

    if (dec->rgba_frame) {
        av_frame_free(&dec->rgba_frame);
    }

    if (dec->frame) {
        av_frame_free(&dec->frame);
    }

    if (dec->rgba_buffer) {
        av_free(dec->rgba_buffer);
        dec->rgba_buffer = nullptr;
    }

    if (dec->packet) {
        av_packet_free(&dec->packet);
    }

    if (dec->sws_ctx) {
        sws_freeContext(dec->sws_ctx);
        dec->sws_ctx = nullptr;
    }

    if (dec->codec_ctx) {
        avcodec_free_context(&dec->codec_ctx);
    }

    if (dec->format_ctx) {
        avformat_close_input(&dec->format_ctx);
    }

    if (dec->avio_ctx) {
        // If an AVIO context was created, it takes ownership of the buffer and
        // will free it when the context is freed. Do not free the buffer
        // separately to avoid double-free.
        avio_context_free(&dec->avio_ctx);
        dec->avio_buffer = nullptr;
    }
}

// --- Encoder (mirror of decoder pattern) ---

struct rffmpeg_encoder {
    AVCodecContext* codec_ctx = nullptr;
    SwsContext* sws_ctx = nullptr;
    AVPacket* packet = nullptr;
    AVFrame* frame = nullptr;
    uint8_t* frame_buffer = nullptr;

    uint32_t width = 0;
    uint32_t height = 0;
    int64_t pts = 0;
    // Muxer / output
    AVFormatContext* fmt_ctx = nullptr;
    AVStream* stream = nullptr;
    bool file_opened = false;
};

static void rffmpeg_free_encoder_internal(rffmpeg_encoder* enc) {
    if (!enc) {
        return;
    }

    if (enc->frame) {
        av_frame_free(&enc->frame);
    }

    if (enc->packet) {
        av_packet_free(&enc->packet);
    }

    if (enc->frame_buffer) {
        av_free(enc->frame_buffer);
        enc->frame_buffer = nullptr;
    }

    if (enc->sws_ctx) {
        sws_freeContext(enc->sws_ctx);
        enc->sws_ctx = nullptr;
    }

    if (enc->codec_ctx) {
        avcodec_free_context(&enc->codec_ctx);
    }

    if (enc->fmt_ctx) {
        if (enc->file_opened && enc->fmt_ctx->pb) {
            avio_closep(&enc->fmt_ctx->pb);
        }
        avformat_free_context(enc->fmt_ctx);
        enc->fmt_ctx = nullptr;
        enc->stream = nullptr;
    }
}

extern "C" rffmpeg_status rffmpegEncoderCreate(uint32_t width, uint32_t height, rffmpeg_encoder** out_encoder) {
    if (!out_encoder || width == 0 || height == 0) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    rffmpeg_encoder* enc = new (std::nothrow) rffmpeg_encoder();
    if (!enc) {
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    memset(enc, 0, sizeof(rffmpeg_encoder));
    enc->width = width;
    enc->height = height;

    const AVCodec* codec = avcodec_find_encoder(AV_CODEC_ID_MJPEG);
    if (!codec) {
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->codec_ctx = avcodec_alloc_context3(codec);
    if (!enc->codec_ctx) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->codec_ctx->bit_rate = 400000;
    enc->codec_ctx->width = width;
    enc->codec_ctx->height = height;
    enc->codec_ctx->time_base = AVRational{1, 25};
    enc->codec_ctx->framerate = AVRational{25, 1};

    // Prefer YUVJ420P for MJPEG if available, fall back to codec default
    enc->codec_ctx->pix_fmt = AV_PIX_FMT_YUVJ420P;
    if (codec->pix_fmts) {
        bool supported = false;
        for (const AVPixelFormat* p = codec->pix_fmts; *p != AV_PIX_FMT_NONE; ++p) {
            if (*p == enc->codec_ctx->pix_fmt) {
                supported = true;
                break;
            }
        }
        if (!supported) {
            enc->codec_ctx->pix_fmt = codec->pix_fmts[0];
        }
    }

    if (avcodec_open2(enc->codec_ctx, codec, nullptr) < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    enc->packet = av_packet_alloc();
    enc->frame = av_frame_alloc();

    if (!enc->packet || !enc->frame) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->frame->format = enc->codec_ctx->pix_fmt;
    enc->frame->width = enc->codec_ctx->width;
    enc->frame->height = enc->codec_ctx->height;

    int alloc_ret = av_image_alloc(enc->frame->data,
        enc->frame->linesize,
        enc->width,
        enc->height,
        enc->codec_ctx->pix_fmt,
        32);
    if (alloc_ret < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->frame_buffer = enc->frame->data[0];

    enc->sws_ctx = sws_getContext(enc->width,
        enc->height,
        AV_PIX_FMT_RGBA,
        enc->width,
        enc->height,
        enc->codec_ctx->pix_fmt,
        SWS_BILINEAR,
        nullptr,
        nullptr,
        nullptr);

    if (!enc->sws_ctx) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    *out_encoder = enc;
    return RFFMPEG_STATUS_OK;
}

// New API: create-to-path, encode frames written to file, finish, destroy
extern "C" rffmpeg_status rffmpegEncoderCreateToPath(const char* path,
    uint32_t width,
    uint32_t height,
    double fps,
    int64_t bitrate,
    rffmpeg_encoder** out_encoder) {
    if (!path || !out_encoder || width == 0 || height == 0 || fps <= 0.0) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    rffmpeg_encoder* enc = new (std::nothrow) rffmpeg_encoder();
    if (!enc) {
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    memset(enc, 0, sizeof(rffmpeg_encoder));
    enc->width = width;
    enc->height = height;

    // Allocate format context for output file
    AVFormatContext* fmt_ctx = nullptr;
    if (avformat_alloc_output_context2(&fmt_ctx, nullptr, nullptr, path) < 0 || !fmt_ctx) {
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->fmt_ctx = fmt_ctx;

    // Choose codec (prefer H264), fallback to MJPEG if not available
    const AVCodec* codec = avcodec_find_encoder(AV_CODEC_ID_H264);
    if (!codec) {
        codec = avcodec_find_encoder(AV_CODEC_ID_MJPEG);
    }
    if (!codec) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->codec_ctx = avcodec_alloc_context3(codec);
    if (!enc->codec_ctx) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->codec_ctx->width = width;
    enc->codec_ctx->height = height;
    enc->codec_ctx->time_base = AVRational{1, static_cast<int>(fps)};
    enc->codec_ctx->framerate = AVRational{static_cast<int>(fps), 1};
    enc->codec_ctx->bit_rate = bitrate > 0 ? bitrate : 400000;

    // set pixel format preference
    AVPixelFormat target_pix_fmt = AV_PIX_FMT_YUV420P;
    if (codec->id == AV_CODEC_ID_MJPEG) {
        // allow YUVJ420P for MJPEG if available
        target_pix_fmt = AV_PIX_FMT_YUVJ420P;
    }

    enc->codec_ctx->pix_fmt = target_pix_fmt;
    if (codec->pix_fmts) {
        bool ok = false;
        for (const AVPixelFormat* p = codec->pix_fmts; *p != AV_PIX_FMT_NONE; ++p) {
            if (*p == enc->codec_ctx->pix_fmt) {
                ok = true;
                break;
            }
        }
        if (!ok) {
            enc->codec_ctx->pix_fmt = codec->pix_fmts[0];
        }
    }

    // Add stream to format context
    AVStream* st = avformat_new_stream(fmt_ctx, nullptr);
    if (!st) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }
    enc->stream = st;

    if (fmt_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
        enc->codec_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    // Open codec
    if (avcodec_open2(enc->codec_ctx, codec, nullptr) < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    // Copy codec parameters to stream
    if (avcodec_parameters_from_context(st->codecpar, enc->codec_ctx) < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    st->time_base = enc->codec_ctx->time_base;

    enc->packet = av_packet_alloc();
    enc->frame = av_frame_alloc();
    if (!enc->packet || !enc->frame) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    enc->frame->format = enc->codec_ctx->pix_fmt;
    enc->frame->width = enc->codec_ctx->width;
    enc->frame->height = enc->codec_ctx->height;

    int alloc_ret = av_image_alloc(enc->frame->data,
        enc->frame->linesize,
        enc->width,
        enc->height,
        enc->codec_ctx->pix_fmt,
        32);
    if (alloc_ret < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }
    enc->frame_buffer = enc->frame->data[0];

    enc->sws_ctx = sws_getContext(enc->width,
        enc->height,
        AV_PIX_FMT_RGBA,
        enc->width,
        enc->height,
        enc->codec_ctx->pix_fmt,
        SWS_BILINEAR,
        nullptr,
        nullptr,
        nullptr);

    if (!enc->sws_ctx) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Open output file
    if (!(fmt_ctx->oformat->flags & AVFMT_NOFILE)) {
        if (avio_open(&fmt_ctx->pb, path, AVIO_FLAG_WRITE) < 0) {
            rffmpeg_free_encoder_internal(enc);
            delete enc;
            return RFFMPEG_STATUS_ALLOCATION_FAILED;
        }
        enc->file_opened = true;
    }

    // Write header
    if (avformat_write_header(fmt_ctx, nullptr) < 0) {
        rffmpeg_free_encoder_internal(enc);
        delete enc;
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    *out_encoder = enc;
    return RFFMPEG_STATUS_OK;
}

// helper to encode available packets and mux them
static rffmpeg_status rffmpeg_encoder_drain_packets(rffmpeg_encoder* enc) {
    if (!enc) return RFFMPEG_STATUS_INVALID_ARGUMENT;

    while (true) {
        int ret = avcodec_receive_packet(enc->codec_ctx, enc->packet);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return RFFMPEG_STATUS_OK;
        }
        if (ret < 0) {
            return RFFMPEG_STATUS_ENCODE_FAILED;
        }

        // rescale packet timestamps to stream timebase
        av_packet_rescale_ts(enc->packet, enc->codec_ctx->time_base, enc->stream->time_base);
        enc->packet->stream_index = enc->stream->index;

        if (av_interleaved_write_frame(enc->fmt_ctx, enc->packet) < 0) {
            av_packet_unref(enc->packet);
            return RFFMPEG_STATUS_ENCODE_FAILED;
        }

        av_packet_unref(enc->packet);
    }
}

extern "C" rffmpeg_status rffmpegEncoderEncode(rffmpeg_encoder* encoder,
    const uint8_t* rgba_in,
    size_t rgba_size) {
    if (!encoder || !rgba_in) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    size_t required = static_cast<size_t>(encoder->width) * encoder->height * 4;
    if (rgba_size < required) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    // Convert RGBA -> encoder pixel format into enc->frame
    const uint8_t* src_slices[1] = { rgba_in };
    int src_stride[1] = { static_cast<int>(encoder->width * 4) };

    sws_scale(encoder->sws_ctx,
        src_slices,
        src_stride,
        0,
        encoder->height,
        encoder->frame->data,
        encoder->frame->linesize);

    encoder->frame->pts = encoder->pts++;

    int ret = avcodec_send_frame(encoder->codec_ctx, encoder->frame);
    if (ret < 0) {
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    // drain produced packets and write them to muxer
    return rffmpeg_encoder_drain_packets(encoder);
}

extern "C" rffmpeg_status rffmpegEncoderFinish(rffmpeg_encoder* encoder) {
    if (!encoder) return RFFMPEG_STATUS_INVALID_ARGUMENT;

    // send NULL frame to flush encoder
    int ret = avcodec_send_frame(encoder->codec_ctx, nullptr);
    if (ret < 0) {
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    rffmpeg_status status = rffmpeg_encoder_drain_packets(encoder);
    if (status != RFFMPEG_STATUS_OK) return status;

    if (av_write_trailer(encoder->fmt_ctx) < 0) {
        return RFFMPEG_STATUS_ENCODE_FAILED;
    }

    return RFFMPEG_STATUS_OK;
}

extern "C" void rffmpegEncoderDestroy(rffmpeg_encoder* encoder) {
    if (!encoder) return;

    rffmpeg_free_encoder_internal(encoder);
    delete encoder;
}

// Forward declaration of helper function
static rffmpeg_status rffmpeg_init_decoder_common(rffmpeg_decoder* dec,
    uint32_t* out_width,
    uint32_t* out_height,
    uint32_t* out_frame_count,
    double* out_fps,
    double* out_duration_sec);

extern "C" rffmpeg_status rffmpegDecoderCreate(const uint8_t* data,
    size_t size,
    rffmpeg_decoder** out_decoder,
    uint32_t* out_width,
    uint32_t* out_height,
    uint32_t* out_frame_count,
    double* out_fps,
    double* out_duration_sec) {
    if (!data || size == 0 || !out_decoder || !out_width || !out_height || !out_frame_count ||
        !out_fps || !out_duration_sec) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    rffmpeg_decoder* dec = new (std::nothrow) rffmpeg_decoder();
    if (!dec) {
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Initialize all fields
    memset(dec, 0, sizeof(rffmpeg_decoder));
    dec->video_stream_index = -1;
    dec->data_ptr = data;
    dec->data_size = size;
    dec->data_pos = 0;

    // Allocate AVIO buffer
    constexpr size_t avio_buffer_size = 4096;
    dec->avio_buffer = static_cast<uint8_t*>(av_malloc(avio_buffer_size));
    if (!dec->avio_buffer) {
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Create custom AVIO context for reading from memory
    dec->avio_ctx = avio_alloc_context(
        dec->avio_buffer, avio_buffer_size, 0, dec, read_packet, nullptr, seek_packet);
    if (!dec->avio_ctx) {
        av_free(dec->avio_buffer);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Allocate format context
    dec->format_ctx = avformat_alloc_context();
    if (!dec->format_ctx) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    dec->format_ctx->pb = dec->avio_ctx;

    // Open input from custom AVIO context
    if (avformat_open_input(&dec->format_ctx, nullptr, nullptr, nullptr) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Retrieve stream information
    if (avformat_find_stream_info(dec->format_ctx, nullptr) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Find the first video stream
    for (unsigned int i = 0; i < dec->format_ctx->nb_streams; i++) {
        if (dec->format_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
            dec->video_stream_index = i;
            break;
        }
    }

    rffmpeg_status status = rffmpeg_init_decoder_common(
        dec, out_width, out_height, out_frame_count, out_fps, out_duration_sec);
    if (status != RFFMPEG_STATUS_OK) {
        return status;
    }

    *out_decoder = dec;

    return RFFMPEG_STATUS_OK;
}

// Helper function to initialize decoder after format/codec setup
static rffmpeg_status rffmpeg_init_decoder_common(rffmpeg_decoder* dec,
    uint32_t* out_width,
    uint32_t* out_height,
    uint32_t* out_frame_count,
    double* out_fps,
    double* out_duration_sec) {
    if (dec->video_stream_index == -1) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    AVStream* video_stream = dec->format_ctx->streams[dec->video_stream_index];
    AVCodecParameters* codecpar = video_stream->codecpar;

    // Find decoder for the video stream
    const AVCodec* codec = avcodec_find_decoder(codecpar->codec_id);
    if (!codec) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Allocate codec context
    dec->codec_ctx = avcodec_alloc_context3(codec);
    if (!dec->codec_ctx) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Copy codec parameters to context
    if (avcodec_parameters_to_context(dec->codec_ctx, codecpar) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Open codec
    if (avcodec_open2(dec->codec_ctx, codec, nullptr) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    dec->width = dec->codec_ctx->width;
    dec->height = dec->codec_ctx->height;

    // Calculate FPS
    AVRational frame_rate = video_stream->avg_frame_rate;
    if (frame_rate.num && frame_rate.den) {
        dec->fps = static_cast<double>(frame_rate.num) / static_cast<double>(frame_rate.den);
    } else {
        dec->fps = 25.0;  // Default fallback
    }

    // Calculate duration and frame count
    if (video_stream->duration != AV_NOPTS_VALUE) {
        dec->duration_sec =
            static_cast<double>(video_stream->duration) * av_q2d(video_stream->time_base);
    } else if (dec->format_ctx->duration != AV_NOPTS_VALUE) {
        dec->duration_sec = static_cast<double>(dec->format_ctx->duration) / AV_TIME_BASE;
    } else {
        dec->duration_sec = 0.0;
    }

    if (video_stream->nb_frames > 0) {
        dec->frame_count = video_stream->nb_frames;
    } else if (dec->duration_sec > 0.0 && dec->fps > 0.0) {
        dec->frame_count = static_cast<uint32_t>(dec->duration_sec * dec->fps);
    } else {
        dec->frame_count = 0;  // Unknown
    }

    // Allocate packet and frames
    dec->packet = av_packet_alloc();
    dec->frame = av_frame_alloc();
    dec->rgba_frame = av_frame_alloc();

    if (!dec->packet || !dec->frame || !dec->rgba_frame) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Allocate RGBA buffer using av_image_alloc which returns an aligned/padded
    // buffer and fills the frame data/linesize arrays. This is safer than
    // computing size + av_malloc + av_image_fill_arrays because the allocator
    // will provide the correct alignment and padding required by sws_scale.
    const int rgba_align = 32;  // give swscale room for vectorized writes
    int alloc_ret = av_image_alloc(dec->rgba_frame->data,
        dec->rgba_frame->linesize,
        dec->width,
        dec->height,
        AV_PIX_FMT_RGBA,
        rgba_align);
    if (alloc_ret < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // av_image_alloc returns the total buffer size and places the pointer to
    // the first plane in data[0]. Keep a reference so we can free it later.
    dec->rgba_buffer = dec->rgba_frame->data[0];

    // Create software scaler context for format conversion
    dec->sws_ctx = sws_getContext(dec->width,
        dec->height,
        dec->codec_ctx->pix_fmt,
        dec->width,
        dec->height,
        AV_PIX_FMT_RGBA,
        SWS_BILINEAR,
        nullptr,
        nullptr,
        nullptr);

    if (!dec->sws_ctx) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    *out_width = dec->width;
    *out_height = dec->height;
    *out_frame_count = dec->frame_count;
    *out_fps = dec->fps;
    *out_duration_sec = dec->duration_sec;

    return RFFMPEG_STATUS_OK;
}

extern "C" rffmpeg_status rffmpegDecoderCreateFromPath(const char* path,
    rffmpeg_decoder** out_decoder,
    uint32_t* out_width,
    uint32_t* out_height,
    uint32_t* out_frame_count,
    double* out_fps,
    double* out_duration_sec) {
    if (!path || !out_decoder || !out_width || !out_height || !out_frame_count || !out_fps ||
        !out_duration_sec) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    rffmpeg_decoder* dec = new (std::nothrow) rffmpeg_decoder();
    if (!dec) {
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Initialize all fields
    memset(dec, 0, sizeof(rffmpeg_decoder));
    dec->video_stream_index = -1;

    // Allocate format context
    dec->format_ctx = avformat_alloc_context();
    if (!dec->format_ctx) {
        delete dec;
        return RFFMPEG_STATUS_ALLOCATION_FAILED;
    }

    // Open input from file/URL path
    if (avformat_open_input(&dec->format_ctx, path, nullptr, nullptr) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Retrieve stream information
    if (avformat_find_stream_info(dec->format_ctx, nullptr) < 0) {
        rffmpeg_free_decoder_internal(dec);
        delete dec;
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Find the first video stream
    for (unsigned int i = 0; i < dec->format_ctx->nb_streams; i++) {
        if (dec->format_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
            dec->video_stream_index = i;
            break;
        }
    }

    rffmpeg_status status = rffmpeg_init_decoder_common(
        dec, out_width, out_height, out_frame_count, out_fps, out_duration_sec);
    if (status != RFFMPEG_STATUS_OK) {
        return status;
    }

    *out_decoder = dec;
    return RFFMPEG_STATUS_OK;
}

extern "C" rffmpeg_status rffmpegDecoderNext(
    rffmpeg_decoder* decoder, uint8_t* rgba_out, size_t rgba_capacity, uint32_t* out_timestamp_ms) {
    if (!decoder || !rgba_out || !out_timestamp_ms) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    size_t required_size = decoder->width * decoder->height * 4;
    if (rgba_capacity < required_size) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    // Read frames until we get a video frame
    while (true) {
        int ret = av_read_frame(decoder->format_ctx, decoder->packet);

        if (ret == AVERROR_EOF) {
            return RFFMPEG_STATUS_END_OF_STREAM;
        }

        if (ret < 0) {
            return RFFMPEG_STATUS_DECODE_FAILED;
        }

        // Skip non-video packets
        if (decoder->packet->stream_index != decoder->video_stream_index) {
            av_packet_unref(decoder->packet);
            continue;
        }

        // Send packet to decoder
        ret = avcodec_send_packet(decoder->codec_ctx, decoder->packet);
        av_packet_unref(decoder->packet);

        if (ret < 0) {
            return RFFMPEG_STATUS_DECODE_FAILED;
        }

        // Receive decoded frame
        ret = avcodec_receive_frame(decoder->codec_ctx, decoder->frame);

        if (ret == AVERROR(EAGAIN)) {
            continue;  // Need more packets
        }

        if (ret < 0) {
            return RFFMPEG_STATUS_DECODE_FAILED;
        }

        // Convert to RGBA
        sws_scale(decoder->sws_ctx,
            decoder->frame->data,
            decoder->frame->linesize,
            0,
            decoder->height,
            decoder->rgba_frame->data,
            decoder->rgba_frame->linesize);

        // Copy RGBA data to output buffer row-by-row using the frame linesize.
        // When the frame linesize (stride) is larger than width*4 (because of
        // alignment/padding), a single memcpy of the contiguous buffer will
        // interleave padding bytes and corrupt the resulting image. Copying
        // per-row ensures we copy exactly width*4 bytes per row into the
        // tightly-packed output buffer.
        {
            int src_linesize = decoder->rgba_frame->linesize[0];
            uint8_t* src = decoder->rgba_frame->data[0];
            const size_t dst_row_bytes = static_cast<size_t>(decoder->width) * 4;
            for (uint32_t y = 0; y < decoder->height; ++y) {
                memcpy(rgba_out + (size_t)y * dst_row_bytes,
                    src + (size_t)y * src_linesize,
                    dst_row_bytes);
            }
        }

        // Calculate timestamp in milliseconds
        AVStream* video_stream = decoder->format_ctx->streams[decoder->video_stream_index];
        int64_t pts = decoder->frame->pts;
        if (pts != AV_NOPTS_VALUE) {
            double timestamp_sec = static_cast<double>(pts) * av_q2d(video_stream->time_base);
            *out_timestamp_ms = static_cast<uint32_t>(timestamp_sec * 1000.0);
        } else {
            *out_timestamp_ms = 0;
        }

        av_frame_unref(decoder->frame);

        return RFFMPEG_STATUS_OK;
    }
}

extern "C" rffmpeg_status rffmpegDecoderReset(rffmpeg_decoder* decoder) {
    if (!decoder) {
        return RFFMPEG_STATUS_INVALID_ARGUMENT;
    }

    // Seek to beginning
    if (av_seek_frame(decoder->format_ctx, decoder->video_stream_index, 0, AVSEEK_FLAG_BACKWARD) <
        0) {
        return RFFMPEG_STATUS_DECODE_FAILED;
    }

    // Flush codec buffers
    avcodec_flush_buffers(decoder->codec_ctx);

    return RFFMPEG_STATUS_OK;
}

extern "C" void rffmpegDecoderDestroy(rffmpeg_decoder* decoder) {
    if (!decoder) {
        return;
    }

    rffmpeg_free_decoder_internal(decoder);
    delete decoder;
}
