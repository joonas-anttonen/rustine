#![allow(dead_code)]

// lua_engine.rs - Minimal safe wrapper
use std::ffi as stdffi;
use std::os::raw;
use std::path::Path;

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
            let c_script = stdffi::CString::new(script).unwrap();

            if ffi::luaL_loadstring(self.state, c_script.as_ptr()) != 0 {
                return Err(self.get_error());
            }

            if ffi::lua_pcall(self.state, 0, 0, 0) != 0 {
                return Err(self.get_error());
            }

            Ok(())
        }
    }

    pub fn execute_file(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let path = path.as_ref();
        let script = std::fs::read_to_string(path)
            .map_err(|err| format!("Failed to read Lua file {}: {err}", path.display()))?;
        self.execute(&script)
    }

    pub fn get_global_number(&mut self, name: &str) -> Option<f64> {
        unsafe {
            let c_name = stdffi::CString::new(name).unwrap();
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
            let c_name = stdffi::CString::new(name).unwrap();
            ffi::lua_pushnumber(self.state, value);
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn register_function(&mut self, name: &str, func: ffi::lua_CFunction) {
        unsafe {
            let c_name = stdffi::CString::new(name).unwrap();
            ffi::lua_pushcfunction(self.state, func);
            ffi::lua_setfield(self.state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());
        }
    }

    pub fn register_log_function(&mut self) {
        self.register_function("log", lua_global_log);
    }

    pub(crate) fn state(&mut self) -> *mut ffi::lua_State {
        self.state
    }

    fn get_error(&mut self) -> String {
        unsafe {
            let err_ptr = ffi::lua_tolstring(self.state, -1, std::ptr::null_mut());
            let err = stdffi::CStr::from_ptr(err_ptr).to_string_lossy().into_owned();
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
extern "C" fn lua_global_log(l: *mut ffi::lua_State) -> raw::c_int {
    unsafe {
        let msg = ffi::lua_tolstring(l, 1, std::ptr::null_mut());
        let msg_str = stdffi::CStr::from_ptr(msg).to_string_lossy();
        crate::log::warning!("[Lua]: {}", msg_str);
        0 // Number of return values
    }
}

pub(crate) mod ffi {
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
        pub fn lua_objlen(L: *mut lua_State, idx: c_int) -> usize;
        pub fn lua_rawgeti(L: *mut lua_State, idx: c_int, n: lua_Integer);
        pub fn lua_next(L: *mut lua_State, idx: c_int) -> c_int;

        // Userdata
        pub fn lua_pushlightuserdata(L: *mut lua_State, p: *mut c_void);
        pub fn lua_touserdata(L: *mut lua_State, idx: c_int) -> *mut c_void;

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
