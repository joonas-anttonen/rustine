#![allow(dead_code)]

use crate::{Matrix4f, gfx};

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

#[derive(Clone)]
pub struct MeshPrimitive {
    pub offset: u32,
    pub count: u32,
    pub material: u32,
}

pub struct Mesh {
    pub topology: gfx::Topology,
    pub primitives: Vec<MeshPrimitive>,
}

pub struct Node {
    pub name: Option<String>,
    pub mesh: Option<u32>,
    pub transform: Matrix4f,
    pub children: Vec<u32>,
}

pub struct Model {
    pub triangle_memory: Vec<gfx::mesh::GpuMeshVertex>,
    pub wire_memory: Vec<gfx::mesh::GpuWireVertex>,
    pub point_memory: Vec<gfx::mesh::GpuPointVertex>,
    pub materials: Vec<gfx::mesh::Material>,
    pub meshes: Vec<Mesh>,
    pub nodes: Vec<Node>,
}
