use rustine::{debug, error, info};
use rustine::{gfx, gfx::*, gui, io, log::*, version::Version};

#[cfg(unix)]
use libc;
use std::collections::VecDeque;
use std::io as stdio;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic};
use std::thread;

static EXIT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn handle_termination_signal(_signal: i32) {
    info!("SIGTERM");
    EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
}

#[cfg(unix)]
fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_termination_signal as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            let os_error = stdio::Error::last_os_error();
            eprintln!("Failed to install SIGINT handler: {os_error:?}",);
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            let os_error = stdio::Error::last_os_error();
            eprintln!("Failed to install SIGTERM handler: {os_error:?}",);
        }
    }
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

fn main() {
    let platform = if cfg!(target_os = "linux") {
        gfx::Platform::Wayland
    } else {
        panic!("Unsupported platform");
    };

    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleListener::new(true));

    install_signal_handlers();

    info!("STARTUP");

    {
        let params = gfx::StartupParameters {
            enable_debugging: true,
            host_platform: platform,
            host_version: Version::new(0, 1, 0),
            host_name: "rustine-app".to_string(),
        };

        let gfx_core = match gfx::Core::builder(params).select_optimal_device().build() {
            Ok(core) => core,
            Err(e) => {
                error!("Failed to build gfx::Core: {}", e);
                return;
            }
        };

        let image_mailbox = gfx_core.image_mailbox();
        let render_mailbox = gfx_core.render_mailbox();

        let dev = gfx_core.selected_physical_device();
        info!("{dev}");

        let gfx = Arc::new(Mutex::new(gfx_core));

        let gui_params = gui::StartupParameters {
            platform,
            window_title: "Rustine".to_string(),
            window_width: Some(1920),
            window_height: Some(1080),
        };
        let gui = gui::Gui::new(Arc::clone(&gfx), gui_params);

        thread::scope(|s| {
            s.spawn(|| {
                gfx_thread_function(Arc::clone(&gfx), &EXIT_FLAG);
            });

            s.spawn(|| {
                image_loader_thread(image_mailbox, &EXIT_FLAG);
            });

            gui_thread_function(&gui, render_mailbox, &EXIT_FLAG);

            EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
        });
    }

    info!("SHUTDOWN");
}

fn gui_thread_function(gui: &gui::Gui, render_mailbox: Arc<Mutex<VecDeque<gfx::RenderFrame>>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gui-render");

    info!("GUI START");

    while !exit_flag.load(atomic::Ordering::Relaxed) && !gui.should_close() {
        gui.wait_events_timeout(16);

        // Generate a render frame with pre-computed vertices and draw commands
        let frame = generate_render_frame(gui.pixel_size());

        if let Ok(mut pending) = render_mailbox.lock() {
            pending.push_back(frame);
        }
    }

    info!("GUI STOP");
}

