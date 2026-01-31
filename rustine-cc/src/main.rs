#![allow(dead_code)]

use rustine::log;

mod network;
use network::IPAdapter;

mod gige;

fn main() {
    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

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
}
