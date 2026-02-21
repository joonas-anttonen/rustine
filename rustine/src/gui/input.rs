#![allow(dead_code, non_camel_case_types)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    UNKNOWN,
    SPACE,
    ESCAPE,
    ENTER,
    DELETE,
    BACKSPACE,
    TAB,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    UP,
    DOWN,
    LEFT,
    RIGHT,
    PERIOD,
    MINUS,
    PAGE_UP,
    PAGE_DOWN,
    HOME,
    END,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    RELEASE,
    PRESS,
}

pub struct Mods(pub i32);
impl std::ops::BitOr for Mods {
    type Output = Mods;

    fn bitor(self, rhs: Mods) -> Mods {
        Mods(self.0 | rhs.0)
    }
}
impl Mods {
    pub const NONE: Mods = Mods(0);
    pub const SHIFT: Mods = Mods(1 << 0);
    pub const CONTROL: Mods = Mods(1 << 1);
    pub const ALT: Mods = Mods(1 << 2);
    pub const SUPER: Mods = Mods(1 << 3);

    pub fn contains(&self, other: Mods) -> bool {
        (self.0 & other.0) != 0
    }
}

pub struct KeyEvent {
    pub key: Key,
    pub action: Action,
    pub mods: Mods,
    pub scancode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    UNKNOWN,
    LEFT,
    RIGHT,
    MIDDLE,
    BACK,
    FORWARD,
}

pub struct MouseMoveEvent {
    pub position: crate::Vector2f,
}

pub struct MouseEnterEvent {
    pub position: crate::Vector2f,
}

pub struct MouseLeaveEvent {
    pub position: crate::Vector2f,
}

pub struct MouseButtonEvent {
    pub position: crate::Vector2f,
    pub button: MouseButton,
    pub action: Action,
    pub mods: Mods,
}

pub struct MouseScrollEvent {
    pub position: crate::Vector2f,
    pub delta: crate::Vector2f,
    pub delta_discrete: crate::Vector2i,
    pub mods: Mods,
}
