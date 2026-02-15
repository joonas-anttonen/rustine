#![allow(dead_code)]

use crate::Color;
use crate::gfx;
use crate::gui;
use crate::gui::{dom, style};
use crate::lua;

use std::collections::HashMap;
use std::ffi;
use std::path;

/// With null terminator for CStr compatibility.
const API_CODE_BYTES: &[u8] = concat!(include_str!("api.lua"), "\0").as_bytes();
const API_CODE: &ffi::CStr = unsafe { ffi::CStr::from_bytes_with_nul_unchecked(API_CODE_BYTES) };
const API_TABLE_NAME: &ffi::CStr = c"ui";
const CONTEXT_KEY: &ffi::CStr = c"__rustine_ui_context";
const NODE_ID_FIELD: &ffi::CStr = c"__node_id";

struct LuaDom {
    dom: dom::Dom,
    root: dom::NodeId,
    style: style::StyleComputer,
    click_handlers: HashMap<dom::NodeId, i32>,
}

impl LuaDom {
    fn new() -> Self {
        let dom = dom::Dom::new();
        let root = dom.root();
        Self {
            dom,
            root,
            style: style::StyleComputer::new(),
            click_handlers: HashMap::new(),
        }
    }

    /// Loads a Lua script from the specified string and executes it to build the UI DOM.
    ///
    /// Note: This method allocates a temporary string for FFI, so prefer `from_cstr` when possible.
    pub fn from_string(code: &str) -> Result<LuaDom, String> {
        let context = LuaDom::new();

        let engine = lua::LuaState::new();
        register_ui_api(&engine, &context)?;
        engine.execute(code)?;

        Ok(context)
    }

    /// Loads a Lua script from the specified file path and executes it to build the UI DOM.
    ///
    /// Preferred over `from_string`, since this avoids allocating temporary string for FFI.
    pub fn from_cstr(script: &ffi::CStr) -> Result<LuaDom, String> {
        let context = LuaDom::new();

        let engine = lua::LuaState::new();
        register_ui_api(&engine, &context)?;
        engine.execute_cstr(script)?;

        Ok(context)
    }
}

pub struct LuaRuntime {
    state: lua::LuaState,
    dom: Box<LuaDom>,
}

impl LuaRuntime {
    pub fn new() -> Result<Self, String> {
        let state = lua::LuaState::new();

        let dom = Box::new(LuaDom::new());
        register_ui_api(&state, &dom)?;

        Ok(Self { state, dom })
    }

    pub fn new_empty() -> Self {
        let state = lua::LuaState::new();
        let dom = Box::new(LuaDom::new());
        Self { state, dom }
    }

    pub fn from_file(path: impl AsRef<path::Path>) -> Result<LuaRuntime, String> {
        let path = path.as_ref();
        let script = std::fs::read_to_string(path)
            .map_err(|err| format!("Failed to read Lua file {}: {err}", path.display()))?;

        LuaRuntime::from_string(&script)
    }

    pub fn from_string(code: &str) -> Result<LuaRuntime, String> {
        let runtime = LuaRuntime::new()?;
        runtime.state.execute(code)?;
        Ok(runtime)
    }

    pub fn dom(&self) -> &dom::Dom {
        &self.dom.dom
    }

    pub fn dom_mut(&mut self) -> &mut dom::Dom {
        &mut self.dom.dom
    }

    pub fn dom_and_style_mut(&mut self) -> (&mut dom::Dom, &mut style::StyleComputer) {
        let context = &mut *self.dom;
        (&mut context.dom, &mut context.style)
    }

    pub fn root(&self) -> dom::NodeId {
        self.dom.root
    }

    pub fn style_mut(&mut self) -> &mut style::StyleComputer {
        &mut self.dom.style
    }

    pub fn dispatch_click(&mut self, node_id: dom::NodeId) -> bool {
        let Some(callback) = self.dom.click_handlers.get(&node_id).copied() else {
            return false;
        };

        unsafe {
            lua::ffi::lua_rawgeti(
                self.state.as_raw(),
                lua::ffi::LUA_REGISTRYINDEX,
                callback as i64,
            );
            push_node_handle(self.state.as_raw(), node_id);
            self.state.pcall(1, 0).unwrap_or_else(|err| {
                crate::log::error!("Error calling click handler for node {node_id}: {err}");
            });
        }

        true
    }
}

struct StyleBundle {
    base: Option<gui::StyleOverride>,
    hover: Option<gui::StyleOverride>,
    press: Option<gui::StyleOverride>,
    focus: Option<gui::StyleOverride>,
}

