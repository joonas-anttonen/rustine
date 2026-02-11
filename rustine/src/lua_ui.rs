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
extern "C" fn lua_ui_pop_scissor(_l: *mut crate::lua::ffi::lua_State) -> c_int {
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

    // DOM building functions
    lua.register_function("ui_dom_panel", lua_ui_dom_panel);
    lua.register_function("ui_dom_text", lua_ui_dom_text);
    lua.register_function("ui_dom_button", lua_ui_dom_button);

    // Register font IDs as constants
    lua.execute(
        r#"
        -- Font constants
        FONT_PROGGY_CLEAN = 4294967290
        FONT_DEPARTURE_MONO = 4294967292
        FONT_CASKAYDIA_MONO = 4294967293
        FONT_NERD_SYMBOLS = 4294967291

        -- UI namespace (immediate mode - legacy)
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

        -- DOM builder helpers (new retained mode API)
        dom = {
            _panel = ui_dom_panel,
            _text = ui_dom_text,
            _button = ui_dom_button,
        }

        -- Helper to extract style properties from Lua table
        function dom._extract_style(style_table)
            if not style_table then return nil end
            return {
                bg_color = style_table.bg_color,
                text_color = style_table.text_color,
                border_color = style_table.border_color,
                border_width = style_table.border_width,
                padding = style_table.padding,
                margin = style_table.margin,
                font_size = style_table.font_size,
                font_id = style_table.font_id,
            }
        end

        -- Panel element builder
        function dom.panel(props)
            props = props or {}
            local normal_style = dom._extract_style(props.style and props.style.normal or props.style)
            local hover_style = dom._extract_style(props.style and props.style.hover)
            local pressed_style = dom._extract_style(props.style and props.style.pressed)
            
            return dom._panel(
                props.id,
                normal_style,
                hover_style,
                pressed_style,
                props.x or 0,
                props.y or 0,
                props.width,
                props.height,
                props.layout or "vertical",
                props.children or {}
            )
        end

        -- Text element builder
        function dom.text(props)
            props = props or {}
            local normal_style = dom._extract_style(props.style and props.style.normal or props.style)
            local hover_style = dom._extract_style(props.style and props.style.hover)
            local pressed_style = dom._extract_style(props.style and props.style.pressed)
            
            return dom._text(
                props.id,
                props.text or "",
                normal_style,
                hover_style,
                pressed_style,
                props.x or 0,
                props.y or 0,
                props.width,
                props.height
            )
        end

        -- Button element builder
        function dom.button(props)
            props = props or {}
            local normal_style = dom._extract_style(props.style and props.style.normal or props.style)
            local hover_style = dom._extract_style(props.style and props.style.hover)
            local pressed_style = dom._extract_style(props.style and props.style.pressed)
            
            return dom._button(
                props.id,
                props.text or "",
                normal_style,
                hover_style,
                pressed_style,
                props.x or 0,
                props.y or 0,
                props.width,
                props.height,
                props.on_click
            )
        end
        "#,
    )
    .expect("Failed to setup UI namespace");
}

//
// DOM Building API
//

use crate::ui_dom::{UiNode, Style, StatefulStyle, Layout, LayoutMode};

thread_local! {
    static DOM_BUILDER: RefCell<Vec<UiNode>> = RefCell::new(Vec::new());
}

/// Helper to parse style table from Lua
unsafe fn parse_style_from_lua(l: *mut crate::lua::ffi::lua_State, index: c_int) -> Option<Style> {
    use crate::lua::ffi::*;
    
    if lua_type(l, index) != LUA_TTABLE {
        return None;
    }

    let mut style = Style::default();

    // bg_color
    lua_getfield(l, index, b"bg_color\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.bg_color = Some(lua_tonumber(l, -1) as u32);
    }
    lua_pop(l, 1);

    // text_color
    lua_getfield(l, index, b"text_color\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.text_color = Some(lua_tonumber(l, -1) as u32);
    }
    lua_pop(l, 1);

    // border_color
    lua_getfield(l, index, b"border_color\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.border_color = Some(lua_tonumber(l, -1) as u32);
    }
    lua_pop(l, 1);

    // border_width
    lua_getfield(l, index, b"border_width\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.border_width = Some(lua_tonumber(l, -1) as f32);
    }
    lua_pop(l, 1);

    // font_size
    lua_getfield(l, index, b"font_size\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.font_size = Some(lua_tonumber(l, -1) as f32);
    }
    lua_pop(l, 1);

    // font_id
    lua_getfield(l, index, b"font_id\0".as_ptr() as *const i8);
    if lua_type(l, -1) == LUA_TNUMBER {
        style.font_id = Some(lua_tonumber(l, -1) as u32);
    }
    lua_pop(l, 1);

    Some(style)
}

