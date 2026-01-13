use rustine::Vector2f;
use rustine::*;
use rustine::{error, info};
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
        Platform::Wayland
    } else {
        panic!("Unsupported platform");
    };

    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleListener::new(true));

    install_signal_handlers();

    info!("STARTUP");

    {
        let mut gfx_core = match gfx::Gfx::builder(platform)
            .debugging(true)
            .app_version(Version::new(0, 1, 0))
            .app_name("rustine-app")
            .device_selector(gfx::DeviceSelector::Optimal)
            .build()
        {
            Ok(core) => core,
            Err(e) => {
                error!("Failed to build gfx::Gfx: {}", e);
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

        let application = Box::new(MyApplication);
        let gui = gui::Gui::builder(platform)
            .window_title("Rustine")
            .window_size(1920, 1080)
            .build(gfx.clone(), application);
        thread::scope(|s| {
            s.spawn(|| {
                gfx::Gfx::run(Arc::clone(&gfx), &EXIT_FLAG, gfx::LoopMode::Continuous);
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

struct MyApplication;

impl rustine::gui::Application for MyApplication {}

fn gui_thread_function(
    gui: &gui::Gui,
    render_mailbox: Arc<Mutex<VecDeque<gfx::RenderFrame>>>,
    static_image: gfx::Image,
    dynamic_image: gfx::Image,
    fallback_image: gfx::Image,
    exit_flag: &atomic::AtomicBool,
) {
    info!("GUI START");

    while !exit_flag.load(atomic::Ordering::Relaxed) && !gui.should_close() {
        gui.wait_events_timeout_ms(16);

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
    let mut frame = gfx::RenderFrame::new(frame_size);

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

    let text_font_metrics = gfx::fonts::get_font_metrics(gfx::fonts::DEPARTUREMONO_FONT_ID)
        .expect("Font metrics exist");
    let text_font_height = text_font_metrics.ascender - text_font_metrics.descender;

    let text_y = address_bar_y + ADDRESS_BAR_HEIGHT
        - ((ADDRESS_BAR_HEIGHT - text_font_height) / 2.0)
        + text_font_metrics.descender;

    let text_area = frame.push_text(
        "rustine q | code = [",
        address_bar_x,
        text_y,
        1.0,
        color,
        gfx::fonts::DEPARTUREMONO_FONT_ID,
    );

    //frame.draw_rectangle(&text_area, 1.0, color);

    let text_area = frame.push_text(
        "\u{e8da}",
        text_area.right(),
        text_y,
        1.0,
        color,
        gfx::fonts::NERDSYMBOLSMONO_FONT_ID,
    );

    let _text_area = frame.push_text(
        "] It's me, <Joonas>",
        text_area.right(),
        text_y,
        1.0,
        color,
        gfx::fonts::DEPARTUREMONO_FONT_ID,
    );

    //frame.draw_rectangle(&text_area, 1.0, color);

    // Test polyline
    {
        let test_points = vec![
            Vector2f::new(100.0, 300.0),
            Vector2f::new(100.0, 300.0),
            Vector2f::new(200.0, 250.0),
            Vector2f::new(300.0, 300.0),
            Vector2f::new(350.0, 200.0),
            Vector2f::new(450.0, 480.0),
            Vector2f::new(100.0, 300.0),
            Vector2f::new(100.0, 300.0),
        ];

        let interpolation = gfx::InterpolationMode::BSpline;
        let tesselation = 4;

        frame.fill_polyline_interpolated(&test_points, 0xFF66887F, interpolation, tesselation);
        frame.draw_polyline_interpolated(&test_points, 2.0, 0xFFFFFFFF, interpolation, tesselation);
    }

    frame
}

fn image_loader_thread(
    mailbox: Arc<Mutex<std::collections::VecDeque<(u32, io::Image)>>>,
    target_image_id: u32,
    exit_flag: &atomic::AtomicBool,
) {
    Log::global().set_current_thread_name("image-loader");

    let pictures_root = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("pictures").join("gif"))
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
