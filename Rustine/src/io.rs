#![allow(dead_code)]

pub mod webp;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: crate::gfx::Format,
    pub pixels: Vec<u8>,
}