/// Lua API: dom._panel(id, normal_style, hover_style, pressed_style, x, y, width, height, layout, children)
extern "C" fn lua_ui_dom_panel(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        use crate::lua::ffi::*;

        // Parse arguments
        let id = if lua_type(l, 1) == LUA_TSTRING {
            let id_ptr = lua_tolstring(l, 1, std::ptr::null_mut());
            Some(CStr::from_ptr(id_ptr).to_string_lossy().to_string())
        } else {
            None
        };

        let normal_style = parse_style_from_lua(l, 2).unwrap_or_default();
        let hover_style = parse_style_from_lua(l, 3);
        let pressed_style = parse_style_from_lua(l, 4);

        let x = lua_tonumber(l, 5) as f32;
        let y = lua_tonumber(l, 6) as f32;
        let width = if lua_type(l, 7) == LUA_TNUMBER {
            Some(lua_tonumber(l, 7) as f32)
        } else {
            None
        };
        let height = if lua_type(l, 8) == LUA_TNUMBER {
            Some(lua_tonumber(l, 8) as f32)
        } else {
            None
        };

        let layout_mode = if lua_type(l, 9) == LUA_TSTRING {
            let mode_ptr = lua_tolstring(l, 9, std::ptr::null_mut());
            let mode_str = CStr::from_ptr(mode_ptr).to_string_lossy();
            match mode_str.as_ref() {
                "horizontal" => LayoutMode::Horizontal,
                "absolute" => LayoutMode::Absolute,
                _ => LayoutMode::Vertical,
            }
        } else {
            LayoutMode::Vertical
        };

        // Parse children array
        let mut children = Vec::new();
        if lua_type(l, 10) == LUA_TTABLE {
            let len = lua_objlen(l, 10) as i32;
            for i in 1..=len {
                lua_rawgeti(l, 10, i);
                // Child should be a userdata or light userdata
                // For now, we'll skip this - children will be added separately
                lua_pop(l, 1);
            }
        }

        let node = UiNode::Panel {
            id,
            style: StatefulStyle {
                normal: normal_style,
                hover: hover_style,
                pressed: pressed_style,
            },
            layout: Layout {
                x,
                y,
                width,
                height,
                ..Default::default()
            },
            layout_mode,
            children,
        };

        // Return a light userdata representing the node
        // Store it in thread-local for now
        DOM_BUILDER.with(|builder| {
            let mut nodes = builder.borrow_mut();
            nodes.push(node);
            let index = nodes.len() - 1;
            lua_pushlightuserdata(l, index as *mut std::ffi::c_void);
        });

        1
    }
}

/// Lua API: dom._text(id, text, normal_style, hover_style, pressed_style, x, y, width, height)
extern "C" fn lua_ui_dom_text(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        use crate::lua::ffi::*;

        let id = if lua_type(l, 1) == LUA_TSTRING {
            let id_ptr = lua_tolstring(l, 1, std::ptr::null_mut());
            Some(CStr::from_ptr(id_ptr).to_string_lossy().to_string())
        } else {
            None
        };

        let text_ptr = lua_tolstring(l, 2, std::ptr::null_mut());
        let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();

        let normal_style = parse_style_from_lua(l, 3).unwrap_or_default();
        let hover_style = parse_style_from_lua(l, 4);
        let pressed_style = parse_style_from_lua(l, 5);

        let x = lua_tonumber(l, 6) as f32;
        let y = lua_tonumber(l, 7) as f32;
        let width = if lua_type(l, 8) == LUA_TNUMBER {
            Some(lua_tonumber(l, 8) as f32)
        } else {
            None
        };
        let height = if lua_type(l, 9) == LUA_TNUMBER {
            Some(lua_tonumber(l, 9) as f32)
        } else {
            Some(30.0) // Default text height
        };

        let node = UiNode::Text {
            id,
            text,
            style: StatefulStyle {
                normal: normal_style,
                hover: hover_style,
                pressed: pressed_style,
            },
            layout: Layout {
                x,
                y,
                width,
                height,
                ..Default::default()
            },
        };

        DOM_BUILDER.with(|builder| {
            let mut nodes = builder.borrow_mut();
            nodes.push(node);
            let index = nodes.len() - 1;
            lua_pushlightuserdata(l, index as *mut std::ffi::c_void);
        });

        1
    }
}

/// Lua API: dom._button(id, text, normal_style, hover_style, pressed_style, x, y, width, height, on_click)
extern "C" fn lua_ui_dom_button(l: *mut crate::lua::ffi::lua_State) -> c_int {
    unsafe {
        use crate::lua::ffi::*;

        let id = if lua_type(l, 1) == LUA_TSTRING {
            let id_ptr = lua_tolstring(l, 1, std::ptr::null_mut());
            Some(CStr::from_ptr(id_ptr).to_string_lossy().to_string())
        } else {
            None
        };

        let text_ptr = lua_tolstring(l, 2, std::ptr::null_mut());
        let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();

        let normal_style = parse_style_from_lua(l, 3).unwrap_or_default();
        let hover_style = parse_style_from_lua(l, 4);
        let pressed_style = parse_style_from_lua(l, 5);

        let x = lua_tonumber(l, 6) as f32;
        let y = lua_tonumber(l, 7) as f32;
        let width = if lua_type(l, 8) == LUA_TNUMBER {
            Some(lua_tonumber(l, 8) as f32)
        } else {
            Some(150.0) // Default button width
        };
        let height = if lua_type(l, 9) == LUA_TNUMBER {
            Some(lua_tonumber(l, 9) as f32)
        } else {
            Some(40.0) // Default button height
        };

        let on_click = if lua_type(l, 10) == LUA_TSTRING {
            let callback_ptr = lua_tolstring(l, 10, std::ptr::null_mut());
            Some(CStr::from_ptr(callback_ptr).to_string_lossy().to_string())
        } else {
            None
        };

        let node = UiNode::Button {
            id,
            text,
            style: StatefulStyle {
                normal: normal_style,
                hover: hover_style,
                pressed: pressed_style,
            },
            layout: Layout {
                x,
                y,
                width,
                height,
                ..Default::default()
            },
            on_click,
        };

        DOM_BUILDER.with(|builder| {
            let mut nodes = builder.borrow_mut();
            nodes.push(node);
            let index = nodes.len() - 1;
            lua_pushlightuserdata(l, index as *mut std::ffi::c_void);
        });

        1
    }
}
