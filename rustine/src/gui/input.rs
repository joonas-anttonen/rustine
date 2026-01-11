#![allow(dead_code)]

pub enum Key {}

pub enum Action {
    Release,
    Press,
}

pub struct Mods(pub i32);
impl std::ops::BitOr for Mods {
    type Output = Mods;

    fn bitor(self, rhs: Mods) -> Mods {
        Mods(self.0 | rhs.0)
    }
}
impl Mods {
    pub const SHIFT: Mods = Mods(0);
    pub const CONTROL: Mods = Mods(0);
    pub const ALT: Mods = Mods(0);
    pub const SUPER: Mods = Mods(0);

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
