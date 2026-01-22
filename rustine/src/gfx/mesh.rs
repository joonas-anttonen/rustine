#![allow(dead_code)]

use std::rc::Rc;

use crate::{Vector2f, Vector3f, Vector4f, gfx::MemoryBuffer};

#[repr(C)]
#[derive(Clone)]
pub struct GpuMeshVertex {
    pub position: Vector3f,
    pub texture: Vector2f,
    pub normal: Vector3f,
}

#[repr(C)]
#[derive(Clone)]
pub struct GpuWireVertex {
    pub position: Vector3f,
    pub color: u32,
}

#[repr(C)]
#[derive(Clone)]
pub struct GpuCloudVertex {
    pub position: Vector3f,
    pub color: u32,
}

/// Topology of a mesh.
pub enum MeshTopology {
    Triangles,
    Lines,
    Points,
}

/// Material properties for a mesh primitive.
pub struct Material {
    pub diffuse: Vector4f,
    pub metalness: f32,
    pub roughness: f32,
}

/// Represents GPU storage of mesh data.
pub struct MeshStorage {
    pub buffer: MemoryBuffer,
}

/// Represents a single drawable primitive within a mesh.
pub struct MeshPrimitive {
    pub material: Material,
}

/// Represents a mesh composed of one or more primitives.
pub struct Mesh {
    pub storage: Rc<MeshStorage>,
    pub primitives: Vec<MeshPrimitive>,
}
