#![allow(dead_code)]

//! Lua UI Scripting Module
//! 
//! Provides Lua bindings for UI rendering using the RenderFrame API.
//! Enables declarative UI definition from Lua scripts with state management.

use crate::lua::LuaEngine;
use crate::gfx::{self, RenderFrame};
use std::os::raw::c_int;
use std::ffi::CStr;
use std::cell::RefCell;

thread_local! {
    static CURRENT_FRAME: RefCell<Option<*mut RenderFrame>> = const { RefCell::new(None) };
}

/// Sets the current render frame for Lua drawing operations
pub fn set_current_frame(frame: &mut RenderFrame) {
    CURRENT_FRAME.with(|f| {
        *f.borrow_mut() = Some(frame as *mut RenderFrame);
    });
}

/// Clears the current render frame reference
pub fn clear_current_frame() {
    CURRENT_FRAME.with(|f| {
        *f.borrow_mut() = None;
    });
}

/// Gets a reference to the current render frame if available
fn with_current_frame<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut RenderFrame) -> R,
{
    CURRENT_FRAME.with(|frame_cell| {
        frame_cell.borrow().and_then(|frame_ptr| {
            if frame_ptr.is_null() {
                None
            } else {
                unsafe { Some(f(&mut *frame_ptr)) }
            }
        })
    })
}

/// Lua API: ui.rect(x, y, w, h, color)
/// Draws a filled rectangle
extern "C" fn lua_ui_rect(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let x = crate::lua::ffi::lua_tonumber(l, 1) as f32;
        let y = crate::lua::ffi::lua_tonumber(l, 2) as f32;
        let w = crate::lua::ffi::lua_tonumber(l, 3) as f32;
        let h = crate::lua::ffi::lua_tonumber(l, 4) as f32;
        let color = crate::lua::ffi::lua_tonumber(l, 5) as u32;

        with_current_frame(|frame| {
            frame.fill_rectangle(&gfx::Rectangle { x, y, w, h }, color);
        });

        0 // Number of return values
    }
}

/// Lua API: ui.text(text, x, y, scale, color, font_id)
/// Draws text at the specified position
extern "C" fn lua_ui_text(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let text_ptr = crate::lua::ffi::lua_tolstring(l, 1, std::ptr::null_mut());
        let text = CStr::from_ptr(text_ptr).to_string_lossy();
        let x = crate::lua::ffi::lua_tonumber(l, 2) as f32;
        let y = crate::lua::ffi::lua_tonumber(l, 3) as f32;
        let scale = crate::lua::ffi::lua_tonumber(l, 4) as f32;
        let color = crate::lua::ffi::lua_tonumber(l, 5) as u32;
        let font_id = crate::lua::ffi::lua_tonumber(l, 6) as u32;

        with_current_frame(|frame| {
            frame.push_text(&text, x, y, scale, color, font_id);
        });

        0
    }
}

/// Lua API: ui.get_window_size()
/// Returns window width and height
extern "C" fn lua_ui_get_window_size(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let (w, h) = with_current_frame(|frame| (frame.size.x, frame.size.y))
            .unwrap_or((0, 0));

        crate::lua::ffi::lua_pushnumber(l, w as f64);
        crate::lua::ffi::lua_pushnumber(l, h as f64);

        2 // Return 2 values
    }
}

/// Lua API: ui.push_scissor(x, y, w, h)
/// Pushes a scissor rectangle for clipping
extern "C" fn lua_ui_push_scissor(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let x = crate::lua::ffi::lua_tonumber(l, 1) as f32;
        let y = crate::lua::ffi::lua_tonumber(l, 2) as f32;
        let w = crate::lua::ffi::lua_tonumber(l, 3) as f32;
        let h = crate::lua::ffi::lua_tonumber(l, 4) as f32;

        with_current_frame(|frame| {
            frame.push_scissor(gfx::Rectangle { x, y, w, h });
        });

        0
    }
}

/// Lua API: ui.pop_scissor()
/// Pops the current scissor rectangle
extern "C" fn lua_ui_pop_scissor(l: *mut crate::lua::ffi::lua_State) -> c_int {
    with_current_frame(|frame| {
        frame.pop_scissor();
    });

    0
}

/// UI state for tracking element interactions
#[derive(Debug, Clone, Copy, Default)]
pub struct ElementState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

/// UI input state shared with Lua
#[derive(Debug, Clone, Default)]
pub struct UiInputState {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left_down: bool,
    pub mouse_left_pressed: bool,
    pub mouse_left_released: bool,
}

impl UiInputState {
    pub fn new() -> Self {
        Self::default()
    }
}

