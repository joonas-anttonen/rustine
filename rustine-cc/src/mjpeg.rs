use std::io::Write;
use std::net::{SocketAddrV4, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use rustine::{gfx::Format, io::Image, log};
use rustine::io::ffmpeg::JpegEncoder;

struct FrameState {
    seq: u64,
    image: Option<Image>,
}

pub struct FrameCache {
    state: Mutex<FrameState>,
    ready: Condvar,
}

impl FrameCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(FrameState { seq: 0, image: None }),
            ready: Condvar::new(),
        })
    }

    pub fn update(&self, image: Image) {
        if let Ok(mut state) = self.state.lock() {
            state.seq = state.seq.wrapping_add(1);
            state.image = Some(image);
            self.ready.notify_all();
        }
    }

    pub fn wait_next(&self, last_seq: u64) -> (u64, Image) {
        let mut state = self.state.lock().unwrap();
        while state.image.is_none() || state.seq == last_seq {
            state = self.ready.wait(state).unwrap();
        }
        let seq = state.seq;
        let image = state.image.as_ref().unwrap().clone();
        (seq, image)
    }
}

pub fn start_server(frame_cache: Arc<FrameCache>, bind_addr: SocketAddrV4) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(bind_addr) {
            Ok(listener) => listener,
            Err(e) => {
                log::error!("MJPEG bind failed: {}", e);
                return;
            }
        };

        log::info!("MJPEG server listening on {}", bind_addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let cache = Arc::clone(&frame_cache);
                    std::thread::spawn(move || handle_client(stream, cache));
                }
                Err(e) => log::error!("MJPEG accept failed: {}", e),
            }
        }
    })
}

fn handle_client(mut stream: TcpStream, frame_cache: Arc<FrameCache>) {
    let _ = stream.set_nodelay(true);

    let header = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Connection: close\r\n",
        "Cache-Control: no-store, no-cache, must-revalidate, max-age=0\r\n",
        "Pragma: no-cache\r\n",
        "Expires: 0\r\n",
        "Content-Type: multipart/x-mixed-replace; boundary=frame\r\n",
        "\r\n"
    );

    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let mut last_seq = 0u64;
    let mut jpeg_encoder: Option<JpegEncoder> = None;
    let mut jpeg_buf: Vec<u8> = Vec::new();
    let min_interval = Duration::from_millis(33);
    let mut next_send = Instant::now();

    loop {
        let (seq, image) = frame_cache.wait_next(last_seq);
        last_seq = seq;

        let now = Instant::now();
        if now < next_send {
            continue;
        }
        next_send = now + min_interval;

        if image.format != Format::R8G8B8A8_UNORM {
            continue;
        }

        if jpeg_encoder.is_none() {
            match JpegEncoder::new(image.width, image.height, 80) {
                Ok(encoder) => jpeg_encoder = Some(encoder),
                Err(e) => {
                    log::error!("JPEG encoder init failed: {}", e);
                    continue;
                }
            }
        }

        let encoder = jpeg_encoder.as_mut().unwrap();
        let jpeg = match encoder.encode_rgba(&image.pixels) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!("JPEG encode failed: {}", e);
                continue;
            }
        };

        jpeg_buf.clear();
        jpeg_buf.extend_from_slice(&jpeg);

        let boundary = format!(
            "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
            jpeg_buf.len()
        );

        if stream.write_all(boundary.as_bytes()).is_err() {
            break;
        }
        if stream.write_all(&jpeg_buf).is_err() {
            break;
        }
        if stream.write_all(b"\r\n").is_err() {
            break;
        }
        let _ = stream.flush();
    }
}

