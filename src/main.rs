mod log;

fn main() {
    let log = log::Log::global();
    log.set_current_thread_name("main");
    log.add_listener(log::ConsoleLogListener::new());
}
