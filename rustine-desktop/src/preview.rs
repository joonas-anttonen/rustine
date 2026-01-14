use rustine::{ConcurrentMailbox, io, log};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
        image_mailbox: Arc<Mutex<VecDeque<(u32, io::Image)>>>,
        target_image_id: u32,
        exit_flag: &AtomicBool,
    ) -> PreviewStatus;
}

pub struct PreviewFrame {
    pub image: io::Image,
    pub duration_ms: u32,
}

pub struct WebPPreviewHandler;

impl PreviewHandler for WebPPreviewHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("webp"))
            .unwrap_or(false)
    }

    fn handle(
        &self,
        path: &Path,
        image_mailbox: Arc<Mutex<VecDeque<(u32, io::Image)>>>,
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
                    if let Ok(mut pending) = image_mailbox.lock() {
                        pending.push_back((
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
                    if let Ok(mut pending) = image_mailbox.lock() {
                        pending.push_back((
                            target_image_id,
                            io::Image {
                                width,
                                height,
                                format: rustine::gfx::Format::R8G8B8A8_UNORM,
                                pixels: frame,
                            },
                        ));
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

                    if exit_flag.load(Ordering::Relaxed) {
                        return PreviewStatus::Ok;
                    }
                }
            }
        }
    }
}

pub fn preview_worker_thread(
    request_queue: Arc<ConcurrentMailbox<PreviewRequest>>,
    image_mailbox: Arc<Mutex<VecDeque<(u32, io::Image)>>>,
    target_image_id: u32,
    exit_flag: &AtomicBool,
    request_flag: &AtomicBool,
) {
    rustine::log::Log::global().set_current_thread_name("preview-worker");

    let handlers: Vec<Box<dyn PreviewHandler + Send>> = vec![Box::new(WebPPreviewHandler)];

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
                        image_mailbox.clone(),
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
