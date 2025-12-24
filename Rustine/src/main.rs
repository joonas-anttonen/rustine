//use std::rc::Rc;

mod gfx;
mod log;

/*type Vector3 = nalgebra::Vector3<f32>;
type Matrix4x4 = nalgebra::Matrix4<f32>;

pub enum GfxMemoryState {
    Free,
    Allocated,
}

struct GfxMemory {
    state: GfxMemoryState,
    ref_count: u32,
}

struct GfxMemoryHandle {
    sys: Rc<GfxSystem>,
    mem: GfxMemory,
}

impl Drop for GfxMemoryHandle {
    fn drop(&mut self) {
        // Logic to free or decrement reference count of the memory
    }
}

struct GfxSystem {

}

impl GfxSystem {

}

/// Represents an object in the application.
struct AppObject {
    pub tf: Matrix4x4,
    pub mem: Rc<GfxMemory>,
}

/// Represents an object in the graphics system.
struct GfxObject {
    pub tf: Matrix4x4,
    pub mem: GfxMemory,
}*/

fn main() {
    let log = log::Log::global();
    log.set_current_thread_name("main");
    log.add_listener(log::ConsoleLogListener::new());

    let params = gfx::ApiParameters {
        enable_debugging: true,
        platform: gfx::parameters::Platform::Windows,
        required_api_version: gfx::Version::new(1, 4, 0),
        app_version: gfx::Version::new(0, 1, 0),
        app_engine_version: gfx::Version::new(0, 1, 0),
        app_name: "Rustine".to_string(),
        app_engine_name: "Rustine".to_string(),
    };

    let gfx_core = match gfx::core::Core::new(&params) {
        Ok(core) => core,
        Err(e) => {
            log.logger("main", "init")
                .error(&format!("Failed to initialize graphics core: {}", e));
            return;
        }
    };
    match gfx_core.enumerate_physical_devices() {
        Ok(devices) => {
            for device in devices {
                log.logger("main", "init")
                    .info(&format!("Found device: {}", device));
            }
        }
        Err(e) => {
            log.logger("main", "init")
                .error(&format!("Failed to enumerate physical devices: {}", e));
        }
    }
}
