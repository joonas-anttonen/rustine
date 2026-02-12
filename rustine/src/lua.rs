#![allow(dead_code)]

use std::ffi::{CStr, CString};

extern "C" fn lua_global_log(l: *mut ffi::lua_State) -> core::ffi::c_int {
    unsafe {
        let msg = ffi::lua_tolstring(l, 1, std::ptr::null_mut());
        let msg_str = CStr::from_ptr(msg).to_string_lossy();
        crate::log::info!("[Lua]: {}", msg_str);
        0 // Number of return values
    }
}

pub struct LuaState {
    state: *mut ffi::lua_State,
}

impl LuaState {
    pub fn new() -> Self {
        unsafe {
            let state = ffi::luaL_newstate();

            // Load standard libraries
            ffi::luaL_openlibs(state);

            // Register global log function
            let c_name = CString::new("log").unwrap();
            ffi::lua_pushcfunction(state, lua_global_log);
            ffi::lua_setfield(state, ffi::LUA_GLOBALSINDEX, c_name.as_ptr());

            Self { state }
        }
    }

    #[inline]
    /// lua_pushlightuserdata
    pub fn pushlightuserdata(&self, data: *mut core::ffi::c_void) {
        unsafe {
            ffi::lua_pushlightuserdata(self.state, data);
        }
    }

    #[inline]
    /// lua_setfield
    pub fn setfield(&self, index: i32, name: &CStr) {
        unsafe {
            ffi::lua_setfield(self.state, index, name.as_ptr());
        }
    }

    #[inline]
    /// lua_pushcfunction
    pub fn pushcfunction(&self, func: ffi::lua_CFunction) {
        unsafe {
            ffi::lua_pushcfunction(self.state, func);
        }
    }

    #[inline]
    /// lua_pushinteger
    pub fn pushinteger(&self, value: i64) {
        unsafe {
            ffi::lua_pushinteger(self.state, value);
        }
    }

    #[inline]
    /// lua_createtable
    ///
    /// `narr` is a hint for how many elements the table will have as a sequence,
    /// `nrec` is a hint for how many other elements the table will have.
    pub fn createtable(&self, narr: i32, nrec: i32) {
        unsafe {
            ffi::lua_createtable(self.state, narr, nrec);
        }
    }

    #[inline]
    /// lua_rawgeti
    pub fn rawgeti(&self, index: i32, value: i64) {
        unsafe {
            ffi::lua_rawgeti(self.state, index, value);
        }
    }

    #[inline]
    /// lua_pop
    pub fn pop(&self, count: i32) {
        unsafe {
            ffi::lua_pop(self.state, count);
        }
    }

    #[inline]
    /// luaL_loadstring
    pub fn load_string(&self, string: &CStr) -> Result<(), String> {
        unsafe {
            if ffi::luaL_loadstring(self.state, string.as_ptr()) != 0 {
                return Err(self.get_error());
            }
            Ok(())
        }
    }

    #[inline]
    /// lua_pcall
    pub fn pcall(&self, nargs: i32, nresults: i32) -> Result<(), String> {
        unsafe {
            if ffi::lua_pcall(self.state, nargs, nresults, 0) != 0 {
                return Err(self.get_error());
            }
            Ok(())
        }
    }

    pub fn execute(&self, script: &str) -> Result<(), String> {
        unsafe {
            let c_script = CString::new(script)
                .map_err(|err| format!("Failed to convert script to C string: {err}"))?;

            if ffi::luaL_loadstring(self.state, c_script.as_ptr()) != 0 {
                return Err(self.get_error());
            }

            if ffi::lua_pcall(self.state, 0, 0, 0) != 0 {
                return Err(self.get_error());
            }

            Ok(())
        }
    }

    pub fn execute_cstr(&self, script: &CStr) -> Result<(), String> {
        unsafe {
            if ffi::luaL_loadstring(self.state, script.as_ptr()) != 0 {
                return Err(self.get_error());
            }

            if ffi::lua_pcall(self.state, 0, 0, 0) != 0 {
                return Err(self.get_error());
            }

            Ok(())
        }
    }

    /// Returns a raw pointer to the Lua state.
    pub(crate) fn as_raw(&self) -> *mut ffi::lua_State {
        self.state
    }

    fn get_error(&self) -> String {
        unsafe {
            let err_ptr = ffi::lua_tolstring(self.state, -1, std::ptr::null_mut());
            let err = CStr::from_ptr(err_ptr).to_string_lossy().into_owned();
            ffi::lua_pop(self.state, 1);
            err
        }
    }
}

impl Drop for LuaState {
    fn drop(&mut self) {
        unsafe {
            ffi::lua_close(self.state);
        }
    }
}

pub(crate) mod ffi {
    #![allow(non_camel_case_types, non_snake_case)]

    use core::ffi::{c_char, c_int, c_void};

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
        pub fn luaL_ref(L: *mut lua_State, t: c_int) -> c_int;
        pub fn luaL_unref(L: *mut lua_State, t: c_int, r: c_int);
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