impl StyleBundle {
    fn empty() -> Self {
        Self {
            base: None,
            hover: None,
            press: None,
            focus: None,
        }
    }

    fn has_rules(&self) -> bool {
        self.hover.is_some() || self.press.is_some() || self.focus.is_some()
    }
}

fn register_ui_api(state: &lua::LuaState, context: &LuaDom) -> Result<(), String> {
    state.pushlightuserdata(context as *const _ as *mut core::ffi::c_void);
    state.setfield(lua::ffi::LUA_REGISTRYINDEX, CONTEXT_KEY);

    state.createtable(0, 5);

    fn set_field_function(state: &lua::LuaState, name: &ffi::CStr, func: lua::ffi::lua_CFunction) {
        state.pushcfunction(func);
        state.setfield(-2, name);
        // Feeling like -2 is not correct? -> Lua uses 1-based indexing.
        // -1 is top of stack, so after pushing the function, the table is at -2.
    }
    set_field_function(state, c"div", ui_div);
    set_field_function(state, c"text", ui_text);
    set_field_function(state, c"dom", ui_dom);
    set_field_function(state, c"set_text", ui_set_text);
    set_field_function(state, c"get_default_font", ui_get_default_font);

    state.setfield(lua::ffi::LUA_GLOBALSINDEX, API_TABLE_NAME);

    state.load_string(API_CODE)?;
    state.pcall(0, 0)?;

    Ok(())
}

fn get_context(state_raw: *mut lua::ffi::lua_State) -> Option<&'static mut LuaDom> {
    unsafe {
        lua::ffi::lua_getfield(state_raw, lua::ffi::LUA_REGISTRYINDEX, CONTEXT_KEY.as_ptr());
        let ptr = lua::ffi::lua_touserdata(state_raw, -1) as *mut LuaDom;
        lua::ffi::lua_pop(state_raw, 1);
        if ptr.is_null() { None } else { Some(&mut *ptr) }
    }
}

extern "C" fn ui_div(state_raw: *mut lua::ffi::lua_State) -> ffi::c_int {
    let Some(context) = get_context(state_raw) else {
        return 0;
    };

    let id = context.dom.create_div();
    let options_index = 1;

    unsafe {
        if lua::ffi::lua_gettop(state_raw) >= options_index
            && lua::ffi::lua_type(state_raw, options_index) == lua::ffi::LUA_TTABLE
        {
            apply_node_options(state_raw, options_index, id, context);
        }
    }

    push_node_handle(state_raw, id);
    1
}

extern "C" fn ui_text(state_raw: *mut lua::ffi::lua_State) -> ffi::c_int {
    unsafe {
        let Some(context) = get_context(state_raw) else {
            return 0;
        };

        let options_index = 1;
        let mut content = String::new();
        let mut font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;
        let mut scale = 1.0f32;
        let mut style_override = None;
        let mut style_rules = StyleBundle::empty();
        let mut has_explicit_size = false;

        if lua::ffi::lua_gettop(state_raw) >= options_index
            && lua::ffi::lua_type(state_raw, options_index) == lua::ffi::LUA_TTABLE
        {
            if let Some(text) = lua_field_string(state_raw, options_index, c"text") {
                content = text;
            }

            if let Some(font_value) = lua_field_integer(state_raw, options_index, c"font_id") {
                font_id = font_value as u32;
            }

            if let Some(scale_value) = lua_field_number(state_raw, options_index, c"font_scale") {
                scale = scale_value;
            }

            if let Some(style_table_index) = lua_field_table(state_raw, options_index, c"style") {
                style_rules = parse_style_bundle(state_raw, style_table_index);
                style_override = style_rules.base;
                if let Some(ref override_style) = style_override {
                    has_explicit_size = override_style.size.is_some();
                }
                lua::ffi::lua_pop(state_raw, 1);
            }
        }

        let id = context.dom.create_text(content, font_id, scale);
        if let Some(node) = context.dom.node_mut(id) {
            if let Some(text) = node.as_text_mut() {
                if let Some(override_style) = style_override.as_ref() {
                    text.style.apply_override(override_style);
                }
                if !has_explicit_size {
                    text.style.size = gui::Size::auto();
                }
            }
        }

        if style_rules.has_rules() {
            let mut rules = style::StyleRules::new(gui::StyleOverride::default());
            if let Some(hover) = style_rules.hover {
                rules = rules.with_hovered(hover);
            }
            if let Some(press) = style_rules.press {
                rules = rules.with_active(press);
            }
            if let Some(focus) = style_rules.focus {
                rules = rules.with_focused(focus);
            }
            context.style.set_rules(id, rules);
        }

        if lua::ffi::lua_gettop(state_raw) >= options_index
            && lua::ffi::lua_type(state_raw, options_index) == lua::ffi::LUA_TTABLE
        {
            if let Some(on_click_ref) =
                lua_field_function_ref(state_raw, options_index, c"on_click")
            {
                context.click_handlers.insert(id, on_click_ref);
            }
            apply_children(state_raw, options_index, id, context);
        }

        push_node_handle(state_raw, id);
        1
    }
}

