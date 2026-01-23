#![allow(dead_code)]

pub mod ffmpeg;
pub mod gltf;
pub mod webp;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: crate::gfx::Format,
    pub pixels: Vec<u8>,
}

pub struct Model {}
