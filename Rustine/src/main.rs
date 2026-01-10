use rustine::*;
use rustine::{debug, error, info};
use rustine::{gfx, gui, io, log::*, version::Version};

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

        let mut gfx_core = match gfx::Core::builder(params).select_optimal_device().build() {
            Ok(core) => core,
            Err(e) => {
                error!("Failed to build gfx::Core: {}", e);
                return;
            }
        };

        // Create a dynamic image handle used for rendering
        let display_image_dynamic = gfx_core.create_dynamic_image();

        // Load a test image on the main thread to exercise staging
        let display_image_static = if let Some(initial_image) =
            load_webp_image(Path::new("/home/jant/pictures/hmm/0yzhnmy0.webp"))
        {
            gfx_core.create_image(initial_image)
        } else {
            gfx::Image::default()
        };

        // Create a fallback image handle used for rendering
        let fallback_image_pixels = vec![0u8; 4]; // Transparent black pixel
        let display_image_fallback = gfx_core.create_image(io::Image {
            width: 1,
            height: 1,
            format: gfx::Format::R8G8B8A8_UNORM,
            pixels: fallback_image_pixels,
        });

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

            let image_mailbox_cloned = Arc::clone(&image_mailbox);
            let display_image_id = display_image_dynamic.id;
            s.spawn(move || {
                image_loader_thread(image_mailbox_cloned, display_image_id, &EXIT_FLAG);
            });

            gui_thread_function(
                &gui,
                render_mailbox,
                display_image_static,
                display_image_dynamic,
                display_image_fallback,
                &EXIT_FLAG,
            );

            EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
        });
    }

    info!("SHUTDOWN");
}

fn gui_thread_function(
    gui: &gui::Gui,
    render_mailbox: Arc<Mutex<VecDeque<gfx::RenderFrame>>>,
    static_image: gfx::Image,
    dynamic_image: gfx::Image,
    fallback_image: gfx::Image,
    exit_flag: &atomic::AtomicBool,
) {
    Log::global().set_current_thread_name("gui-render");

    info!("GUI START");

    while !exit_flag.load(atomic::Ordering::Relaxed) && !gui.should_close() {
        gui.wait_events_timeout(16);

        // Generate a render frame with pre-computed vertices and draw commands
        let frame = generate_render_frame(
            gui.pixel_size(),
            &static_image,
            &dynamic_image,
            &fallback_image,
        );

        if let Ok(mut pending) = render_mailbox.lock() {
            pending.push_back(frame);
        }
    }

    info!("GUI STOP");
}

fn generate_render_frame(
    frame_size: Vector2u,
    static_image: &gfx::Image,
    dynamic_image: &gfx::Image,
    fallback_image: &gfx::Image,
) -> gfx::RenderFrame {
    let mut frame = gfx::RenderFrame::new();

    let w = frame_size.x as f32;
    let h = frame_size.y as f32;

    const TOP_BAR_HEIGHT: f32 = 48.0;
    const SIDE_BAR_WIDTH: f32 = 48.0;
    const ADDRESS_BAR_HEIGHT: f32 = 32.0;
    const ADDRESS_BAR_MARGIN: f32 = 8.0;

    let content_x = SIDE_BAR_WIDTH;
    let content_y = TOP_BAR_HEIGHT;
    let content_w = w - SIDE_BAR_WIDTH;
    let content_h = h - TOP_BAR_HEIGHT;
    let half_content_w = content_w / 2.0;

    let color = 0xFFFFFF_FFu32;
    let bar_color = 0x1B232F_FFu32;
    let address_bar_color = 0xFFFFFF7Fu32;

    // Draw top bar (48 pixels tall)
    frame.fill_rectangle(
        &gfx::Rectangle {
            x: 0.0,
            y: 0.0,
            w,
            h: TOP_BAR_HEIGHT,
        },
        bar_color,
    );

    // Draw side bar (48 pixels wide)
    frame.fill_rectangle(
        &gfx::Rectangle {
            x: 0.0,
            y: TOP_BAR_HEIGHT,
            w: SIDE_BAR_WIDTH,
            h: content_h,
        },
        bar_color,
    );

    // Draw address bar (32 pixels tall, centered on top bar with margin)
    let address_bar_x = content_x + ADDRESS_BAR_MARGIN;
    let address_bar_y = (TOP_BAR_HEIGHT - ADDRESS_BAR_HEIGHT) / 2.0;
    let address_bar_w = content_w - ADDRESS_BAR_MARGIN * 2.0;

    frame.draw_rectangle(
        &gfx::Rectangle {
            x: address_bar_x,
            y: address_bar_y,
            w: address_bar_w,
            h: ADDRESS_BAR_HEIGHT,
        },
        2.0,
        address_bar_color,
    );

    // Draw images in content area (avoiding the edge bars)
    frame.push_image(
        static_image,
        None,
        gfx::Rectangle {
            x: content_x,
            y: content_y,
            w: half_content_w,
            h: content_h,
        },
        gfx::Fit::FIT_KEEP_ASPECT,
        color,
    );

    frame.push_image(
        dynamic_image,
        Some(fallback_image),
        gfx::Rectangle {
            x: content_x + half_content_w,
            y: content_y,
            w: half_content_w,
            h: content_h,
        },
        gfx::Fit::FIT_KEEP_ASPECT,
        color,
    );

    frame.push_text(
        "J{oo}nas [A]nttonen -> (@_åäö)\nAnother row !!! | ??? /\\ ^ ~* '",
        address_bar_x,
        address_bar_y + gfx::fonts::get_font_size(gfx::fonts::DEPARTUREMONO_FONT_ID),
        1.0,
        color,
        gfx::fonts::DEPARTUREMONO_FONT_ID,
    );

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

fn format_bytes_iec(bytes: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{:.0} {}", value, UNITS[idx])
    } else if value < 10.0 {
        format!("{:.1} {}", value, UNITS[idx])
    } else {
        format!("{:.0} {}", value, UNITS[idx])
    }
}

