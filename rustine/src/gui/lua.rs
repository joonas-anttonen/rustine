#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

use crate::Color;
use crate::gfx;
use crate::gui::{dom, style};
use crate::lua;

use std::collections::HashMap;
use std::ffi;
use std::os::raw;
use std::path;

const CONTEXT_KEY: &[u8] = b"__rustine_ui_context\0";
const NODE_ID_FIELD: &str = "__node_id";

pub struct LuaDom {
    pub dom: dom::Dom,
    pub root: dom::NodeId,
    pub style: style::StyleComputer,
}

pub fn load_dom_from_file(path: impl AsRef<path::Path>) -> Result<LuaDom, String> {
    let mut engine = lua::LuaEngine::new();
    engine.register_log_function();

    let mut context = LuaUiContext::new();
    unsafe {
        register_ui_api(engine.state(), &mut context)?;
    }

    engine.execute_file(path)?;

    unsafe {
        clear_context(engine.state());
    }

    Ok(LuaDom {
        dom: context.dom,
        root: context.root,
        style: context.style,
    })
}

pub fn load_dom_from_string(code: &str) -> Result<LuaDom, String> {
    let mut engine = lua::LuaEngine::new();
    engine.register_log_function();

    let mut context = LuaUiContext::new();
    unsafe {
        register_ui_api(engine.state(), &mut context)?;
    }

    engine.execute(code)?;

    unsafe {
        clear_context(engine.state());
    }

    Ok(LuaDom {
        dom: context.dom,
        root: context.root,
        style: context.style,
    })
}

struct LuaUiContext {
    dom: dom::Dom,
    root: dom::NodeId,
    style: style::StyleComputer,
    click_handlers: HashMap<dom::NodeId, i32>,
}

impl LuaUiContext {
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
}

pub struct LuaUiRuntime {
    engine: lua::LuaEngine,
    context: Box<LuaUiContext>,
}

impl LuaUiRuntime {
    pub fn new() -> Result<Self, String> {
        let mut engine = lua::LuaEngine::new();
        engine.register_log_function();

        let mut context = Box::new(LuaUiContext::new());
        unsafe {
            register_ui_api(engine.state(), &mut *context)?;
        }

        Ok(Self { engine, context })
    }

    pub fn new_empty() -> Self {
        let mut engine = lua::LuaEngine::new();
        engine.register_log_function();
        let context = Box::new(LuaUiContext::new());
        Self { engine, context }
    }

    pub fn dom(&self) -> &dom::Dom {
        &self.context.dom
    }

    pub fn dom_mut(&mut self) -> &mut dom::Dom {
        &mut self.context.dom
    }

    pub fn dom_and_style_mut(&mut self) -> (&mut dom::Dom, &mut style::StyleComputer) {
        let context = &mut *self.context;
        (&mut context.dom, &mut context.style)
    }

    pub fn root(&self) -> dom::NodeId {
        self.context.root
    }

    pub fn style_mut(&mut self) -> &mut style::StyleComputer {
        &mut self.context.style
    }

    pub fn dispatch_click(&mut self, node_id: dom::NodeId) -> bool {
        let Some(callback) = self.context.click_handlers.get(&node_id).copied() else {
            return false;
        };

        unsafe {
            let state = self.engine.state();
            lua::ffi::lua_rawgeti(state, lua::ffi::LUA_REGISTRYINDEX, callback as i64);
            push_node_handle(state, node_id);
            if lua::ffi::lua_pcall(state, 1, 0, 0) != 0 {
                let err = get_lua_error(state);
                crate::log::warning!("Lua on_click failed: {err}");
                return false;
            }
        }

        true
    }
}

pub fn load_runtime_from_file(path: impl AsRef<path::Path>) -> Result<LuaUiRuntime, String> {
    let mut runtime = LuaUiRuntime::new()?;
    runtime.engine.execute_file(path)?;
    Ok(runtime)
}

pub fn load_runtime_from_string(code: &str) -> Result<LuaUiRuntime, String> {
    let mut runtime = LuaUiRuntime::new()?;
    runtime.engine.execute(code)?;
    Ok(runtime)
}

