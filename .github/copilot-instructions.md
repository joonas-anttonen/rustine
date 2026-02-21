# GitHub Copilot Instructions

## Workspace architecture (read first)
- This is a multi-crate Rust workspace (`Cargo.toml` at repo root) with shared core + app binaries.
- Core engine crate is `rustine/` (`gfx`, `gui`, `scene`, `io`, `log`, `lua`), reused by:
	- `rustine-ed/` (text editor prototype)
	- `rustine-explorer/` (file explorer + media preview worker)
	- `rustine-mech/` (Lua-driven UI sandbox)
	- `rustine-cc/` (GigE camera client + MJPEG stream)
- `rustine-sc/` is shader compiler FFI glue used by `rustine/build.rs` for HLSL compilation.

## Runtime model and data flow
- Typical app startup pattern (see `rustine-*/src/main.rs`):
	1. set signal handlers (`SIGINT`/`SIGTERM`) and a shared shutdown flag
	2. create `Gfx` via builder
	3. create `Gui` with a crate-local `Application` impl
	4. run `gfx::run` on a thread, `gui::run` on main thread, optional `scene::run`
- Most inter-thread communication uses `Mailbox<T>` + `AutoResetEvent` (`rustine/src/lib.rs`) rather than channels.
- UI apps usually call `gui.mark_damaged()` in input handlers to request redraw.
- `RunMode::Event` is used for on-demand redraw (`rustine-ed`, `rustine-mech`); `RunMode::Continuous` is used for streaming/live updates (`rustine-explorer`, `rustine-cc`).

## Native integrations and prerequisites
- `rustine/build.rs` builds/link native components in `rustine/ext/*`:
	- LuaJIT via `make` (`ext/luajit/src`)
	- CMake modules: `rustine-vma`, `rustine-webp`, `rustine-ffmpeg`, `rustine-wl`
	- links Vulkan, Wayland, xkbcommon, FFmpeg, WebP, and `stdc++`
- `rustine-sc/build.rs` requires environment variable:
	- `DXC_INCLUDE_DIR` pointing to directory containing `dxcapi.h`
- Build scripts assume Ninja generator (`generator = "Ninja"`).

## Build, test, and run workflow
- Preferred verification after changes:
	- `cargo build` (workspace)
	- `cargo test --tests` (workspace)
- Useful crate-scoped commands:
	- `cargo run -p rustine-ed`
	- `cargo run -p rustine-explorer`
	- `cargo run -p rustine-mech`
	- `cargo run -p rustine-cc`
- If touching media preview, scene, or camera code, test the relevant binary, not just unit tests.

## Codebase-specific conventions
- Keep changes surgical and aligned with existing patterns in each crate; avoid cross-crate refactors unless requested.
- Prefer `use std::foo;` style imports over deep item imports from `std`.
- Keep `unwrap`/`expect` limited to startup/builder boundaries; use `Result`/`Option` in runtime paths.
- Avoid adding dependencies unless necessary for learning goals; existing crates often implement functionality in-house.
- Use the project logging API (`rustine::log` + macros) instead of ad-hoc prints for runtime diagnostics.

## Where to look for examples
- App bootstrap pattern: `rustine-ed/src/main.rs`, `rustine-explorer/src/main.rs`
- Mailbox + worker thread pattern: `rustine-explorer/src/application.rs`, `rustine-explorer/src/preview.rs`
- UI DOM/layout + Lua runtime flow: `rustine-mech/src/application.rs`, `rustine/src/gui/{dom.rs,style.rs,api.rs}`
- Scene command queue flow: `rustine/src/scene.rs`
- Native build/link logic: `rustine/build.rs`, `rustine-sc/build.rs`