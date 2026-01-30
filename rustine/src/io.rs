#![allow(dead_code)]

use crate::{Matrix4f, gfx};

mod byte_rw;
pub use byte_rw::*;
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

pub struct Buffer {
    pub size: usize,
    pub data: Vec<u8>,
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

impl Model {
    pub fn calculate_memory_size(&self) -> usize {
        self.triangle_memory.len() * std::mem::size_of::<gfx::mesh::GpuMeshVertex>()
            + self.wire_memory.len() * std::mem::size_of::<gfx::mesh::GpuWireVertex>()
            + self.point_memory.len() * std::mem::size_of::<gfx::mesh::GpuPointVertex>()
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("triangle_memory", &self.triangle_memory.len())
            .field("wire_memory", &self.wire_memory.len())
            .field("point_memory", &self.point_memory.len())
            .field("materials", &self.materials.len())
            .field("meshes", &self.meshes.len())
            .field("nodes", &self.nodes.len())
            .finish()
    }
}
