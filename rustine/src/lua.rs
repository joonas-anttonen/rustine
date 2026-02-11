#![allow(dead_code)]

// lua_engine.rs - Minimal safe wrapper
use std::ffi::{CStr, CString};
use std::os::raw::c_int;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Number(f64),
    String(String),
    Boolean(bool),
}

pub struct LuaEngine {
    state: *mut ffi::lua_State,
}

impl LuaEngine {
    pub fn new() -> Self {
        unsafe {
            let state = ffi::luaL_newstate();
            ffi::luaL_openlibs(state); // Load standard libraries
            Self { state }
        }
    }

    pub fn execute(&mut self, script: &str) -> Result<(), String> {
        unsafe {
            let c_script = CString::new(script).unwrap();

            if ffi::luaL_loadstring(self.state, c_script.as_ptr()) != 0 {
                return Err(self.get_error());
            }

            if ffi::lua_pcall(self.state, 0, 0, 0) != 0 {
                return Err(self.get_error());
            }

            Ok(())
        }
    }

    pub fn get_global_number(&mut self, name: &str) -> Option<f64> {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_getfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());

            if ffi::lua_type(self.state, -1) == ffi::LUA_TNUMBER {
                let value = ffi::lua_tonumber(self.state, -1);
                ffi::lua_pop(self.state, 1);
                Some(value)
            } else {
                ffi::lua_pop(self.state, 1);
                None
            }
        }
    }

    pub fn set_global_number(&mut self, name: &str, value: f64) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_pushnumber(self.state, value);
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn get_global_string(&mut self, name: &str) -> Option<String> {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_getfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());

            if ffi::lua_type(self.state, -1) == ffi::LUA_TSTRING {
                let value_ptr = ffi::lua_tolstring(self.state, -1, std::ptr::null_mut());
                let value = CStr::from_ptr(value_ptr).to_string_lossy().into_owned();
                ffi::lua_pop(self.state, 1);
                Some(value)
            } else {
                ffi::lua_pop(self.state, 1);
                None
            }
        }
    }

    pub fn set_global_string(&mut self, name: &str, value: &str) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let c_value = CString::new(value).unwrap();
            ffi::lua_pushstring(self.state, c_value.as_ptr());
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn get_global_bool(&mut self, name: &str) -> Option<bool> {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_getfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());

            if ffi::lua_type(self.state, -1) == ffi::LUA_TBOOLEAN {
                let value = ffi::lua_toboolean(self.state, -1) != 0;
                ffi::lua_pop(self.state, 1);
                Some(value)
            } else {
                ffi::lua_pop(self.state, 1);
                None
            }
        }
    }

    pub fn set_global_bool(&mut self, name: &str, value: bool) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_pushboolean(self.state, if value { 1 } else { 0 });
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn call_function(&mut self, name: &str, args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_getfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());

            if ffi::lua_type(self.state, -1) != ffi::LUA_TFUNCTION {
                ffi::lua_pop(self.state, 1);
                return Err(format!("'{}' is not a function", name));
            }

            for arg in args {
                match arg {
                    LuaValue::Number(n) => ffi::lua_pushnumber(self.state, *n),
                    LuaValue::String(s) => {
                        let c_str = CString::new(s.as_str()).unwrap();
                        ffi::lua_pushstring(self.state, c_str.as_ptr());
                    }
                    LuaValue::Boolean(b) => ffi::lua_pushboolean(self.state, if *b { 1 } else { 0 }),
                    LuaValue::Nil => ffi::lua_pushnil(self.state),
                }
            }

            let nargs = args.len() as c_int;
            let nresults = ffi::LUA_MULTRET;

            let top_before = ffi::lua_gettop(self.state);
            if ffi::lua_pcall(self.state, nargs, nresults, 0) != 0 {
                return Err(self.get_error());
            }
            let top_after = ffi::lua_gettop(self.state);
            let result_count = top_after - top_before + nargs + 1;

            let mut results = Vec::new();
            for i in 0..result_count {
                let idx = -result_count + i;
                let lua_type = ffi::lua_type(self.state, idx);
                match lua_type {
                    ffi::LUA_TNUMBER => {
                        results.push(LuaValue::Number(ffi::lua_tonumber(self.state, idx)));
                    }
                    ffi::LUA_TSTRING => {
                        let value_ptr = ffi::lua_tolstring(self.state, idx, std::ptr::null_mut());
                        let value = CStr::from_ptr(value_ptr).to_string_lossy().into_owned();
                        results.push(LuaValue::String(value));
                    }
                    ffi::LUA_TBOOLEAN => {
                        results.push(LuaValue::Boolean(ffi::lua_toboolean(self.state, idx) != 0));
                    }
                    _ => {
                        results.push(LuaValue::Nil);
                    }
                }
            }

            ffi::lua_pop(self.state, result_count);
            Ok(results)
        }
    }

    pub fn register_function(&mut self, name: &str, func: ffi::lua_CFunction) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            ffi::lua_pushcfunction(self.state, func);
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn register_log_function(&mut self) {
        self.register_function("log", lua_global_log);
    }

    fn get_error(&mut self) -> String {
        unsafe {
            let err_ptr = ffi::lua_tolstring(self.state, -1, std::ptr::null_mut());
            let err = CStr::from_ptr(err_ptr).to_string_lossy().into_owned();
            ffi::lua_pop(self.state, 1);
            err
        }
    }
}

