

use std::env;
use std::fs::read;
use std::error::Error;

use rustine::io::webp::{WebPDecoder, WebPResult};
use rustine::io::ffmpeg::VideoEncoder;

fn main() -> Result<(), Box<dyn Error>> {
	let mut args = env::args();
	let _prog = args.next();

	let input = match args.next() {
		Some(p) => p,
		None => {
			eprintln!("Usage: webp_to_mp4 <input.webp> [output.mp4]");
			std::process::exit(2);
		}
	};

	let output = match args.next() {
		Some(p) => p,
		None => {
			let mut out = input.clone();
			if out.ends_with(".webp") {
				out.truncate(out.len() - 5);
			}
			out.push_str(".mp4");
			out
		}
	};

	let data = read(&input)?;
	let mut decoder = WebPDecoder::new(&data).map_err(|e| format!("Failed to open WebP: {}", e))?;

	let width = decoder.width();
	let height = decoder.height();

	let frame_size = (width as usize) * (height as usize) * 4;
	let mut buf1 = vec![0u8; frame_size];
	let mut buf2 = vec![0u8; frame_size];

	// Read first frame
	let t0 = match decoder.next_frame(&mut buf1) {
		WebPResult::Ok(ts) => ts,
		WebPResult::EndOfStream => {
			return Err("WebP contains no frames".into());
		}
		WebPResult::Error(e) => return Err(format!("Failed to decode first frame: {}", e).into()),
	};

	// Try to read second frame to determine frame interval
	let mut fps = 30.0_f64;
	let mut have_buf2 = false;
	if let WebPResult::Ok(t1) = decoder.next_frame(&mut buf2) {
		have_buf2 = true;
		if t1 > t0 {
			let delta = (t1 as f64) - (t0 as f64);
			if delta > 0.0 {
				fps = 1000.0 / delta;
			}
		}
	}

	// Create encoder
	let bitrate = 2_000_000u64; // 2 Mbps
	let mut encoder = VideoEncoder::create_to_path(&output, width, height, fps.max(1.0), bitrate)
		.map_err(|e| format!("Failed to create encoder: {}", e))?;

	println!("Encoding {}x{} @ {:.2} fps -> {}", width, height, fps, output);

	// Encode the first frame
	encoder.encode_frame(&buf1).map_err(|e| format!("Encode failed: {}", e))?;

	// If we already have the second frame, encode it, otherwise loop and read more
	if have_buf2 {
		encoder.encode_frame(&buf2).map_err(|e| format!("Encode failed: {}", e))?;
	}

	// Continue encoding remaining frames
	loop {
		let mut frame = vec![0u8; frame_size];
		match decoder.next_frame(&mut frame) {
			WebPResult::Ok(_ts) => {
				encoder.encode_frame(&frame).map_err(|e| format!("Encode failed: {}", e))?;
			}
			WebPResult::EndOfStream => break,
			WebPResult::Error(e) => return Err(format!("Failed to decode frame: {}", e).into()),
		}
	}

	encoder.finish().map_err(|e| format!("Failed to finish encoder: {}", e))?;

	println!("Finished encoding to {}", output);

	Ok(())
}
