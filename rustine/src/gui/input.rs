#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    UNKNOWN,
    SPACE,
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
