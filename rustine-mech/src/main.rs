use rustine::{log, lua::LuaEngine};

fn main() {
    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    let mut engine = LuaEngine::new();
    engine.register_log_function();

    // Execute Lua script
    let script = r#"
        log("Hello from Lua!")
        
        player_x = 100
        player_y = 200
        
        function update(delta)
            player_x = player_x + delta * 10
            log("Player moved to: " .. player_x)
        end
    "#;

    engine.execute(script).unwrap();

    // Call Lua from Rust
    engine.set_global_number("delta", 0.016);
    engine.execute("update(delta)").unwrap();

    // Read values from Lua
    if let Some(x) = engine.get_global_number("player_x") {
        println!("Player X from Rust: {}", x);
    }
}