struct StyleBundle {
    base: Option<dom::StyleOverride>,
    hover: Option<dom::StyleOverride>,
    press: Option<dom::StyleOverride>,
    focus: Option<dom::StyleOverride>,
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

unsafe fn register_ui_api(
    state: *mut lua::ffi::lua_State,
    context: &mut LuaUiContext,
) -> Result<(), String> {
    set_context(state, context as *mut LuaUiContext);

    lua::ffi::lua_createtable(state, 0, 4);

    set_field_function(state, "div", ui_div);
    set_field_function(state, "text", ui_text);
    set_field_function(state, "dom", ui_dom);
    set_field_function(state, "set_text", ui_set_text);
    set_field_function(state, "get_default_font", ui_get_default_font);

    let name = ffi::CString::new("ui").map_err(|_| "Invalid UI table name".to_string())?;
    lua::ffi::lua_setfield(state, lua::ffi::LUA_GLOBALSINDEX, name.as_ptr());

    execute_lua_setup(state)?;
    Ok(())
}

unsafe fn execute_lua_setup(state: *mut lua::ffi::lua_State) -> Result<(), String> {
    let api_code = include_bytes!("api.lua");
    let api_str =
        std::str::from_utf8(api_code).map_err(|_| "API code is not valid UTF-8".to_string())?;

    let c_code =
        ffi::CString::new(api_str).map_err(|_| "API code contains null byte".to_string())?;

    if lua::ffi::luaL_loadstring(state, c_code.as_ptr()) != 0 {
        let err = get_lua_error(state);
        return Err(err);
    }

    if lua::ffi::lua_pcall(state, 0, 0, 0) != 0 {
        let err = get_lua_error(state);
        return Err(err);
    }

    Ok(())
}

unsafe fn get_lua_error(state: *mut lua::ffi::lua_State) -> String {
    let err_ptr = lua::ffi::lua_tolstring(state, -1, std::ptr::null_mut());
    let err = ffi::CStr::from_ptr(err_ptr).to_string_lossy().into_owned();
    lua::ffi::lua_pop(state, 1);
    err
}

unsafe fn clear_context(state: *mut lua::ffi::lua_State) {
    lua::ffi::lua_pushnil(state);
    let key = context_key();
    lua::ffi::lua_setfield(state, lua::ffi::LUA_REGISTRYINDEX, key.as_ptr());
}

unsafe fn set_context(state: *mut lua::ffi::lua_State, context: *mut LuaUiContext) {
    let key = context_key();
    lua::ffi::lua_pushlightuserdata(state, context as *mut raw::c_void);
    lua::ffi::lua_setfield(state, lua::ffi::LUA_REGISTRYINDEX, key.as_ptr());
}

unsafe fn context_key() -> &'static ffi::CStr {
    ffi::CStr::from_bytes_with_nul_unchecked(CONTEXT_KEY)
}

unsafe fn set_field_function(
    state: *mut lua::ffi::lua_State,
    name: &str,
    func: lua::ffi::lua_CFunction,
) {
    if let Ok(c_name) = ffi::CString::new(name) {
        lua::ffi::lua_pushcfunction(state, func);
        lua::ffi::lua_setfield(state, -2, c_name.as_ptr());
    }
}

unsafe fn get_context(state: *mut lua::ffi::lua_State) -> Option<&'static mut LuaUiContext> {
    let key = context_key();
    lua::ffi::lua_getfield(state, lua::ffi::LUA_REGISTRYINDEX, key.as_ptr());
    let ptr = lua::ffi::lua_touserdata(state, -1) as *mut LuaUiContext;
    lua::ffi::lua_pop(state, 1);
    if ptr.is_null() { None } else { Some(&mut *ptr) }
}

extern "C" fn ui_div(state: *mut lua::ffi::lua_State) -> raw::c_int {
    unsafe {
        let Some(context) = get_context(state) else {
            return 0;
        };

        let id = context.dom.create_div();
        let options_index = 1;
        if lua::ffi::lua_gettop(state) >= options_index
            && lua::ffi::lua_type(state, options_index) == lua::ffi::LUA_TTABLE
        {
            apply_node_options(state, options_index, id, context);
        }

        push_node_handle(state, id);
        1
    }
}

