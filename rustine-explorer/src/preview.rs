use rustine::{Mailbox, io, log};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
};

pub enum PreviewStatus {
    Ok,
    EndOfStream,
    Error(String),
}

pub enum PreviewRequest {
    Load(PathBuf),
    Clear,
}

pub trait PreviewHandler {
    fn can_handle(&self, path: &Path) -> bool;
    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image_id: u32,
        exit_flag: &AtomicBool,
    ) -> PreviewStatus;
}

pub struct PreviewFrame {
    pub image: io::Image,
    pub duration_ms: u32,
}

pub struct WebPPreviewHandler;

impl WebPPreviewHandler {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &["webp"];
}

impl PreviewHandler for WebPPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let lower = ext.to_ascii_lowercase();
                WebPPreviewHandler::SUPPORTED_EXTENSIONS.contains(&lower.as_str())
            })
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image_id: u32,
        exit_flag: &AtomicBool,
    ) -> PreviewStatus {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(err) => {
                return PreviewStatus::Error(format!("Failed to read file {:?}: {}", path, err));
            }
        };

        let mut decoder = match rustine::io::webp::WebPDecoder::new(&data) {
            Ok(d) => d,
            Err(err) => {
                return PreviewStatus::Error(format!(
                    "Failed to create WebPDecoder for {:?}: {}",
                    path, err
                ));
            }
        };

        let width = decoder.width();
        let height = decoder.height();
        let frame_count = decoder.frame_count();

        if frame_count == 0 {
            return PreviewStatus::Error(format!("WebP image has zero frames: {:?}", path));
        }

        // If single frame, load it normally
        if frame_count == 1 {
            let mut frame = vec![0u8; (width * height * 4) as usize];
            match decoder.next_frame(&mut frame) {
                io::webp::WebPResult::EndOfStream => return PreviewStatus::EndOfStream, // End of frames
                io::webp::WebPResult::Error(err) => {
                    log::error!("WebP decoding error: {}", err);
                    return PreviewStatus::Error(format!("WebP decoding error: {}", err));
                }
                io::webp::WebPResult::Ok(_) => {
                    image_mailbox.push((
                        target_image_id,
                        io::Image {
                            width,
                            height,
                            format: rustine::gfx::Format::R8G8B8A8_UNORM,
                            pixels: frame,
                        },
                    ));
                }
            }
            return PreviewStatus::Ok;
        }

        // Multi-frame animation: present all frames with their timings
        let mut prev_timestamp = 0u32;
        loop {
            let mut frame = vec![0u8; (width * height * 4) as usize];
            match decoder.next_frame(&mut frame) {
                io::webp::WebPResult::EndOfStream => {
                    // Preview loops forever
                    match decoder.reset() {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("WebP decoding error on reset: {}", err);
                            return PreviewStatus::Error(format!(
                                "WebP decoding error on reset: {}",
                                err
                            ));
                        }
                    }
                }
                io::webp::WebPResult::Error(err) => {
                    log::error!("WebP decoding error: {}", err);
                    return PreviewStatus::Error(format!("WebP decoding error: {}", err));
                }
                io::webp::WebPResult::Ok(timestamp_ms) => {
                    image_mailbox.push((
                        target_image_id,
                        io::Image {
                            width,
                            height,
                            format: rustine::gfx::Format::R8G8B8A8_UNORM,
                            pixels: frame,
                        },
                    ));

                    // Calculate frame duration and sleep
                    let frame_duration = timestamp_ms.saturating_sub(prev_timestamp);
                    if frame_duration > 0 {
                        let duration = std::time::Duration::from_millis(frame_duration as u64);
                        let mut remaining = duration;
                        while remaining > std::time::Duration::from_millis(0)
                            && !exit_flag.load(Ordering::Relaxed)
                        {
                            let step = std::time::Duration::from_millis(50).min(remaining);
                            std::thread::sleep(step);
                            remaining = remaining.saturating_sub(step);
                        }
                    }

                    prev_timestamp = timestamp_ms;

                    if exit_flag.load(Ordering::Relaxed) {
                        return PreviewStatus::Ok;
                    }
                }
            }
        }
    }
}

struct FfmpegPreviewHandler;

impl FfmpegPreviewHandler {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] =
        &["mp4", "mkv", "webm", "mov", "avi", "flv", "m4v"];
}