extern "C" fn ui_set_text(state_raw: *mut lua::ffi::lua_State) -> ffi::c_int {
    unsafe {
        let Some(context) = get_context(state_raw) else {
            return 0;
        };

        if lua::ffi::lua_gettop(state_raw) < 2 {
            return 0;
        }

        let Some(node_id) = lua_node_id(state_raw, 1) else {
            return 0;
        };

        let Some(text) = lua_string(state_raw, 2) else {
            return 0;
        };

        context.dom.set_text(node_id, text);
        0
    }
}

extern "C" fn ui_dom(state_raw: *mut lua::ffi::lua_State) -> ffi::c_int {
    let Some(context) = get_context(state_raw) else {
        return 0;
    };

    unsafe {
        if lua::ffi::lua_gettop(state_raw) < 1 {
            return 0;
        }

        if let Some(node_id) = lua_node_id(state_raw, 1) {
            context.dom.append_child(context.root, node_id);
        }

        0
    }
}

extern "C" fn ui_get_default_font(state_raw: *mut lua::ffi::lua_State) -> ffi::c_int {
    unsafe {
        lua::ffi::lua_pushinteger(state_raw, gfx::fonts::CASKAYDIAMONO_FONT_ID as i64);
        1
    }
}

fn apply_node_options(
    state: *mut lua::ffi::lua_State,
    options_index: ffi::c_int,
    node_id: dom::NodeId,
    context: &mut LuaDom,
) {
    let mut style_rules = StyleBundle::empty();
    let mut style_override = None;

    if let Some(style_table_index) = lua_field_table(state, options_index, c"style") {
        style_rules = parse_style_bundle(state, style_table_index);
        style_override = style_rules.base;
        unsafe {
            lua::ffi::lua_pop(state, 1);
        }
    }

    if let Some(node) = context.dom.node_mut(node_id) {
        if let Some(style_ref) = node.style_mut() {
            if let Some(override_style) = style_override.as_ref() {
                style_ref.apply_override(override_style);
            }
        }
    }

    if style_rules.has_rules() {
        let mut rules = style::StyleRules::new(gui::StyleOverride::default());
        if let Some(hover) = style_rules.hover {
            rules = rules.with_hovered(hover);
        }
        if let Some(press) = style_rules.press {
            rules = rules.with_active(press);
        }
        if let Some(focus) = style_rules.focus {
            rules = rules.with_focused(focus);
        }
        context.style.set_rules(node_id, rules);
    }

    if let Some(on_click_ref) = lua_field_function_ref(state, options_index, c"on_click") {
        context.click_handlers.insert(node_id, on_click_ref);
    }

    apply_children(state, options_index, node_id, context);
}

fn apply_children(
    pstate: *mut lua::ffi::lua_State,
    options_index: ffi::c_int,
    node_id: dom::NodeId,
    context: &mut LuaDom,
) {
    let Some(children_index) = lua_field_table(pstate, options_index, c"children") else {
        return;
    };

    unsafe {
        let len = lua::ffi::lua_objlen(pstate, children_index) as i64;
        for idx in 1..=len {
            lua::ffi::lua_rawgeti(pstate, children_index, idx);
            if let Some(child_id) = lua_node_id(pstate, -1) {
                context.dom.append_child(node_id, child_id);
            }
            lua::ffi::lua_pop(pstate, 1);
        }

        lua::ffi::lua_pop(pstate, 1);
    }
}

fn push_node_handle(state: *mut lua::ffi::lua_State, node_id: dom::NodeId) {
    unsafe {
        lua::ffi::lua_createtable(state, 0, 1);
        lua::ffi::lua_pushinteger(state, node_id as i64);
        lua::ffi::lua_setfield(state, -2, NODE_ID_FIELD.as_ptr());
    }
}