extern "C" fn ui_text(state: *mut lua::ffi::lua_State) -> raw::c_int {
    unsafe {
        let Some(context) = get_context(state) else {
            return 0;
        };

        let options_index = 1;
        let mut content = String::new();
        let mut font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;
        let mut scale = 1.0f32;
        let mut style_override = None;
        let mut style_rules = StyleBundle::empty();
        let mut has_explicit_size = false;

        if lua::ffi::lua_gettop(state) >= options_index
            && lua::ffi::lua_type(state, options_index) == lua::ffi::LUA_TTABLE
        {
            if let Some(text) = lua_field_string(state, options_index, "text") {
                content = text;
            }

            if let Some(font_value) = lua_field_integer(state, options_index, "font_id") {
                font_id = font_value as u32;
            }

            if let Some(scale_value) = lua_field_number(state, options_index, "font_scale") {
                scale = scale_value;
            }

            if let Some(style_table_index) = lua_field_table(state, options_index, "style") {
                style_rules = parse_style_bundle(state, style_table_index);
                style_override = style_rules.base;
                if let Some(ref override_style) = style_override {
                    has_explicit_size = override_style.size.is_some();
                }
                lua::ffi::lua_pop(state, 1);
            }
        }

        let id = context.dom.create_text(content, font_id, scale);
        if let Some(node) = context.dom.node_mut(id) {
            if let Some(text) = node.as_text_mut() {
                if let Some(override_style) = style_override.as_ref() {
                    text.style.apply_override(override_style);
                }
                if !has_explicit_size {
                    text.style.size = dom::Size2::auto();
                }
            }
        }

        if style_rules.has_rules() {
            let mut rules = style::StyleRules::new(dom::StyleOverride::default());
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

        if lua::ffi::lua_gettop(state) >= options_index
            && lua::ffi::lua_type(state, options_index) == lua::ffi::LUA_TTABLE
        {
            if let Some(on_click_ref) = lua_field_function_ref(state, options_index, "on_click") {
                context.click_handlers.insert(id, on_click_ref);
            }
            apply_children(state, options_index, id, context);
        }

        push_node_handle(state, id);
        1
    }
}

extern "C" fn ui_set_text(state: *mut lua::ffi::lua_State) -> raw::c_int {
    unsafe {
        let Some(context) = get_context(state) else {
            return 0;
        };

        if lua::ffi::lua_gettop(state) < 2 {
            return 0;
        }

        let Some(node_id) = lua_node_id(state, 1) else {
            return 0;
        };

        let Some(text) = lua_string(state, 2) else {
            return 0;
        };

        context.dom.set_text(node_id, text);
        0
    }
}

extern "C" fn ui_dom(state: *mut lua::ffi::lua_State) -> raw::c_int {
    unsafe {
        let Some(context) = get_context(state) else {
            return 0;
        };

        if lua::ffi::lua_gettop(state) < 1 {
            return 0;
        }

        if let Some(node_id) = lua_node_id(state, 1) {
            context.dom.append_child(context.root, node_id);
        }

        0
    }
}

extern "C" fn ui_get_default_font(state: *mut lua::ffi::lua_State) -> raw::c_int {
    unsafe {
        lua::ffi::lua_pushinteger(state, gfx::fonts::CASKAYDIAMONO_FONT_ID as i64);
        1
    }
}

unsafe fn apply_node_options(
    state: *mut lua::ffi::lua_State,
    options_index: raw::c_int,
    node_id: dom::NodeId,
    context: &mut LuaUiContext,
) {
    let mut style_rules = StyleBundle::empty();
    let mut style_override = None;

    if let Some(style_table_index) = lua_field_table(state, options_index, "style") {
        style_rules = parse_style_bundle(state, style_table_index);
        style_override = style_rules.base;
        lua::ffi::lua_pop(state, 1);
    }

    if let Some(node) = context.dom.node_mut(node_id) {
        if let Some(style_ref) = node.style_mut() {
            if let Some(override_style) = style_override.as_ref() {
                style_ref.apply_override(override_style);
            }
        }
    }

    if style_rules.has_rules() {
        let mut rules = style::StyleRules::new(dom::StyleOverride::default());
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

    if let Some(on_click_ref) = lua_field_function_ref(state, options_index, "on_click") {
        context.click_handlers.insert(node_id, on_click_ref);
    }

    apply_children(state, options_index, node_id, context);
}

unsafe fn apply_children(
    state: *mut lua::ffi::lua_State,
    options_index: raw::c_int,
    node_id: dom::NodeId,
    context: &mut LuaUiContext,
) {
    let Some(children_index) = lua_field_table(state, options_index, "children") else {
        return;
    };

    let len = lua::ffi::lua_objlen(state, children_index) as i64;
    for idx in 1..=len {
        lua::ffi::lua_rawgeti(state, children_index, idx);
        if let Some(child_id) = lua_node_id(state, -1) {
            context.dom.append_child(node_id, child_id);
        }
        lua::ffi::lua_pop(state, 1);
    }

    lua::ffi::lua_pop(state, 1);
}

unsafe fn push_node_handle(state: *mut lua::ffi::lua_State, node_id: dom::NodeId) {
    lua::ffi::lua_createtable(state, 0, 1);
    lua::ffi::lua_pushinteger(state, node_id as i64);
    if let Ok(name) = ffi::CString::new(NODE_ID_FIELD) {
        lua::ffi::lua_setfield(state, -2, name.as_ptr());
    } else {
        lua::ffi::lua_pop(state, 1);
        lua::ffi::lua_pushinteger(state, node_id as i64);
    }
}

unsafe fn lua_node_id(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> Option<dom::NodeId> {
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

    if let Ok(name) = ffi::CString::new(NODE_ID_FIELD) {
        lua::ffi::lua_getfield(state, idx, name.as_ptr());
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
        return id_value;
    }

    None
}

unsafe fn lua_field_table(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<raw::c_int> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    if lua::ffi::lua_type(state, -1) == lua::ffi::LUA_TTABLE {
        Some(lua_abs_index(state, -1))
    } else {
        lua::ffi::lua_pop(state, 1);
        None
    }
}

unsafe fn lua_field_string(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<String> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = lua_string(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn lua_field_number(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<f32> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = lua_number(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn lua_field_integer(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<i64> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = lua_integer(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn lua_field_function_ref(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<i32> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };

    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    if lua::ffi::lua_type(state, -1) == lua::ffi::LUA_TFUNCTION {
        let reference = lua::ffi::luaL_ref(state, lua::ffi::LUA_REGISTRYINDEX);
        Some(reference)
    } else {
        lua::ffi::lua_pop(state, 1);
        None
    }
}

unsafe fn lua_string(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> Option<String> {
    if lua::ffi::lua_type(state, idx) != lua::ffi::LUA_TSTRING {
        return None;
    }
    let mut len = 0usize;
    let ptr = lua::ffi::lua_tolstring(state, idx, &mut len);
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    Some(String::from_utf8_lossy(bytes).into_owned())
}

unsafe fn lua_number(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> Option<f32> {
    if lua::ffi::lua_type(state, idx) != lua::ffi::LUA_TNUMBER {
        return None;
    }
    Some(lua::ffi::lua_tonumber(state, idx) as f32)
}

unsafe fn lua_integer(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> Option<i64> {
    if lua::ffi::lua_type(state, idx) != lua::ffi::LUA_TNUMBER {
        return None;
    }
    Some(lua::ffi::lua_tointeger(state, idx) as i64)
}

unsafe fn lua_abs_index(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> raw::c_int {
    if idx > 0 || idx <= lua::ffi::LUA_REGISTRYINDEX {
        idx
    } else {
        lua::ffi::lua_gettop(state) + idx + 1
    }
}

unsafe fn parse_style_bundle(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> StyleBundle {
    if lua::ffi::lua_type(state, table_index) != lua::ffi::LUA_TTABLE {
        return StyleBundle::empty();
    }

    let index = lua_abs_index(state, table_index);
    let mut bundle = StyleBundle::empty();

    if let Some(normal_index) = lua_field_table(state, index, "normal") {
        bundle.base = parse_style_override(state, normal_index);
        lua::ffi::lua_pop(state, 1);
    } else if let Some(base_index) = lua_field_table(state, index, "base") {
        bundle.base = parse_style_override(state, base_index);
        lua::ffi::lua_pop(state, 1);
    } else {
        bundle.base = parse_style_override(state, index);
    }

    if let Some(hover_index) = lua_field_table(state, index, "hover") {
        bundle.hover = parse_style_override(state, hover_index);
        lua::ffi::lua_pop(state, 1);
    }

    if let Some(press_index) = lua_field_table(state, index, "press") {
        bundle.press = parse_style_override(state, press_index);
        lua::ffi::lua_pop(state, 1);
    }

    if let Some(focus_index) = lua_field_table(state, index, "focus") {
        bundle.focus = parse_style_override(state, focus_index);
        lua::ffi::lua_pop(state, 1);
    }

    bundle
}

unsafe fn parse_style_override(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::StyleOverride> {
    if lua::ffi::lua_type(state, table_index) != lua::ffi::LUA_TTABLE {
        return None;
    }

    let index = lua_abs_index(state, table_index);
    let mut override_style = dom::StyleOverride::default();
    let mut any = false;

    if let Some(layout) = parse_layout_style(state, index) {
        override_style.layout = Some(layout);
        any = true;
    }

    if let Some(size) = parse_size2_field(state, index, "size") {
        override_style.size = Some(size);
        any = true;
    } else if let Some(size) = parse_size2_from_dimensions(state, index, "width", "height") {
        override_style.size = Some(size);
        any = true;
    }

    if let Some(size) = parse_size2_field(state, index, "min_size") {
        override_style.min_size = Some(size);
        any = true;
    } else if let Some(size) = parse_size2_from_dimensions(state, index, "min_width", "min_height")
    {
        override_style.min_size = Some(size);
        any = true;
    }

    if let Some(size) = parse_size2_field(state, index, "max_size") {
        override_style.max_size = Some(size);
        any = true;
    } else if let Some(size) = parse_size2_from_dimensions(state, index, "max_width", "max_height")
    {
        override_style.max_size = Some(size);
        any = true;
    }

    if let Some(position) = parse_position_style(state, index) {
        override_style.position = Some(position);
        any = true;
    }

    if let Some(padding) = parse_edge_sizes_field(state, index, "padding") {
        override_style.padding = Some(padding);
        any = true;
    }

    if let Some(margin) = parse_edge_sizes_field(state, index, "margin") {
        override_style.margin = Some(margin);
        any = true;
    }

    if let Some(border) = parse_edge_sizes_field(state, index, "border") {
        override_style.border = Some(border);
        any = true;
    }

    if let Some(color) = parse_color_field(state, index, "foreground")
        .or_else(|| parse_color_field(state, index, "text_color"))
    {
        override_style.foreground = Some(color);
        any = true;
    }

    if let Some(color) = parse_color_field(state, index, "background")
        .or_else(|| parse_color_field(state, index, "background_color"))
    {
        override_style.background = Some(color);
        any = true;
    }

    if let Some(color) = parse_color_field(state, index, "border_color") {
        override_style.border_color = Some(color);
        any = true;
    }

    if any { Some(override_style) } else { None }
}

unsafe fn parse_layout_style(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::LayoutStyle> {
    let mut layout = dom::LayoutStyle::default();
    let mut any = false;

    if let Some(layout_index) = lua_field_table(state, table_index, "layout") {
        if let Some(parsed) = parse_layout_style_table(state, layout_index) {
            layout = parsed;
            any = true;
        }
        lua::ffi::lua_pop(state, 1);
    } else if let Some(parsed) = parse_layout_style_table(state, table_index) {
        layout = parsed;
        any = true;
    }

    if any { Some(layout) } else { None }
}

unsafe fn parse_layout_style_table(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::LayoutStyle> {
    let mut layout = dom::LayoutStyle::default();
    let mut any = false;

    if let Some(direction) = lua_field_string(state, table_index, "direction") {
        if let Some(parsed) = parse_layout_direction(&direction) {
            layout.direction = parsed;
            any = true;
        }
    }

    if let Some(align) = lua_field_string(state, table_index, "align_items")
        .or_else(|| lua_field_string(state, table_index, "align"))
    {
        if let Some(parsed) = parse_align_items(&align) {
            layout.align_items = parsed;
            any = true;
        }
    }

    if let Some(justify) = lua_field_string(state, table_index, "justify_content")
        .or_else(|| lua_field_string(state, table_index, "justify"))
    {
        if let Some(parsed) = parse_justify_content(&justify) {
            layout.justify_content = parsed;
            any = true;
        }
    }

    if let Some(gap) = lua_field_number(state, table_index, "gap") {
        layout.gap = gap.max(0.0);
        any = true;
    }

    if any { Some(layout) } else { None }
}

fn parse_layout_direction(value: &str) -> Option<dom::LayoutDirection> {
    match value.to_ascii_lowercase().as_str() {
        "row" => Some(dom::LayoutDirection::Row),
        "column" | "col" => Some(dom::LayoutDirection::Column),
        _ => None,
    }
}

fn parse_align_items(value: &str) -> Option<dom::AlignItems> {
    match value.to_ascii_lowercase().as_str() {
        "start" => Some(dom::AlignItems::Start),
        "center" => Some(dom::AlignItems::Center),
        "end" => Some(dom::AlignItems::End),
        "stretch" => Some(dom::AlignItems::Stretch),
        _ => None,
    }
}

fn parse_justify_content(value: &str) -> Option<dom::JustifyContent> {
    match value.to_ascii_lowercase().as_str() {
        "start" => Some(dom::JustifyContent::Start),
        "center" => Some(dom::JustifyContent::Center),
        "end" => Some(dom::JustifyContent::End),
        "space-between" | "space_between" => Some(dom::JustifyContent::SpaceBetween),
        "space-around" | "space_around" => Some(dom::JustifyContent::SpaceAround),
        "space-evenly" | "space_evenly" => Some(dom::JustifyContent::SpaceEvenly),
        _ => None,
    }
}

unsafe fn parse_size2_field(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<dom::Size2> {
    let Some(size_index) = lua_field_table(state, table_index, name) else {
        return None;
    };
    let result = parse_size2_table(state, size_index);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn parse_size2_from_dimensions(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    width_name: &str,
    height_name: &str,
) -> Option<dom::Size2> {
    let width = parse_length_field(state, table_index, width_name);
    let height = parse_length_field(state, table_index, height_name);
    if width.is_none() && height.is_none() {
        return None;
    }

    Some(dom::Size2 {
        width: width.unwrap_or(dom::Length::Auto),
        height: height.unwrap_or(dom::Length::Auto),
    })
}

unsafe fn parse_size2_table(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::Size2> {
    let width = parse_length_field(state, table_index, "width")
        .or_else(|| parse_length_field(state, table_index, "w"));
    let height = parse_length_field(state, table_index, "height")
        .or_else(|| parse_length_field(state, table_index, "h"));

    if width.is_none() && height.is_none() {
        return None;
    }

    Some(dom::Size2 {
        width: width.unwrap_or(dom::Length::Auto),
        height: height.unwrap_or(dom::Length::Auto),
    })
}

unsafe fn parse_length_field(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<dom::Length> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = parse_length_value(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn parse_length_value(
    state: *mut lua::ffi::lua_State,
    idx: raw::c_int,
) -> Option<dom::Length> {
    match lua::ffi::lua_type(state, idx) {
        lua::ffi::LUA_TNUMBER => {
            let value = lua::ffi::lua_tonumber(state, idx) as f32;
            Some(dom::Length::Px(value.max(0.0)))
        }
        lua::ffi::LUA_TSTRING => {
            let value = lua_string(state, idx)?;
            parse_length_string(&value)
        }
        _ => None,
    }
}

fn parse_length_string(value: &str) -> Option<dom::Length> {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower == "auto" {
        return Some(dom::Length::Auto);
    }
    if lower == "fill" {
        return Some(dom::Length::Fill);
    }
    if let Some(percent) = lower.strip_suffix('%') {
        if let Ok(value) = percent.trim().parse::<f32>() {
            return Some(dom::Length::Percent((value / 100.0).max(0.0)));
        }
    }
    if let Some(px) = lower.strip_suffix("px") {
        if let Ok(value) = px.trim().parse::<f32>() {
            return Some(dom::Length::Px(value.max(0.0)));
        }
    }
    if let Ok(value) = lower.parse::<f32>() {
        return Some(dom::Length::Px(value.max(0.0)));
    }
    None
}

unsafe fn parse_edge_sizes_field(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<dom::EdgeSizes> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = parse_edge_sizes_value(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn parse_edge_sizes_value(
    state: *mut lua::ffi::lua_State,
    idx: raw::c_int,
) -> Option<dom::EdgeSizes> {
    match lua::ffi::lua_type(state, idx) {
        lua::ffi::LUA_TNUMBER => {
            let value = lua::ffi::lua_tonumber(state, idx) as f32;
            Some(dom::EdgeSizes {
                left: value,
                right: value,
                top: value,
                bottom: value,
            })
        }
        lua::ffi::LUA_TTABLE => {
            let index = lua_abs_index(state, idx);
            let mut edge = dom::EdgeSizes::zero();
            let mut any = false;

            if let Some(value) = lua_field_number(state, index, "left") {
                edge.left = value;
                any = true;
            }
            if let Some(value) = lua_field_number(state, index, "right") {
                edge.right = value;
                any = true;
            }
            if let Some(value) = lua_field_number(state, index, "top") {
                edge.top = value;
                any = true;
            }
            if let Some(value) = lua_field_number(state, index, "bottom") {
                edge.bottom = value;
                any = true;
            }

            if let Some(value) = lua_field_number(state, index, "x") {
                edge.left = value;
                edge.right = value;
                any = true;
            }
            if let Some(value) = lua_field_number(state, index, "y") {
                edge.top = value;
                edge.bottom = value;
                any = true;
            }

            if any { Some(edge) } else { None }
        }
        _ => None,
    }
}

unsafe fn parse_color_field(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<Color> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = parse_color_value(state, -1);
    lua::ffi::lua_pop(state, 1);
    result
}

unsafe fn parse_color_value(state: *mut lua::ffi::lua_State, idx: raw::c_int) -> Option<Color> {
    match lua::ffi::lua_type(state, idx) {
        lua::ffi::LUA_TNUMBER => {
            let value = lua::ffi::lua_tonumber(state, idx) as u32;
            Some(Color::from_u32(value))
        }
        lua::ffi::LUA_TSTRING => {
            let value = lua_string(state, idx)?;
            parse_color_string(&value)
        }
        lua::ffi::LUA_TTABLE => {
            let index = lua_abs_index(state, idx);
            let r = lua_field_number(state, index, "r");
            let g = lua_field_number(state, index, "g");
            let b = lua_field_number(state, index, "b");
            let a = lua_field_number(state, index, "a").unwrap_or(1.0);
            if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                let color = Color::new(r, g, b, a);
                Some(color)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_color_string(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).ok().map(Color::from_u32);
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "white" => Some(Color::from_u32(0xFFFF_FFFF)),
        "black" => Some(Color::from_u32(0x0000_00FF)),
        "gray" | "grey" => Some(Color::from_u32(0x8080_80FF)),
        "lightgray" | "lightgrey" => Some(Color::from_u32(0xD3D3_D3FF)),
        "blue" => Some(Color::from_u32(0x0000_FFFF)),
        "lightblue" => Some(Color::from_u32(0x87CE_FAFF)),
        "darkblue" => Some(Color::from_u32(0x0000_8BFF)),
        "yellow" => Some(Color::from_u32(0xFFFF_00FF)),
        "red" => Some(Color::from_u32(0xFF00_00FF)),
        "green" => Some(Color::from_u32(0x00FF_00FF)),
        "transparent" => Some(Color::transparent()),
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let value = match hex.len() {
        6 => {
            let rgb = u32::from_str_radix(hex, 16).ok()?;
            (rgb << 8) | 0xFF
        }
        8 => u32::from_str_radix(hex, 16).ok()?,
        _ => return None,
    };
    Some(Color::from_u32(value))
}

unsafe fn parse_position_style(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::PositionStyle> {
    let mut any = false;
    let mut position = dom::PositionStyle::default();

    if let Some(position_index) = lua_field_table(state, table_index, "position") {
        if let Some(parsed) = parse_position_style_table(state, position_index) {
            position = parsed;
            any = true;
        }
        lua::ffi::lua_pop(state, 1);
    } else if let Some(parsed) = parse_position_style_table(state, table_index) {
        position = parsed;
        any = true;
    }

    if any { Some(position) } else { None }
}

unsafe fn parse_position_style_table(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::PositionStyle> {
    let mut position = dom::PositionStyle::default();
    let mut any = false;

    if let Some(mode) = lua_field_string(state, table_index, "mode") {
        if let Some(parsed) = parse_position_mode(&mode) {
            position.mode = parsed;
            any = true;
        }
    }

    if let Some(anchors) = parse_anchors(state, table_index) {
        position.anchors = anchors;
        any = true;
    }

    if any { Some(position) } else { None }
}

fn parse_position_mode(value: &str) -> Option<dom::PositionMode> {
    match value.to_ascii_lowercase().as_str() {
        "flow" => Some(dom::PositionMode::Flow),
        "absolute" => Some(dom::PositionMode::Absolute),
        _ => None,
    }
}

unsafe fn parse_anchors(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
) -> Option<dom::Anchors> {
    if let Some(anchor_value) = lua_field_string(state, table_index, "anchors") {
        return match anchor_value.to_ascii_lowercase().as_str() {
            "fill" => Some(dom::Anchors::fill()),
            "horizontal" => Some(dom::Anchors::horizontal()),
            "vertical" => Some(dom::Anchors::vertical()),
            "none" => Some(dom::Anchors::none()),
            _ => None,
        };
    }

    if let Some(anchors_index) = lua_field_table(state, table_index, "anchors") {
        let index = lua_abs_index(state, anchors_index);
        let mut anchors = dom::Anchors::none();
        let mut any = false;

        if let Some(value) = lua_field_bool(state, index, "left") {
            anchors.left = value;
            any = true;
        }
        if let Some(value) = lua_field_bool(state, index, "right") {
            anchors.right = value;
            any = true;
        }
        if let Some(value) = lua_field_bool(state, index, "top") {
            anchors.top = value;
            any = true;
        }
        if let Some(value) = lua_field_bool(state, index, "bottom") {
            anchors.bottom = value;
            any = true;
        }

        lua::ffi::lua_pop(state, 1);
        if any {
            return Some(anchors);
        }
    }

    None
}

unsafe fn lua_field_bool(
    state: *mut lua::ffi::lua_State,
    table_index: raw::c_int,
    name: &str,
) -> Option<bool> {
    let index = lua_abs_index(state, table_index);
    let Ok(c_name) = ffi::CString::new(name) else {
        return None;
    };
    lua::ffi::lua_getfield(state, index, c_name.as_ptr());
    let result = match lua::ffi::lua_type(state, -1) {
        lua::ffi::LUA_TBOOLEAN => Some(lua::ffi::lua_toboolean(state, -1) != 0),
        _ => None,
    };
    lua::ffi::lua_pop(state, 1);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_simple_div_creation() {
        let code = r###"
local div = ui.div({
    style = { background = "#FF0000FF" },
})
ui.dom(div)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1, "Root should have 1 child");

        let child_id = root.children[0];
        let child = lua_dom
            .dom
            .node(child_id)
            .expect("Child should exist");
        assert!(
            child.as_div().is_some(),
            "Child should be a div, got {:?}",
            child.kind
        );
    }

    #[test]
    fn lua_text_node_creation() {
        let code = r###"
local text = ui.text({
    text = "Hello World",
    font_scale = 1.5,
})
ui.dom(text)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1, "Root should have 1 child");

        let child_id = root.children[0];
        let child = lua_dom
            .dom
            .node(child_id)
            .expect("Child should exist");
        let text_node = child.as_text().expect("Should be a text node");
        assert_eq!(text_node.content, "Hello World");
        assert_eq!(text_node.scale, 1.5);
    }

    #[test]
    fn lua_nested_children() {
        let code = r###"
local parent = ui.div({
    children = {
        ui.text({ text = "Child 1" }),
        ui.text({ text = "Child 2" }),
        ui.div({ children = { ui.text({ text = "Grandchild" }) } }),
    },
})
ui.dom(parent)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 1);

        let parent_id = root.children[0];
        let parent = lua_dom
            .dom
            .node(parent_id)
            .expect("Parent should exist");
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
        let code = r###"
local div = ui.div({
    style = {
        background = "#AABBCCFF",
        padding = 10,
        margin = { left = 5, right = 5, top = 2, bottom = 2 },
    },
})
ui.dom(div)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom
            .dom
            .node(child_id)
            .expect("Child should exist");
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
    fn lua_compose_helper() {
        let code = r###"
local base = { background = "#FF0000FF", padding = 10 }
local override = ui.compose(base, { background = "#00FF00FF" })

local div = ui.div({ style = override })
ui.dom(div)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom
            .dom
            .node(child_id)
            .expect("Child should exist");
        let style = child.style().expect("Should have style");

        assert_eq!(style.background, crate::Color::from_u32(0x00FF00FF), "Override should win");
        assert_eq!(style.padding.left, 10.0, "Base padding should persist");
    }

    #[test]
    fn lua_label_helper() {
        let code = r###"
local label = ui.label("Test Label", { scale = 2.0 })
ui.dom(label)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        let child_id = root.children[0];
        let child = lua_dom
            .dom
            .node(child_id)
            .expect("Child should exist");
        let text = child.as_text().expect("Should be text");
        assert_eq!(text.content, "Test Label");
        assert_eq!(text.scale, 2.0);
    }

    #[test]
    fn lua_error_handling() {
        let code = r#"
local invalid = ui.div_does_not_exist({
    children = {}
})
"#;
        let result = load_dom_from_string(code);
        assert!(result.is_err(), "Should fail with undefined function");
    }

    #[test]
    fn lua_color_parsing() {
        let code = r###"
local div1 = ui.div({ style = { background = "#FF0000FF" } })
local div2 = ui.div({ style = { background = "red" } })
local div3 = ui.div({ style = { background = 0xFF0000FF } })
ui.dom(div1)
ui.dom(div2)
ui.dom(div3)
"###;
        let lua_dom = load_dom_from_string(code).expect("Failed to load DOM");

        let root = lua_dom.dom.node(lua_dom.root).expect("Root should exist");
        assert_eq!(root.children.len(), 3, "Should have 3 children");

        for i in 0..3 {
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