impl PreviewHandler for FfmpegPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let lower = ext.to_ascii_lowercase();
                FfmpegPreviewHandler::SUPPORTED_EXTENSIONS.contains(&lower.as_str())
            })
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image_id: u32,
        exit_flag: &AtomicBool,
    ) -> PreviewStatus {
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return PreviewStatus::Error(format!("Invalid path: {:?}", path)),
        };

        let mut decoder = match io::ffmpeg::VideoDecoder::from_path(path_str) {
            Ok(d) => d,
            Err(err) => {
                return PreviewStatus::Error(format!(
                    "Failed to create FFmpeg VideoDecoder for {:?}: {}",
                    path, err
                ));
            }
        };

        let width = decoder.width();
        let height = decoder.height();

        let mut prev_timestamp = 0u32;

        loop {
            if exit_flag.load(Ordering::Relaxed) {
                return PreviewStatus::Ok;
            }

            let mut frame = vec![0u8; (width * height * 4) as usize];
            match decoder.next_frame(&mut frame) {
                Ok(timestamp_ms) => {
                    image_mailbox.push((
                        target_image_id,
                        io::Image {
                            width,
                            height,
                            format: rustine::gfx::Format::R8G8B8A8_UNORM,
                            pixels: frame,
                        },
                    ));

                    // Calculate frame duration and sleep
                    let frame_duration = timestamp_ms.saturating_sub(prev_timestamp);
                    if frame_duration > 0 {
                        let duration = std::time::Duration::from_millis(frame_duration as u64);
                        let mut remaining = duration;
                        while remaining > std::time::Duration::from_millis(0)
                            && !exit_flag.load(Ordering::Relaxed)
                        {
                            let step = std::time::Duration::from_millis(50).min(remaining);
                            std::thread::sleep(step);
                            remaining = remaining.saturating_sub(step);
                        }
                    }

                    prev_timestamp = timestamp_ms;
                }
                Err(err) => {
                    // Treat end-of-stream by resetting to loop the preview; other errors are fatal
                    if err.to_lowercase().contains("end of stream") {
                        match decoder.reset() {
                            Ok(_) => continue,
                            Err(e) => {
                                log::error!("FFmpeg reset error: {}", e);
                                return PreviewStatus::Error(format!(
                                    "FFmpeg decoding error on reset: {}",
                                    e
                                ));
                            }
                        }
                    } else {
                        log::error!("FFmpeg decoding error: {}", err);
                        return PreviewStatus::Error(format!("FFmpeg decoding error: {}", err));
                    }
                }
            }
        }
    }
}

pub fn can_preview(path: &std::path::Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    let mut can_handle = WebPPreviewHandler::SUPPORTED_EXTENSIONS
        .iter()
        .any(|&s| Some(s) == extension.as_deref());

    if !can_handle {
        can_handle = FfmpegPreviewHandler::SUPPORTED_EXTENSIONS
            .iter()
            .any(|&s| Some(s) == extension.as_deref());
    }

    can_handle
}

pub fn preview_worker_thread(
    request_queue: Arc<Mailbox<PreviewRequest>>,
    image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
    target_image_id: u32,
    exit_flag: &AtomicBool,
    request_flag: &AtomicBool,
) {
    rustine::log::Log::global().set_current_thread_name("preview-worker");

    // Handlers for different preview types (WebP first, then FFmpeg for video)
    let handlers: Vec<Box<dyn PreviewHandler + Send>> =
        vec![Box::new(WebPPreviewHandler), Box::new(FfmpegPreviewHandler)];

    loop {
        if exit_flag.load(Ordering::Relaxed) {
            break;
        }
        match request_queue.pop() {
            Some(PreviewRequest::Load(_path)) => {
                if let Some(handler) = handlers.iter().find(|handler| handler.can_handle(&_path)) {
                    request_flag.store(false, Ordering::Relaxed);
                    match handler.handle(
                        &_path,
                        Arc::clone(&image_mailbox),
                        target_image_id,
                        request_flag,
                    ) {
                        PreviewStatus::Ok => {
                            log::debug!("Preview succeeded for {:?}", _path)
                        }
                        PreviewStatus::EndOfStream => {
                            log::debug!("Preview reached end of stream for {:?}", _path)
                        }
                        PreviewStatus::Error(err) => {
                            log::error!("Preview error for {:?}: {}", _path, err);
                        }
                    };
                } else {
                    log::warning!("No handler found for previewing {:?}", _path);
                }
            }
            Some(PreviewRequest::Clear) => {}
            None => {
                // No requests, sleep briefly
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    rustine::log::info!("Preview worker thread exiting");
}
