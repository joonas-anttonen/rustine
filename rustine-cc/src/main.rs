#![allow(dead_code)]

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::{Arc, Mutex, atomic};

use rustine::{Version, log};

mod network;
use network::IPAdapter;

mod gige;
use gige::*;

mod genicam;

mod mjpeg;

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

fn main() {
    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    let mut discovered_devices: Vec<GigEDevice> = Vec::new();

    let adapters: Vec<IPAdapter> = IPAdapter::get_adapters();
    if adapters.is_empty() {
        log::warning!("No ethernet IPv4 adapters found.");
    } else {
        for a in &adapters {
            log::info!("Discovering from: {:#?}", a);

            match gige::discover(a) {
                Ok(devices) => {
                    for device in devices {
                        match device {
                            Ok(d) => {
                                log::info!("Discovered device: {:#?}", d);
                                discovered_devices.push(d);
                            }
                            Err(e) => {
                                log::error!("Error discovering device: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error during discovery: {}", e);
                }
            }
        }
    }

    {
        let gfx_builder = rustine::gfx::Gfx::builder(rustine::Platform::Wayland)
            .app_name("rustine-cc")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
        let gfx = Arc::new(Mutex::new(gfx_builder.build().unwrap()));

        let a_client_image = Arc::new(gfx.lock().unwrap().create_dynamic_image());
        let application = Box::new(MyApplication::new(Arc::clone(&a_client_image)));

        let gui_builder = rustine::gui::Gui::builder(rustine::Platform::Wayland)
            .window_title("rustine-cc")
            .window_size(1280, 720)
            .window_type(rustine::gui::WindowType::Normal);
        let gui = gui_builder.build(Arc::clone(&gfx), application);

        let stream_cache = mjpeg::FrameCache::new();
        let stream_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8090);
        let _mjpeg_thread = mjpeg::start_server(Arc::clone(&stream_cache), stream_addr);

        let mode = rustine::RunMode::Continuous;
        std::thread::scope(|scope| {
            for device in discovered_devices {
                let a_client = Arc::new(Mutex::new(GigEClient::new(device)));

                let a_client = Arc::clone(&a_client);
                let a_client_image = Arc::clone(&a_client_image);
                let a_client_image_mailbox = gui.image_mailbox();
                let a_stream_cache = Some(Arc::clone(&stream_cache));
                scope.spawn(|| {
                    let run_result = GigEClient::run(
                        a_client,
                        a_client_image,
                        a_client_image_mailbox,
                        a_stream_cache,
                        &SHUTDOWN_FLAG,
                    );
                    if let Err(e) = run_result {
                        log::error!("GigEClient::run error: {}", e);
                    }
                });
            }

            scope.spawn(|| {
                rustine::gfx::run(Arc::clone(&gfx), &SHUTDOWN_FLAG, mode);
            });

            rustine::gui::run(&gui, &SHUTDOWN_FLAG, mode);

            SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
            gfx.lock().unwrap().wake_up();
        });
    }
}

struct MyApplicationState {
    camera_stream_image: Arc<rustine::gfx::Image>,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl Drop for MyApplication {
    fn drop(&mut self) {}
}

impl MyApplication {
    fn new(camera_stream_image: Arc<rustine::gfx::Image>) -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState {
                camera_stream_image,
            }),
        }
    }
}

impl rustine::gui::Application for MyApplication {
    fn startup(&self, _gui: &rustine::gui::Gui) {
        //let mut state = self.state.borrow_mut();
        //state.camera_stream_image = Arc::new(gui.create_dynamic_image());
    }

    fn on_key(&self, _gui: &rustine::gui::Gui, _key: rustine::gui::KeyEvent) {}

    fn on_char(&self, _gui: &rustine::gui::Gui, _char: char) {}

    fn render(&self, _gui: &rustine::gui::Gui, frame: &mut rustine::gfx::RenderFrame) {
        let state = self.state.borrow();

        let window_w = frame.size.x as f32;
        let window_h = frame.size.y as f32;

        frame.push_image(
            &state.camera_stream_image,
            rustine::gfx::Rectangle {
                x: 0.0,
                y: 0.0,
                w: window_w,
                h: window_h,
            },
            rustine::gfx::Fit::FILL_KEEP_ASPECT,
            0xFFFFFFFF,
        );
    }
}
