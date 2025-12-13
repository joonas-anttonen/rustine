mod gfx;
mod log;

fn main() {
    let log = log::Log::global();
    log.set_current_thread_name("main");
    log.add_listener(log::ConsoleLogListener::new());

    let params = gfx::ApiParameters {
        enable_debugging: true,
        required_api_version: gfx::Version::new(1, 0, 0),
        app_version: gfx::Version::new(1, 0, 0),
        app_engine_version: gfx::Version::new(1, 0, 0),
        app_name: "Rustine".to_string(),
        app_engine_name: "Rustine".to_string(),
    };
    
    let logger = log::Log::global().logger("type_name", "main");
    logger.debug(&format!("Initializing graphics with params: {:?}", params));
}