fn lua_node_id(state: *mut lua::ffi::lua_State, idx: ffi::c_int) -> Option<dom::NodeId> {
    unsafe {
        let value_type = lua::ffi::lua_type(state, idx);
        if value_type == lua::ffi::LUA_TNUMBER {
            let value = lua::ffi::lua_tointeger(state, idx);
            if value >= 0 {
                return Some(value as dom::NodeId);
            }
            return None;
        }

        if value_type != lua::ffi::LUA_TTABLE {
            return None;
        }

        lua::ffi::lua_getfield(state, idx, NODE_ID_FIELD.as_ptr());
        let id_value = if lua::ffi::lua_type(state, -1) == lua::ffi::LUA_TNUMBER {
            let value = lua::ffi::lua_tointeger(state, -1);
            if value >= 0 {
                Some(value as dom::NodeId)
            } else {
                None
            }
        } else {
            None
        };
        lua::ffi::lua_pop(state, 1);
        id_value
    }
}

fn lua_field_table(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<ffi::c_int> {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        if lua::ffi::lua_type(pstate, -1) == lua::ffi::LUA_TTABLE {
            Some(lua_abs_index(pstate, -1))
        } else {
            lua::ffi::lua_pop(pstate, 1);
            None
        }
    }
}

fn lua_field_bool(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<bool> {
    let index = lua_abs_index(pstate, table_index);
    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = match lua::ffi::lua_type(pstate, -1) {
            lua::ffi::LUA_TBOOLEAN => Some(lua::ffi::lua_toboolean(pstate, -1) != 0),
            _ => None,
        };
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

fn lua_field_number(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<f32> {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = lua_number(pstate, -1);
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

fn lua_field_integer(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<i64> {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = lua_integer(pstate, -1);
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

fn lua_field_function_ref(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<i32> {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        if lua::ffi::lua_type(pstate, -1) == lua::ffi::LUA_TFUNCTION {
            let reference = lua::ffi::luaL_ref(pstate, lua::ffi::LUA_REGISTRYINDEX);
            Some(reference)
        } else {
            lua::ffi::lua_pop(pstate, 1);
            None
        }
    }
}

fn lua_field_string(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<String> {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = lua_string(pstate, -1);
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

fn lua_field_string_func<F: FnOnce(&ffi::CStr)>(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
    func: F,
) -> bool {
    let index = lua_abs_index(pstate, table_index);

    unsafe {
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = lua_string_func(pstate, -1, func);
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

/// `lua_type` == `LUA_TSTRING` into `lua_tolstring` at the specified index.
/// Wraps the returned Lua string into `CStr` and calls `func` with it.
///
/// Returns `true` if found valid string and `func` was called, otherwise `false`.
fn lua_string_func<F: FnOnce(&ffi::CStr)>(
    pstate: *mut lua::ffi::lua_State,
    idx: ffi::c_int,
    func: F,
) -> bool {
    unsafe {
        if lua::ffi::lua_type(pstate, idx) != lua::ffi::LUA_TSTRING {
            return false;
        }
        let mut len = 0usize;
        let ptr = lua::ffi::lua_tolstring(pstate, idx, &mut len);
        if ptr.is_null() {
            return false;
        }
        let c_str = ffi::CStr::from_ptr(ptr);
        func(c_str);
        true
    }
}

/// `lua_type` == `LUA_TSTRING` into `lua_tolstring` at the specified index,
/// then convert to Rust `String`.
///
/// Returns `Some(String)` if found valid string, otherwise `None`.
fn lua_string(pstate: *mut lua::ffi::lua_State, idx: ffi::c_int) -> Option<String> {
    unsafe {
        if lua::ffi::lua_type(pstate, idx) != lua::ffi::LUA_TSTRING {
            return None;
        }
        let mut len = 0usize;
        let ptr = lua::ffi::lua_tolstring(pstate, idx, &mut len);
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn lua_number(pstate: *mut lua::ffi::lua_State, idx: ffi::c_int) -> Option<f32> {
    unsafe {
        if lua::ffi::lua_type(pstate, idx) != lua::ffi::LUA_TNUMBER {
            return None;
        }
        Some(lua::ffi::lua_tonumber(pstate, idx) as f32)
    }
}

fn lua_integer(pstate: *mut lua::ffi::lua_State, idx: ffi::c_int) -> Option<i64> {
    unsafe {
        if lua::ffi::lua_type(pstate, idx) != lua::ffi::LUA_TNUMBER {
            return None;
        }
        Some(lua::ffi::lua_tointeger(pstate, idx) as i64)
    }
}

fn lua_abs_index(pstate: *mut lua::ffi::lua_State, idx: ffi::c_int) -> ffi::c_int {
    if idx > 0 || idx <= lua::ffi::LUA_REGISTRYINDEX {
        idx
    } else {
        unsafe { lua::ffi::lua_gettop(pstate) + idx + 1 }
    }
}

fn parse_style_bundle(pstate: *mut lua::ffi::lua_State, table_index: ffi::c_int) -> StyleBundle {
    unsafe {
        if lua::ffi::lua_type(pstate, table_index) != lua::ffi::LUA_TTABLE {
            return StyleBundle::empty();
        }

        let index = lua_abs_index(pstate, table_index);
        let mut bundle = StyleBundle::empty();

        // Normal style is inlined in the main table, while hover/press/focus are in subtables.
        bundle.base = parse_style_table(pstate, index);

        if let Some(hover_index) = lua_field_table(pstate, index, c"hover") {
            bundle.hover = parse_style_table(pstate, hover_index);
            lua::ffi::lua_pop(pstate, 1);
        }

        if let Some(press_index) = lua_field_table(pstate, index, c"press") {
            bundle.press = parse_style_table(pstate, press_index);
            lua::ffi::lua_pop(pstate, 1);
        }

        if let Some(focus_index) = lua_field_table(pstate, index, c"focus") {
            bundle.focus = parse_style_table(pstate, focus_index);
            lua::ffi::lua_pop(pstate, 1);
        }

        bundle
    }
}

fn parse_style_table(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
) -> Option<gui::StyleOverride> {
    unsafe {
        if lua::ffi::lua_type(pstate, table_index) != lua::ffi::LUA_TTABLE {
            return None;
        }

        let index = lua_abs_index(pstate, table_index);
        let mut override_style = gui::StyleOverride::default();
        let mut any = false;

        if let Some(layout) = parse_layout_style(pstate, index) {
            override_style.layout = Some(layout);
            any = true;
        }

        if let Some(size) = parse_size2_from_dimensions(pstate, index, c"width", c"height") {
            override_style.size = Some(size);
            any = true;
        }

        if let Some(size) = parse_size2_from_dimensions(pstate, index, c"min_width", c"min_height")
        {
            override_style.min_size = Some(size);
            any = true;
        }

        if let Some(size) = parse_size2_from_dimensions(pstate, index, c"max_width", c"max_height")
        {
            override_style.max_size = Some(size);
            any = true;
        }

        if let Some(position) = parse_position_mode(pstate, index, c"mode") {
            override_style.position_mode = Some(position);
            any = true;
        }

        if let Some(x) = lua_field_number(pstate, index, c"x") {
            override_style.x = Some(x);
            any = true;
        }

        if let Some(y) = lua_field_number(pstate, index, c"y") {
            override_style.y = Some(y);
            any = true;
        }

        if let Some(padding) = parse_edge_sizes_field(pstate, index, c"padding") {
            override_style.padding = Some(padding);
            any = true;
        }

        if let Some(margin) = parse_edge_sizes_field(pstate, index, c"margin") {
            override_style.margin = Some(margin);
            any = true;
        }

        if let Some(border) = parse_edge_sizes_field(pstate, index, c"border") {
            override_style.border = Some(border);
            any = true;
        }

        if let Some(color) = parse_color_field(pstate, index, c"border_color") {
            override_style.border_color = Some(color);
            any = true;
        }

        if let Some(color) = parse_color_field(pstate, index, c"foreground") {
            override_style.foreground = Some(color);
            any = true;
        }

        if let Some(color) = parse_color_field(pstate, index, c"background") {
            override_style.background = Some(color);
            any = true;
        }

        if any { Some(override_style) } else { None }
    }
}

fn parse_layout_style(
    state: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
) -> Option<gui::LayoutStyle> {
    let mut layout = gui::LayoutStyle::default();
    let mut any = false;

    if let Some(layout_index) = lua_field_table(state, table_index, c"layout") {
        if let Some(parsed) = parse_layout_style_table(state, layout_index) {
            layout = parsed;
            any = true;
        }
        unsafe {
            lua::ffi::lua_pop(state, 1);
        }
    } else if let Some(parsed) = parse_layout_style_table(state, table_index) {
        layout = parsed;
        any = true;
    }

    if any { Some(layout) } else { None }
}

fn parse_layout_style_table(
    state: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
) -> Option<gui::LayoutStyle> {
    let mut layout = gui::LayoutStyle::default();
    let mut any = false;

    lua_field_string_func(state, table_index, c"direction", |field_string| {
        if let Some(parsed) = if field_string == c"row" {
            Some(gui::LayoutDirection::Row)
        } else if field_string == c"column" {
            Some(gui::LayoutDirection::Column)
        } else {
            crate::log::warning!(
                "Invalid [direction] '{:?}' Provide one of: 'row', 'column'.",
                field_string
            );
            None
        } {
            layout.direction = parsed;
            any = true;
        }
    });

    lua_field_string_func(state, table_index, c"align", |field_string| {
        if let Some(parsed) = if field_string == c"start" {
            Some(gui::Align::Start)
        } else if field_string == c"center" {
            Some(gui::Align::Center)
        } else if field_string == c"end" {
            Some(gui::Align::End)
        } else if field_string == c"stretch" {
            Some(gui::Align::Stretch)
        } else {
            crate::log::warning!(
                "Invalid [align] '{:?}' Provide one of: 'start', 'center', 'end', 'stretch'.",
                field_string
            );
            None
        } {
            layout.align = parsed;
            any = true;
        }
    });

    lua_field_string_func(state, table_index, c"justify", |field_string| {
        if let Some(parsed) = if field_string == c"start" {
            Some(gui::Justify::Start)
        } else if field_string == c"center" {
            Some(gui::Justify::Center)
        } else if field_string == c"end" {
            Some(gui::Justify::End)
        } else if field_string == c"space-between" {
            Some(gui::Justify::SpaceBetween)
        } else if field_string == c"space-around" {
            Some(gui::Justify::SpaceAround)
        } else if field_string == c"space-evenly" {
            Some(gui::Justify::SpaceEvenly)
        } else {
            crate::log::warning!(
                "Invalid [justify] '{:?}' Provide one of: 'start', 'center', 'end', 'space-between', 'space-around', 'space-evenly'.",
                field_string
            );
            None
        } {
            layout.justify = parsed;
            any = true;
        }
    });

    if let Some(gap) = lua_field_number(state, table_index, c"gap") {
        layout.gap = gap.max(0.0);
        any = true;
    }

    if any { Some(layout) } else { None }
}

fn parse_size2_from_dimensions(
    state: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    width_name: &ffi::CStr,
    height_name: &ffi::CStr,
) -> Option<gui::Size> {
    let width = parse_length_field(state, table_index, width_name);
    let height = parse_length_field(state, table_index, height_name);
    if width.is_none() && height.is_none() {
        return None;
    }

    Some(gui::Size {
        width: width.unwrap_or(gui::Length::Auto),
        height: height.unwrap_or(gui::Length::Auto),
    })
}

fn parse_length_field(
    state: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<gui::Length> {
    unsafe {
        let index = lua_abs_index(state, table_index);
        lua::ffi::lua_getfield(state, index, name.as_ptr());
        let result = parse_length_value(state, -1);
        lua::ffi::lua_pop(state, 1);
        result
    }
}

fn parse_length_value(state: *mut lua::ffi::lua_State, idx: ffi::c_int) -> Option<gui::Length> {
    unsafe {
        match lua::ffi::lua_type(state, idx) {
            lua::ffi::LUA_TNUMBER => {
                let value = lua::ffi::lua_tonumber(state, idx) as f32;
                Some(gui::Length::Px(value.max(0.0)))
            }
            lua::ffi::LUA_TSTRING => {
                let mut result = None;
                lua_string_func(state, idx, |value| {
                    if let Ok(value) = value.to_str() {
                        result = {
                            let trimmed = value.trim();
                            let lower = trimmed.to_ascii_lowercase();
                            if lower == "auto" {
                                Some(gui::Length::Auto)
                            } else if lower == "fill" {
                                Some(gui::Length::Fill)
                            } else if let Some(percent) = lower.strip_suffix('%')
                                && let Ok(value) = percent.trim().parse::<f32>()
                            {
                                Some(gui::Length::Percent((value / 100.0).max(0.0)))
                            } else if let Some(px) = lower.strip_suffix("px")
                                && let Ok(value) = px.trim().parse::<f32>()
                            {
                                Some(gui::Length::Px(value.max(0.0)))
                            } else if let Ok(value) = lower.parse::<f32>() {
                                Some(gui::Length::Px(value.max(0.0)))
                            } else {
                                crate::log::warning!(
                                    "Invalid length value '{:?}'. Provide a number (pixels), a percentage string like '50%', or one of the keywords: 'auto', 'fill'.",
                                    value
                                );
                                None
                            }
                        }
                    }
                });
                result
            }
            _ => None,
        }
    }
}

fn parse_edge_sizes_field(
    state: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<gui::EdgeSizes> {
    let index = lua_abs_index(state, table_index);

    unsafe {
        lua::ffi::lua_getfield(state, index, name.as_ptr());
        let result = parse_edge_sizes_value(state, -1);
        lua::ffi::lua_pop(state, 1);
        result
    }
}

fn parse_edge_sizes_value(
    state: *mut lua::ffi::lua_State,
    idx: ffi::c_int,
) -> Option<gui::EdgeSizes> {
    unsafe {
        match lua::ffi::lua_type(state, idx) {
            lua::ffi::LUA_TNUMBER => {
                let value = lua::ffi::lua_tonumber(state, idx) as f32;
                Some(gui::EdgeSizes {
                    left: value,
                    right: value,
                    top: value,
                    bottom: value,
                })
            }
            lua::ffi::LUA_TTABLE => {
                let index = lua_abs_index(state, idx);
                let mut edge = gui::EdgeSizes::zero();
                let mut any = false;

                if let Some(value) = lua_field_number(state, index, c"left") {
                    edge.left = value;
                    any = true;
                }
                if let Some(value) = lua_field_number(state, index, c"right") {
                    edge.right = value;
                    any = true;
                }
                if let Some(value) = lua_field_number(state, index, c"top") {
                    edge.top = value;
                    any = true;
                }
                if let Some(value) = lua_field_number(state, index, c"bottom") {
                    edge.bottom = value;
                    any = true;
                }

                if let Some(value) = lua_field_number(state, index, c"x") {
                    edge.left = value;
                    edge.right = value;
                    any = true;
                }
                if let Some(value) = lua_field_number(state, index, c"y") {
                    edge.top = value;
                    edge.bottom = value;
                    any = true;
                }

                if any { Some(edge) } else { None }
            }
            _ => None,
        }
    }
}

fn parse_color_field(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<Color> {
    unsafe {
        let index = lua_abs_index(pstate, table_index);
        lua::ffi::lua_getfield(pstate, index, name.as_ptr());
        let result = {
            let idx = -1;
            match lua::ffi::lua_type(pstate, idx) {
                lua::ffi::LUA_TNUMBER => {
                    let value = lua::ffi::lua_tonumber(pstate, idx) as u32;
                    Some(Color::from_u32(value))
                }
                lua::ffi::LUA_TSTRING => {
                    let mut result = None;
                    lua_string_func(pstate, idx, |value| {
                        result = parse_color_cstr(value);
                    });
                    result
                }
                _ => None,
            }
        };
        lua::ffi::lua_pop(pstate, 1);
        result
    }
}

fn parse_color_cstr(value: &ffi::CStr) -> Option<Color> {
    let Ok(value) = value.to_str() else {
        return None;
    };

    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        let value = match hex.len() {
            6 => {
                let rgb = u32::from_str_radix(hex, 16).ok()?;
                (rgb << 8) | 0xFF
            }
            8 => u32::from_str_radix(hex, 16).ok()?,
            _ => return None,
        };
        Some(Color::from_u32(value))
    } else if let Some(hex) = trimmed.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok().map(Color::from_u32)
    } else {
        None
    }
}

fn parse_position_mode(
    pstate: *mut lua::ffi::lua_State,
    table_index: ffi::c_int,
    name: &ffi::CStr,
) -> Option<gui::PositionMode> {
    let mut result = None;

    lua_field_string_func(pstate, table_index, name, |field_string| {
        if let Some(parsed) = {
            if field_string == c"flow" {
                Some(gui::PositionMode::Flow)
            } else if field_string == c"absolute" {
                Some(gui::PositionMode::Absolute)
            } else {
                crate::log::warning!(
                    "Invalid [mode] '{:?}' Provide one of: 'flow', 'absolute'.",
                    field_string
                );
                None
            }
        } {
            result = Some(parsed);
        }
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_simple_div_creation() {
        let code = cr###"
            local div = ui.div({
                style = { background = "#FF0000FF" },
            })
            ui.dom(div)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1, "Root should have 1 child");

        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        assert!(
            child.as_div().is_some(),
            "Child should be a div, got {:?}",
            child.kind
        );
    }

    #[test]
    fn lua_text_node_creation() {
        let code = cr###"
            local text = ui.text({
                text = "Hello World",
                font_scale = 1.5,
            })
            ui.dom(text)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1, "Root should have 1 child");

        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        let text_node = child.as_text().expect("Should be a text node");
        assert_eq!(text_node.content, "Hello World");
        assert_eq!(text_node.scale, 1.5);
    }

    #[test]
    fn lua_nested_children() {
        let code = cr###"
            local parent = ui.div({
                children = {
                    ui.text({ text = "Child 1" }),
                    ui.text({ text = "Child 2" }),
                    ui.div({ children = { ui.text({ text = "Grandchild" }) } }),
                },
            })
            ui.dom(parent)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1);

        let parent_id = root.children[0];
        let parent = lua_dom.dom.node(parent_id).expect("Parent should exist");
        assert_eq!(parent.children.len(), 3, "Parent should have 3 children");

        let grandchild_parent_id = parent.children[2];
        let grandchild_parent = lua_dom
            .dom
            .node(grandchild_parent_id)
            .expect("Grandchild parent should exist");
        assert_eq!(
            grandchild_parent.children.len(),
            1,
            "Grandchild parent should have 1 child"
        );

        let grandchild_id = grandchild_parent.children[0];
        let grandchild = lua_dom
            .dom
            .node(grandchild_id)
            .expect("Grandchild should exist");
        let grandchild_text = grandchild.as_text().expect("Should be text");
        assert_eq!(grandchild_text.content, "Grandchild");
    }

    #[test]
    fn lua_style_application() {
        let code = cr###"
            local div = ui.div({
                style = {
                    background = "#AABBCCFF",
                    padding = 10,
                    margin = { left = 5, right = 5, top = 2, bottom = 2 },
                },
            })
            ui.dom(div)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        let style = child.style().expect("Should have style");

        assert_eq!(style.background, crate::Color::from_u32(0xAABBCCFF));
        assert_eq!(style.padding.left, 10.0);
        assert_eq!(style.padding.right, 10.0);
        assert_eq!(style.margin.left, 5.0);
        assert_eq!(style.margin.right, 5.0);
        assert_eq!(style.margin.top, 2.0);
        assert_eq!(style.margin.bottom, 2.0);
    }

    #[test]
    fn lua_absolute_positioning() {
        let code = cr###"
            local div = ui.div({
                style = {
                    mode = "absolute",
                    x = 12,
                    y = 34,
                },
            })
            ui.dom(div)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        let style = child.style().expect("Should have style");

        assert_eq!(style.position, crate::gui::PositionMode::Absolute);
        assert_eq!(style.x, 12.0);
        assert_eq!(style.y, 34.0);
    }

    #[test]
    fn lua_compose_helper() {
        let code = cr###"
            local base = { background = "#FF0000FF", padding = 10 }
            local override = ui.compose(base, { background = "#00FF00FF" })

            local div = ui.div({ style = override })
            ui.dom(div)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        let style = child.style().expect("Should have style");

        assert_eq!(
            style.background,
            crate::Color::from_u32(0x00FF00FF),
            "Override should win"
        );
        assert_eq!(style.padding.left, 10.0, "Base padding should persist");
    }

    #[test]
    fn lua_label_helper() {
        let code = cr###"
            local label = ui.label("Test Label", { scale = 2.0 })
            ui.dom(label)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom.dom.node(child_id).expect("Child should exist");
        let text = child.as_text().expect("Should be text");
        assert_eq!(text.content, "Test Label");
        assert_eq!(text.scale, 2.0);
    }

    #[test]
    fn lua_error_handling() {
        let code = cr#"
            local invalid = ui.div_does_not_exist({
                children = {}
            })
        "#;
        let result = LuaDom::from_cstr(code);
        assert!(result.is_err(), "Should fail with undefined function");
    }

    #[test]
    fn lua_color_parsing() {
        let code = cr###"
            local div1 = ui.div({ style = { background = "#FF0000FF" } })
            local div3 = ui.div({ style = { background = 0xFF0000FF } })
            ui.dom(div1)
            ui.dom(div3)
        "###;
        let lua_dom = LuaDom::from_cstr(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 2, "Should have 2 children");

        for i in 0..2 {
            let child = lua_dom
                .dom
                .node(root.children[i])
                .expect("Child should exist");
            let style = child.style().expect("Should have style");
            assert_eq!(
                style.background,
                crate::Color::from_u32(0xFF0000FF),
                "Color {} should be red",
                i
            );
        }
    }
}
