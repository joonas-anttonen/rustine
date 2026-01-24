#![allow(dead_code)]

use std::rc::Rc;

use crate::gfx;

pub mod ffmpeg;
pub mod gltf;
pub mod webp;

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: gfx::Format,
    pub pixels: Vec<u8>,
}

pub enum MeshMemory {
    Points(Vec<gfx::mesh::GpuCloudVertex>),
    Wires(Vec<gfx::mesh::GpuWireVertex>),
    Triangles(Vec<gfx::mesh::GpuMeshVertex>),
}

pub struct Mesh {
    pub memory: Rc<MeshMemory>,
    pub primitives: Vec<MeshPrimitive>,
}

pub struct MeshPrimitive {
    pub offset: u32,
    pub count: u32,
    pub material: Option<u32>,
}

pub struct Model {}
pub struct Node {}
