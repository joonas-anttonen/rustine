use rustine::{error, info};
use rustine::{gfx, gui, log::ConsoleLogListener, log::Log, version::Version};

#[cfg(unix)]
use libc;
use std::io;
use std::sync::{Arc, Mutex, atomic};
use std::thread;

static EXIT_FLAG: atomic::AtomicBool = atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn handle_termination_signal(_signal: i32) {
    EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
}

#[cfg(unix)]
fn install_signal_handlers() -> io::Result<()> {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_termination_signal as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
        if libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn install_signal_handlers() -> io::Result<()> {
    Ok(())
}
use crate::gfx::dxc_ffi::{DxcCompiler, RdxcShaderStage};
fn test_dxc() {
    let simple_shader_str = r#"
struct PerCommand
{
	float2 Scale;
    bool isSdf;
	float sdfRange;
};

[[vk::push_constant]] PerCommand command;

[[vk::binding(0, 0)]] Texture2D commandTexture;
[[vk::binding(1, 0)]] SamplerState commandSampler;

struct vertex_input
{
	float2 Position : POSITION0;
	float2 UV : TEXCOORD0;
	uint Color : COLOR0;
};

struct fragment_input
{
	float4 Position : SV_POSITION;
	float2 UV : TEXCOORD0;
	float4 Color : COLOR0;
};

float4 UnpackColor(uint packed)
{
    float r = (float)(packed & 0xFF) / 255.0f;
    float g = (float)((packed >> 8) & 0xFF) / 255.0f;
    float b = (float)((packed >> 16) & 0xFF) / 255.0f;
    float a = (float)((packed >> 24) & 0xFF) / 255.0f;
    return float4(r, g, b, a);
}

[shader("vertex")]
fragment_input vertex(vertex_input input, in uint vertexIndex : SV_VertexID)
{
    fragment_input output = (fragment_input)0;
    output.Position = float4(input.Position * command.Scale + float2(-1, -1), 0.0, 1.0);
	output.UV = input.UV;
	output.Color = UnpackColor(input.Color);
	return output;
}

[shader("pixel")]
float4 fragment(fragment_input input) : SV_TARGET
{ 
	float4 geometryColor = input.Color;
	float4 textureColor = commandTexture.Sample(commandSampler, input.UV);

    float alpha = textureColor.a;

	if (command.isSdf)
	{ 
		float sdf = textureColor.a - 0.5;
		alpha = smoothstep(-command.sdfRange, +command.sdfRange, sdf);
	}

	return float4(geometryColor.rgb * textureColor.rgb, alpha * geometryColor.a);
}"#;

    let compiler = DxcCompiler::new().unwrap();
    let compile_result = compiler.compile(
        simple_shader_str,
        "vertex",
        RdxcShaderStage::Vertex,
        &[],
    );
    println!("Compile result: {:?}", compile_result);
}

fn main() {
    let log = Log::global();
    log.set_current_thread_name("main");
    log.add_listener(ConsoleLogListener::new(true));

    test_dxc();
    return;

    if let Err(e) = install_signal_handlers() {
        error!("Failed to install signal handlers: {}", e);
    }

    info!("STARTUP");

    {
        let host_platform = if cfg!(target_os = "windows") {
            gfx::Platform::Windows
        } else if cfg!(target_os = "linux") {
            // For Linux, we default to Wayland; adjust as necessary.
            gfx::Platform::Wayland
        } else {
            panic!("Unsupported platform");
        };

        let params = gfx::StartupParameters {
            enable_debugging: true,
            host_platform: host_platform,
            host_version: Version::new(0, 1, 0),
            host_name: "rustine-app".to_string(),
        };

        let gfx_core = match gfx::Core::builder(params).select_optimal_device().build() {
            Ok(core) => core,
            Err(e) => {
                error!("Failed to build gfx::Core: {}", e);
                return;
            }
        };

        let dev = gfx_core.selected_physical_device();
        info!("Selected device: {}", dev);

        let gfx = Arc::new(Mutex::new(gfx_core));

        let gui_params = gui::StartupParameters {
            platform: host_platform,
            window_title: "Rustine".to_string(),
            window_width: Some(1920),
            window_height: Some(1080),
        };
        let gui = gui::Gui::new(Arc::clone(&gfx), gui_params);

        thread::scope(|s| {
            s.spawn(|| {
                gfx_thread_function(Arc::clone(&gfx), &EXIT_FLAG);
            });

            gui_thread_function(&gui, &EXIT_FLAG);

            EXIT_FLAG.store(true, atomic::Ordering::Relaxed);
        });
    }

    info!("SHUTDOWN");
}

fn gui_thread_function(gui: &gui::Gui, exit_flag: &atomic::AtomicBool) {
    info!("GUI START");

    while !exit_flag.load(atomic::Ordering::Relaxed) && !gui.should_close() {
        gui.process_events();
    }
    info!("GUI STOP");
}

fn gfx_thread_function(gfx: Arc<Mutex<gfx::Core>>, exit_flag: &atomic::AtomicBool) {
    Log::global().set_current_thread_name("gfx");

    info!("GFX START");

    let mut skip_frame = false;
    let start_instant = std::time::Instant::now();
    let mut last_instant = std::time::Instant::now();

    while !exit_flag.load(atomic::Ordering::Relaxed) {
        {
            let attempted_lock = gfx.try_lock();
            if attempted_lock.is_err() {
                skip_frame = true;
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            if skip_frame {
                info!("CONTINUE");
                skip_frame = false;
            }
            let mut core = attempted_lock.unwrap();

            let now = std::time::Instant::now();
            let t = now.duration_since(start_instant).as_secs_f64();
            let dt = now.duration_since(last_instant).as_secs_f32();
            core.render(t, dt);
            last_instant = now;
        }

        // NOTE: Sleeping seems necessary
        // If we don't sleep, the host thread struggles to acquire the lock,
        // causing unresponsiveness, especially when resizing the window.
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    info!("GFX STOP");
}