fn generate_render_frame(frame_size: Vector2u) -> gfx::RenderFrame {
    let mut frame = gfx::RenderFrame::new();

    // Build a simple fullscreen quad with the dynamic image (ID 0)
    // Vertices are in screen coordinates (0 to width/height)
    // Shader will transform to NDC space during vertex processing
    // Vertices: 4 corners (position: 2xf32, uv: 2xf32, color: u32)
    let mut vertices_bytes = Vec::new();
    let color = 0xFFFFFFFFu32;

    // Screen coordinates: fullscreen quad (0,0) to (1920, 1080)
    // Vertex 0: (0, 0), uv (0, 0), color white
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&color.to_le_bytes());

    // Vertex 1: (1920, 0), uv (1, 0)
    vertices_bytes.extend_from_slice(&(frame_size.x as f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(1.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&color.to_le_bytes());

    // Vertex 2: (1920, 1080), uv (1, 1)
    vertices_bytes.extend_from_slice(&(frame_size.x as f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(frame_size.y as f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(1.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(1.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&color.to_le_bytes());

    // Vertex 3: (0, 1080), uv (0, 1)
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(frame_size.y as f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(0.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&(1.0_f32).to_le_bytes());
    vertices_bytes.extend_from_slice(&color.to_le_bytes());

    // Indices: two triangles (0, 1, 2, 0, 2, 3)
    let mut indices_bytes = Vec::new();
    for idx in &[0u32, 1, 2, 0, 2, 3] {
        indices_bytes.extend_from_slice(&idx.to_le_bytes());
    }

    frame.vertices = vertices_bytes;
    frame.indices = indices_bytes;

    // Create a single batch with a draw command for the quad
    let mut batch = gfx::DrawBatch::new();
    batch.push_command(gfx::DrawCommand::new(0, 6, Some(0)));

    frame.push_batch(batch);

    frame
}

fn format_duration(seconds: f64) -> String {
    if seconds >= 3600.0 {
        format!("{:.0} h", seconds / 3600.0)
    } else if seconds >= 60.0 {
        format!("{:.0} m", seconds / 60.0)
    } else if seconds >= 1.0 {
        format!("{:.0} s", seconds)
    } else if seconds >= 1e-3 {
        format!("{:.0} ms", seconds * 1e3)
    } else if seconds >= 1e-6 {
        format!("{:.0} us", seconds * 1e6)
    } else {
        format!("{:.0} ns", seconds * 1e9)
    }
}

fn image_loader_thread(mailbox: Arc<Mutex<std::collections::VecDeque<io::Image>>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("image-loader");

    let pictures_root = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("pictures"))
        .unwrap_or_else(|_| PathBuf::from("/home/jant/pictures"));

    let images = collect_webp_images(&pictures_root);
    if images.is_empty() {
        info!("Image loader: no .webp files under {:?}", pictures_root);
        return;
    }

    const SLIDE_DELAY: std::time::Duration = std::time::Duration::from_millis(1500);

    let mut index = 0usize;
    while !exit_flag.load(atomic::Ordering::Relaxed) {
        let path = &images[index % images.len()];
        if let Some(image) = load_webp_image(path) {
            if let Ok(mut pending) = mailbox.lock() {
                pending.push_back(image);
            }
        }

        index = index.wrapping_add(1);

        let mut slept = std::time::Duration::ZERO;
        while slept < SLIDE_DELAY && !exit_flag.load(atomic::Ordering::Relaxed) {
            let step = std::time::Duration::from_millis(50);
            std::thread::sleep(step);
            slept += step;
        }
    }
}

fn collect_webp_images(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    fn recurse(acc: &mut Vec<PathBuf>, path: &Path) {
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                recurse(acc, &p);
            } else if let Some(ext) = p.extension() {
                if ext.eq_ignore_ascii_case("webp") {
                    acc.push(p);
                }
            }
        }
    }
    recurse(&mut files, root);
    files.sort();
    files
}

fn load_webp_image(path: &Path) -> Option<io::Image> {
    let data = std::fs::read(path).ok()?;
    let mut decoder = rustine::io::webp::WebPDecoder::new(&data).ok()?;
    let width = decoder.width();
    let height = decoder.height();
    let mut frame = vec![0u8; (width * height * 4) as usize];
    decoder.next_frame(&mut frame).ok()?;

    Some(io::Image {
        width,
        height,
        format: gfx::Format::R8G8B8A8_UNORM,
        data: frame,
    })
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("GFX START");

    const TARGET_FPS: f64 = 120.0;
    let target_frame_time = std::time::Duration::from_secs_f64(1.0 / TARGET_FPS);
    const SPIN_THRESHOLD: std::time::Duration = std::time::Duration::from_micros(500);

    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();
    let mut last_stat_instant = std::time::Instant::now();
    let mut frame_delta_times: rustine::RingBuffer<f64> = rustine::RingBuffer::new(120);

    while !exit_flag.load(atomic::Ordering::Relaxed) {
        let frame_start = std::time::Instant::now();

        {
            let mut core = gfx.lock().unwrap();

            let now = std::time::Instant::now();
            let t = now.duration_since(start_instant).as_secs_f64();
            let dt = now.duration_since(last_instant).as_secs_f32();
            frame_delta_times.push(dt as f64);
            core.render(t, dt);
            last_instant = now;

            let stat_now = std::time::Instant::now();
            if stat_now.duration_since(last_stat_instant).as_secs_f64() >= 1.0 {
                let frame_cpu_times = core.frame_cpu_times();
                if let Some((min, max, mean)) = frame_cpu_times.min_max_mean() {
                    debug!(
                        "Frame CPU -> min: {}, max: {}, mean: {}",
                        format_duration(min),
                        format_duration(max),
                        format_duration(mean)
                    );
                }
                if let Some((min, max, mean)) = frame_delta_times.min_max_mean() {
                    debug!(
                        "Frame Delta -> min: {}, max: {}, mean: {}",
                        format_duration(min as f64),
                        format_duration(max as f64),
                        format_duration(mean as f64)
                    );
                }
                last_stat_instant = stat_now;
            }
        }

        // Adaptive spin-sleep frame rate limiting
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_time {
            let mut remaining = target_frame_time - elapsed;

            // Sleep for bulk of remaining time
            while remaining > SPIN_THRESHOLD {
                std::thread::sleep(std::time::Duration::from_millis(1));
                remaining = target_frame_time.saturating_sub(frame_start.elapsed());
            }

            // Spin for precise timing
            while frame_start.elapsed() < target_frame_time {
                std::hint::spin_loop();
            }
        }
    }

    info!("GFX STOP");
}