impl Drop for LuaEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::lua_close(self.state);
        }
    }
}

// Example: Register a Rust function callable from Lua
extern "C" fn lua_global_log(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        let msg = ffi::lua_tolstring(l, 1, std::ptr::null_mut());
        let msg_str = CStr::from_ptr(msg).to_string_lossy();
        crate::log::warning!("[Lua]: {}", msg_str);
        0 // Number of return values
    }
}

mod ffi {
    #![allow(non_camel_case_types, non_snake_case)]

    use std::os::raw::{c_char, c_int, c_void};

    pub type lua_State = c_void;
    pub type lua_CFunction = extern "C" fn(*mut lua_State) -> c_int;
    pub type lua_Number = f64;
    pub type lua_Integer = i64;

    // Lua types
    pub const LUA_TNONE: c_int = -1;
    pub const LUA_TNIL: c_int = 0;
    pub const LUA_TBOOLEAN: c_int = 1;
    pub const LUA_TNUMBER: c_int = 3;
    pub const LUA_TSTRING: c_int = 4;
    pub const LUA_TTABLE: c_int = 5;
    pub const LUA_TFUNCTION: c_int = 6;

    // Pseudo-indices
    pub const LUA_REGISTRYINDEX: c_int = -10000;
    pub const LUA_GLOBALSINDEX: c_int = -10002;

    // Special values
    pub const LUA_MULTRET: c_int = -1;

    #[link(name = "luajit")]
    unsafe extern "C" {
        // State manipulation
        pub fn luaL_newstate() -> *mut lua_State;
        pub fn lua_close(L: *mut lua_State);

        // Basic stack manipulation
        pub fn lua_gettop(L: *mut lua_State) -> c_int;
        pub fn lua_settop(L: *mut lua_State, idx: c_int);
        pub fn lua_pushvalue(L: *mut lua_State, idx: c_int);
        pub fn lua_remove(L: *mut lua_State, idx: c_int);
        pub fn lua_type(L: *mut lua_State, idx: c_int) -> c_int;

        // Push functions
        pub fn lua_pushnil(L: *mut lua_State);
        pub fn lua_pushnumber(L: *mut lua_State, n: lua_Number);
        pub fn lua_pushinteger(L: *mut lua_State, n: lua_Integer);
        pub fn lua_pushboolean(L: *mut lua_State, b: c_int);
        pub fn lua_pushstring(L: *mut lua_State, s: *const c_char);
        pub fn lua_pushcclosure(L: *mut lua_State, f: lua_CFunction, n: c_int);

        // Get functions
        pub fn lua_toboolean(L: *mut lua_State, idx: c_int) -> c_int;
        pub fn lua_tonumber(L: *mut lua_State, idx: c_int) -> lua_Number;
        pub fn lua_tointeger(L: *mut lua_State, idx: c_int) -> lua_Integer;
        pub fn lua_tolstring(L: *mut lua_State, idx: c_int, len: *mut usize) -> *const c_char;

        // Table operations
        pub fn lua_createtable(L: *mut lua_State, narr: c_int, nrec: c_int);
        pub fn lua_gettable(L: *mut lua_State, idx: c_int);
        pub fn lua_settable(L: *mut lua_State, idx: c_int);
        pub fn lua_getfield(L: *mut lua_State, idx: c_int, k: *const c_char);
        pub fn lua_setfield(L: *mut lua_State, idx: c_int, k: *const c_char);

        // Load and call
        pub fn luaL_loadstring(L: *mut lua_State, s: *const c_char) -> c_int;
        pub fn lua_pcall(L: *mut lua_State, nargs: c_int, nresults: c_int, errfunc: c_int)
        -> c_int;

        // Auxiliary library
        pub fn luaL_openlibs(L: *mut lua_State);
    }

    #[inline]
    pub unsafe fn lua_pop(L: *mut lua_State, n: c_int) {
        unsafe { lua_settop(L, -n - 1) };
    }

    #[inline]
    pub unsafe fn lua_pushcfunction(L: *mut lua_State, f: lua_CFunction) {
        unsafe { lua_pushcclosure(L, f, 0) };
    }
}