fn image_loader_thread(
    mailbox: Arc<Mutex<std::collections::VecDeque<(u32, io::Image)>>>,
    target_image_id: u32,
    exit_flag: &atomic::AtomicBool,
) {
    Log::global().set_current_thread_name("image-loader");

    let pictures_root = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("pictures").join("hmm"))
        .unwrap_or_else(|_| PathBuf::from("/home/jant/pictures/nsfw"));

    let images = collect_webp_images(&pictures_root);
    if images.is_empty() {
        info!("Image loader: no .webp files under {:?}", pictures_root);
        return;
    }

    const SLIDE_DELAY: std::time::Duration = std::time::Duration::from_millis(1500);

    let mut index = 0usize;
    while !exit_flag.load(atomic::Ordering::Relaxed) {
        let path = &images[index % images.len()];
        load_and_display_webp(path, target_image_id, &mailbox, &EXIT_FLAG);

        index = index.wrapping_add(1);

        let mut slept = std::time::Duration::ZERO;
        while slept < SLIDE_DELAY && !exit_flag.load(atomic::Ordering::Relaxed) {
            let step = std::time::Duration::from_millis(50);
            std::thread::sleep(step);
            slept += step;
        }
    }
}

fn load_and_display_webp(
    path: &Path,
    target_image_id: u32,
    mailbox: &Arc<Mutex<std::collections::VecDeque<(u32, io::Image)>>>,
    exit_flag: &atomic::AtomicBool,
) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut decoder = match rustine::io::webp::WebPDecoder::new(&data) {
        Ok(d) => d,
        Err(_) => return,
    };

    let width = decoder.width();
    let height = decoder.height();
    let frame_count = decoder.frame_count();

    if frame_count == 0 {
        return;
    }

    // If single frame, load it normally
    if frame_count == 1 {
        let mut frame = vec![0u8; (width * height * 4) as usize];
        if decoder.next_frame(&mut frame).is_ok() {
            if let Ok(mut pending) = mailbox.lock() {
                pending.push_back((
                    target_image_id,
                    io::Image {
                        width,
                        height,
                        format: gfx::Format::R8G8B8A8_UNORM,
                        pixels: frame,
                    },
                ));
            }
        }
        return;
    }

    // Multi-frame animation: present all frames with their timings
    let mut prev_timestamp = 0u32;
    loop {
        let mut frame = vec![0u8; (width * height * 4) as usize];
        match decoder.next_frame(&mut frame) {
            Ok(timestamp_ms) => {
                if let Ok(mut pending) = mailbox.lock() {
                    pending.push_back((
                        target_image_id,
                        io::Image {
                            width,
                            height,
                            format: gfx::Format::R8G8B8A8_UNORM,
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
                        && !exit_flag.load(atomic::Ordering::Relaxed)
                    {
                        let step = std::time::Duration::from_millis(50).min(remaining);
                        std::thread::sleep(step);
                        remaining = remaining.saturating_sub(step);
                    }
                }

                prev_timestamp = timestamp_ms;

                if exit_flag.load(atomic::Ordering::Relaxed) {
                    return;
                }
            }
            Err(_) => break, // End of frames
        }
    }
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
        pixels: frame,
    })
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
                let (allocs, deallocs) = rustine::alloc::counts();
                let current = rustine::alloc::current_bytes();
                let peak = rustine::alloc::peak_bytes();
                let rss = rustine::alloc::rss_bytes();
                match rss {
                    Some(rss_b) => debug!(
                        "Heap -> current: {}, peak: {}, allocs: {}, deallocs: {} | RSS: {}",
                        format_bytes_iec(current),
                        format_bytes_iec(peak),
                        allocs,
                        deallocs,
                        format_bytes_iec(rss_b)
                    ),
                    None => debug!(
                        "Heap -> current: {}, peak: {}, allocs: {}, deallocs: {}",
                        format_bytes_iec(current),
                        format_bytes_iec(peak),
                        allocs,
                        deallocs
                    ),
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