thread_local! {
    static UI_INPUT: RefCell<UiInputState> = RefCell::new(UiInputState::default());
}

/// Updates the UI input state
pub fn update_input_state(state: UiInputState) {
    UI_INPUT.with(|ui| {
        *ui.borrow_mut() = state;
    });
}

/// Gets the current UI input state
pub fn get_input_state() -> UiInputState {
    UI_INPUT.with(|ui| ui.borrow().clone())
}

/// Lua API: ui.get_mouse_pos()
/// Returns mouse x, y position
extern "C" fn lua_ui_get_mouse_pos(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let (x, y) = UI_INPUT.with(|ui| {
            let state = ui.borrow();
            (state.mouse_x, state.mouse_y)
        });

        crate::lua::ffi::lua_pushnumber(l, x as f64);
        crate::lua::ffi::lua_pushnumber(l, y as f64);

        2
    }
}

/// Lua API: ui.is_mouse_down()
/// Returns true if left mouse button is down
extern "C" fn lua_ui_is_mouse_down(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let down = UI_INPUT.with(|ui| ui.borrow().mouse_left_down);
        crate::lua::ffi::lua_pushboolean(l, if down { 1 } else { 0 });
        1
    }
}

/// Lua API: ui.is_mouse_pressed()
/// Returns true if left mouse button was just pressed this frame
extern "C" fn lua_ui_is_mouse_pressed(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let pressed = UI_INPUT.with(|ui| ui.borrow().mouse_left_pressed);
        crate::lua::ffi::lua_pushboolean(l, if pressed { 1 } else { 0 });
        1
    }
}

/// Lua API: ui.is_mouse_released()
/// Returns true if left mouse button was just released this frame
extern "C" fn lua_ui_is_mouse_released(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let released = UI_INPUT.with(|ui| ui.borrow().mouse_left_released);
        crate::lua::ffi::lua_pushboolean(l, if released { 1 } else { 0 });
        1
    }
}

/// Lua API: ui.is_rect_hovered(x, y, w, h)
/// Returns true if mouse is over the given rectangle
extern "C" fn lua_ui_is_rect_hovered(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        let x = crate::lua::ffi::lua_tonumber(l, 1) as f32;
        let y = crate::lua::ffi::lua_tonumber(l, 2) as f32;
        let w = crate::lua::ffi::lua_tonumber(l, 3) as f32;
        let h = crate::lua::ffi::lua_tonumber(l, 4) as f32;

        let hovered = UI_INPUT.with(|ui| {
            let state = ui.borrow();
            state.mouse_x >= x
                && state.mouse_x <= x + w
                && state.mouse_y >= y
                && state.mouse_y <= y + h
        });

        crate::lua::ffi::lua_pushboolean(l, if hovered { 1 } else { 0 });
        1
    }
}

/// Registers all UI functions with the Lua engine
pub fn register_ui_functions(lua: &mut LuaEngine) {
    lua.register_function("ui_rect", lua_ui_rect);
    lua.register_function("ui_text", lua_ui_text);
    lua.register_function("ui_get_window_size", lua_ui_get_window_size);
    lua.register_function("ui_push_scissor", lua_ui_push_scissor);
    lua.register_function("ui_pop_scissor", lua_ui_pop_scissor);
    lua.register_function("ui_get_mouse_pos", lua_ui_get_mouse_pos);
    lua.register_function("ui_is_mouse_down", lua_ui_is_mouse_down);
    lua.register_function("ui_is_mouse_pressed", lua_ui_is_mouse_pressed);
    lua.register_function("ui_is_mouse_released", lua_ui_is_mouse_released);
    lua.register_function("ui_is_rect_hovered", lua_ui_is_rect_hovered);

    // Register font IDs as constants
    lua.execute(
        r#"
        -- Font constants
        FONT_PROGGY_CLEAN = 0
        FONT_DEPARTURE_MONO = 1
        FONT_CASKAYDIA_MONO = 2
        FONT_NERD_SYMBOLS = 3

        -- UI namespace
        ui = {
            rect = ui_rect,
            text = ui_text,
            get_window_size = ui_get_window_size,
            push_scissor = ui_push_scissor,
            pop_scissor = ui_pop_scissor,
            get_mouse_pos = ui_get_mouse_pos,
            is_mouse_down = ui_is_mouse_down,
            is_mouse_pressed = ui_is_mouse_pressed,
            is_mouse_released = ui_is_mouse_released,
            is_rect_hovered = ui_is_rect_hovered,
        }
        "#,
    )
    .expect("Failed to setup UI namespace");
}
