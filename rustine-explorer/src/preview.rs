use rustine::{gfx, Mailbox, io, log};
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
    Load(PathBuf, Arc<gfx::Image>),
    Clear,
}

pub trait PreviewHandler {
    fn can_handle(&self, path: &Path) -> bool;
    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image_id: Arc<gfx::Image>,
        exit_flag: &AtomicBool,
    ) -> PreviewStatus;
}

pub struct PreviewFrame {
    pub image: io::Image,
    pub duration_ms: u32,
}

pub struct WebPPreviewHandler;

impl WebPPreviewHandler {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &["WEBP"];
}

impl PreviewHandler for WebPPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let upper = ext.to_uppercase();
                WebPPreviewHandler::SUPPORTED_EXTENSIONS.contains(&upper.as_str())
            })
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image: Arc<gfx::Image>,
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
                        target_image.id(),
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
            if exit_flag.load(Ordering::Relaxed) {
                return PreviewStatus::Ok;
            }

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
                        target_image.id(),
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
            }
        }
    }
}

struct FfmpegPreviewHandler;

impl FfmpegPreviewHandler {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] =
        &["MP4", "MKV", "WEBM", "WEBP", "MOV", "AVI", "FLV", "M4V"];
}

impl PreviewHandler for FfmpegPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let upper = ext.to_uppercase();
                FfmpegPreviewHandler::SUPPORTED_EXTENSIONS.contains(&upper.as_str())
            })
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        target_image: Arc<gfx::Image>,
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

        log::debug!("FFmpeg frame count: {}", decoder.frame_count());

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
                        target_image.id(),
                        io::Image {
                            width,
                            height,
                            format: rustine::gfx::Format::R8G8B8A8_UNORM,
                            pixels: frame,
                        },
                    ));

                    // If there is only one frame, no need to repeat
                    if decoder.frame_count() <= 1 {
                        return PreviewStatus::Ok;
                    }

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
                            Ok(_) => {
                                // If there is only one frame, no need to repeat
                                if decoder.frame_count() <= 1 {
                                    return PreviewStatus::Ok;
                                }
                                continue;
                            }
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
        .map(|ext| ext.to_uppercase());

    let mut can_handle = WebPPreviewHandler::SUPPORTED_EXTENSIONS
        .iter()
        .any(|&s| Some(s) == extension.as_deref());

    if !can_handle {
        can_handle = FfmpegPreviewHandler::SUPPORTED_EXTENSIONS
            .iter()
            .any(|&s| Some(s) == extension.as_deref());
    }

    if !can_handle {
        can_handle = GltfPreviewHandler::SUPPORTED_EXTENSIONS
            .iter()
            .any(|&s| Some(s) == extension.as_deref());
    }

    can_handle
}

pub fn preview_worker_thread(
    request_queue: Arc<Mailbox<PreviewRequest>>,
    image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
    exit_flag: &AtomicBool,
    request_flag: &AtomicBool,
) {
    rustine::log::Log::global().set_current_thread_name("preview-worker");

    // Handlers for different preview types (WebP first, then FFmpeg for video)
    let handlers: Vec<Box<dyn PreviewHandler + Send>> = vec![
        Box::new(WebPPreviewHandler),
        Box::new(FfmpegPreviewHandler),
        Box::new(GltfPreviewHandler),
    ];

    loop {
        if exit_flag.load(Ordering::Relaxed) {
            break;
        }

        request_queue.wait();

        match request_queue.pop_back_and_discard() {
            Some(PreviewRequest::Load(path, image_id)) => {
                if let Some(handler) = handlers.iter().find(|handler| handler.can_handle(&path)) {
                    request_flag.store(false, Ordering::Relaxed);
                    match handler.handle(&path, Arc::clone(&image_mailbox), image_id, request_flag)
                    {
                        PreviewStatus::Ok => {
                            log::debug!("Preview succeeded for {:?}", path)
                        }
                        PreviewStatus::EndOfStream => {
                            log::debug!("Preview reached end of stream for {:?}", path)
                        }
                        PreviewStatus::Error(err) => {
                            log::error!("Preview error for {:?}: {}", path, err);
                        }
                    };
                } else {
                    log::warning!("No handler found for previewing {:?}", path);
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

struct GltfPreviewHandler;

impl GltfPreviewHandler {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &["GLB"];
}

impl PreviewHandler for GltfPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let upper = ext.to_uppercase();
                GltfPreviewHandler::SUPPORTED_EXTENSIONS.contains(&upper.as_str())
            })
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        _image_mailbox: Arc<Mailbox<(u32, io::Image)>>,
        _target_image: Arc<gfx::Image>,
        _exit_flag: &AtomicBool,
    ) -> PreviewStatus {
        match rustine::io::gltf::deserialize(path.to_str().unwrap()) {
            Ok(gltf) => {
                let _ = rustine::io::gltf::parse(gltf);
                PreviewStatus::EndOfStream
            }
            Err(err) => PreviewStatus::Error(format!("Failed to deserialize glTF: {}", err)),
        }
    }
}
