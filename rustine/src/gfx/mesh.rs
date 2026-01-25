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
pub struct GpuPointVertex {
    pub position: Vector3f,
    pub color: u32,
}

/// Material properties for a mesh primitive.
#[derive(Copy, Clone, PartialEq)]
pub struct Material {
    pub diffuse: Vector4f,
    pub metalness: f32,
    pub roughness: f32,
}

impl std::hash::Hash for Material {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.diffuse.hash(state);
        self.metalness.to_bits().hash(state);
        self.roughness.to_bits().hash(state);
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            diffuse: Vector4f::new(1.0, 1.0, 1.0, 1.0),
            metalness: 1.0,
            roughness: 1.0,
        }
    }
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
