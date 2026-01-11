use std::sync::atomic;

use rustine::{Version, log};

static SHUTDOWN_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

extern "C" fn handle_sigterm(_signal: i32) {
    log::info!("SIGTERM");
    SHUTDOWN_FLAG.store(true, atomic::Ordering::Relaxed);
}
fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_sigterm as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            let os_error = std::io::Error::last_os_error();
            eprintln!("Failed to install SIGINT handler: {os_error:?}",);
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            let os_error = std::io::Error::last_os_error();
            eprintln!("Failed to install SIGTERM handler: {os_error:?}",);
        }
    }
}

struct Arguments {}
fn parse_arguments() -> Option<Arguments> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 {
        None
    } else {
        Some(Arguments {})
    }
}

fn main() -> std::process::ExitCode {
    install_signal_handlers();

    let args = parse_arguments();
    if args.is_none() {
        eprintln!("Usage: {}", "");
        return std::process::ExitCode::from(1);
    }

    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    log::debug!("STARTUP");

    {
        let _builder = rustine::gfx::Gfx::builder(rustine::Platform::Wayland)
            .app_name("rustine-desktop")
            .app_version(Version::new(0, 1, 0))
            .debugging(true)
            .device_selector(rustine::gfx::DeviceSelector::Optimal);
    }

    log::debug!("SHUTDOWN");

    std::process::ExitCode::SUCCESS
}
