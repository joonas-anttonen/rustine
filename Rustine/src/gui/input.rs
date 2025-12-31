#![allow(dead_code)]

use crate::gui::glfw_ffi as glfw;

pub struct Key(pub i32);
impl Key {
    pub const ESCAPE: Key = Key(glfw::KEY_ESCAPE);
    pub const ENTER: Key = Key(glfw::KEY_ENTER);
    pub const SPACE: Key = Key(glfw::KEY_SPACE);
    pub const LEFT: Key = Key(glfw::KEY_LEFT);
    pub const RIGHT: Key = Key(glfw::KEY_RIGHT);
    pub const UP: Key = Key(glfw::KEY_UP);
    pub const DOWN: Key = Key(glfw::KEY_DOWN);
    pub const LEFT_SHIFT: Key = Key(glfw::KEY_LEFT_SHIFT);
    pub const RIGHT_SHIFT: Key = Key(glfw::KEY_RIGHT_SHIFT);
    pub const A: Key = Key(glfw::KEY_A);
    pub const B: Key = Key(glfw::KEY_B);
    pub const C: Key = Key(glfw::KEY_C);
    pub const D: Key = Key(glfw::KEY_D);
    pub const E: Key = Key(glfw::KEY_E);
    pub const F: Key = Key(glfw::KEY_F);
    pub const G: Key = Key(glfw::KEY_G);
    pub const H: Key = Key(glfw::KEY_H);
    pub const I: Key = Key(glfw::KEY_I);
    pub const J: Key = Key(glfw::KEY_J);
    pub const K: Key = Key(glfw::KEY_K);
    pub const L: Key = Key(glfw::KEY_L);
    pub const M: Key = Key(glfw::KEY_M);
    pub const N: Key = Key(glfw::KEY_N);
    pub const O: Key = Key(glfw::KEY_O);
    pub const P: Key = Key(glfw::KEY_P);
    pub const Q: Key = Key(glfw::KEY_Q);
    pub const R: Key = Key(glfw::KEY_R);
    pub const S: Key = Key(glfw::KEY_S);
    pub const T: Key = Key(glfw::KEY_T);
    pub const U: Key = Key(glfw::KEY_U);
    pub const V: Key = Key(glfw::KEY_V);
    pub const W: Key = Key(glfw::KEY_W);
    pub const X: Key = Key(glfw::KEY_X);
    pub const Y: Key = Key(glfw::KEY_Y);
    pub const Z: Key = Key(glfw::KEY_Z);
    pub const F1: Key = Key(glfw::KEY_F1);
    pub const F2: Key = Key(glfw::KEY_F2);
    pub const F3: Key = Key(glfw::KEY_F3);
    pub const F4: Key = Key(glfw::KEY_F4);
    pub const F5: Key = Key(glfw::KEY_F5);
    pub const F6: Key = Key(glfw::KEY_F6);
    pub const F7: Key = Key(glfw::KEY_F7);
    pub const F8: Key = Key(glfw::KEY_F8);
    pub const F9: Key = Key(glfw::KEY_F9);
    pub const F10: Key = Key(glfw::KEY_F10);
    pub const F11: Key = Key(glfw::KEY_F11);
    pub const F12: Key = Key(glfw::KEY_F12);
    pub const TAB: Key = Key(glfw::KEY_TAB);
    pub const BACKSPACE: Key = Key(glfw::KEY_BACKSPACE);
    pub const DELETE: Key = Key(glfw::KEY_DELETE);
    pub const INSERT: Key = Key(glfw::KEY_INSERT);
    pub const HOME: Key = Key(glfw::KEY_HOME);
    pub const END: Key = Key(glfw::KEY_END);

    pub fn from_code(code: i32) -> Key {
        Key(code)
    }
}

pub struct Action(pub i32);
impl std::ops::BitOr for Action {
    type Output = Action;

    fn bitor(self, rhs: Action) -> Action {
        Action(self.0 | rhs.0)
    }
}
impl Action {
    pub const PRESS: Action = Action(glfw::PRESS);
    pub const RELEASE: Action = Action(glfw::RELEASE);
    pub const REPEAT: Action = Action(glfw::REPEAT);

    pub fn from_code(code: i32) -> Action {
        Action(code)
    }

    pub fn contains(&self, other: Action) -> bool {
        (self.0 & other.0) != 0
    }
}

pub struct Mods(pub i32);
impl std::ops::BitOr for Mods {
    type Output = Mods;

    fn bitor(self, rhs: Mods) -> Mods {
        Mods(self.0 | rhs.0)
    }
}
impl Mods {
    pub const SHIFT: Mods = Mods(glfw::MOD_SHIFT);
    pub const CONTROL: Mods = Mods(glfw::MOD_CONTROL);
    pub const ALT: Mods = Mods(glfw::MOD_ALT);
    pub const SUPER: Mods = Mods(glfw::MOD_SUPER);

    pub fn from_code(code: i32) -> Mods {
        Mods(code)
    }

    pub fn contains(&self, other: Mods) -> bool {
        (self.0 & other.0) != 0
    }
}

pub struct KeyEvent {
    pub key: Key,
    pub action: Action,
    pub mods: Mods,
}
