mod log;
mod gfx;

fn main() {
    let log = log::Log::global();
    log.set_current_thread_name("main");
    log.add_listener(log::ConsoleLogListener::new());

    let params = gfx::ApiParameters {
        enable_debugging: true,
        app_version: gfx::Version::new(1, 0, 0),
        app_engine_version: gfx::Version::new(1, 0, 0),
        required_api_version: gfx::Version::new(1, 0, 0),
        app_name: "MyApp".to_string(),
        app_engine_name: "MyEngine".to_string(),
    };
    log.info_msg(format!("{:?}", params));
}
