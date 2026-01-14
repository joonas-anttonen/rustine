use rustine::io;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

pub enum PreviewRequest {
	Load(PathBuf),
	Clear,
}

pub trait PreviewHandler {
	fn can_handle(&self, path: &Path) -> bool;
	fn load(&self, path: &Path) -> Option<Vec<PreviewFrame>>;
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

	fn load(&self, path: &Path) -> Option<Vec<PreviewFrame>> {
		let data = std::fs::read(path).ok()?;
		let mut decoder = rustine::io::webp::WebPDecoder::new(&data).ok()?;

		let width = decoder.width();
		let height = decoder.height();
		let frame_count = decoder.frame_count();

		if frame_count == 0 {
			return None;
		}

		let mut frames = Vec::new();
		let mut prev_timestamp = 0u32;

		for _ in 0..frame_count {
			let mut pixels = vec![0u8; (width * height * 4) as usize];
			match decoder.next_frame(&mut pixels) {
				Ok(timestamp_ms) => {
					let duration = timestamp_ms.saturating_sub(prev_timestamp);
					frames.push(PreviewFrame {
						image: io::Image {
							width,
							height,
							format: rustine::gfx::Format::R8G8B8A8_UNORM,
							pixels,
						},
						duration_ms: duration,
					});
					prev_timestamp = timestamp_ms;
				}
				Err(_) => break,
			}
		}

		if frames.is_empty() {
			None
		} else {
			Some(frames)
		}
	}
}

pub fn preview_worker_thread(
	request_queue: Arc<Mutex<VecDeque<PreviewRequest>>>,
	image_mailbox: Arc<Mutex<VecDeque<(u32, io::Image)>>>,
	target_image_id: u32,
	exit_flag: &AtomicBool,
) {
	rustine::log::Log::global().set_current_thread_name("preview-worker");

	let handlers: Vec<Box<dyn PreviewHandler + Send>> = vec![
		Box::new(WebPPreviewHandler),
	];

	while !exit_flag.load(Ordering::Relaxed) {
		let request = {
			let mut queue = request_queue.lock().unwrap();
			queue.pop_front()
		};

		match request {
			Some(PreviewRequest::Load(path)) => {
				let mut handled = false;
				for handler in &handlers {
					if handler.can_handle(&path) {
						if let Some(frames) = handler.load(&path) {
							if frames.is_empty() {
								break;
							}

							if frames.len() == 1 {
								// Static image - send once
								if let Ok(mut mailbox) = image_mailbox.lock() {
									mailbox.push_back((target_image_id, frames[0].image.clone()));
								}
							} else {
								// Animated image - loop through frames
								loop {
									let mut should_continue = true;
									for frame in &frames {
										if exit_flag.load(Ordering::Relaxed) {
											should_continue = false;
											break;
										}

										// Check if we have a new request
										{
											let queue = request_queue.lock().unwrap();
											if !queue.is_empty() {
												should_continue = false;
												break;
											}
										}

										if let Ok(mut mailbox) = image_mailbox.lock() {
											mailbox.push_back((target_image_id, frame.image.clone()));
										}

										if frame.duration_ms > 0 {
											let duration = std::time::Duration::from_millis(frame.duration_ms as u64);
											let mut remaining = duration;
											while remaining > std::time::Duration::ZERO && !exit_flag.load(Ordering::Relaxed) {
												let step = std::time::Duration::from_millis(50).min(remaining);
												std::thread::sleep(step);
												remaining = remaining.saturating_sub(step);

												// Check for new requests
												let queue = request_queue.lock().unwrap();
												if !queue.is_empty() {
													should_continue = false;
													break;
												}
											}
										}

										if !should_continue {
											break;
										}
									}

									if !should_continue {
										break;
									}
								}
							}
							handled = true;
						}
						break;
					}
				}

				if !handled {
					rustine::log::debug!("No handler for preview: {}", path.display());
				}
			}
			Some(PreviewRequest::Clear) => {
				// Optionally send a clear/fallback image
			}
			None => {
				// No requests, sleep briefly
				std::thread::sleep(std::time::Duration::from_millis(100));
			}
		}
	}

	rustine::log::info!("Preview worker thread exiting");
}
