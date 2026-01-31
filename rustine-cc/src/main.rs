#![allow(dead_code)]

use std::sync::{Arc, Mutex, atomic};
use std::thread;

use rustine::log;

mod network;
use network::IPAdapter;

mod gige;
use gige::*;

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

    thread::scope(|scope| {
        for device in discovered_devices {
            let a_client = Arc::new(Mutex::new(GigEClient::new(device)));

            let a_client_clone = Arc::clone(&a_client);
            scope.spawn(|| {
                GigEClient::run(a_client_clone);
            });
        }
    });
}
