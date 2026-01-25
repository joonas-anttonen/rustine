#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Matrix4f, Quaternionf, Vector2f, Vector3f, gfx, io};

const GLTF_MAGIC: u32 = 0x46546C67;
const JSON_CHUNK_TYPE: u32 = 0x4E4F534A;
const BINARY_CHUNK_TYPE: u32 = 0x004E4942;

pub struct ByteArrayReader {
    buffer: Vec<u8>,
    position: usize,
}

impl ByteArrayReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        // Enough bytes to read a u32?
        if self.position + 4 <= self.buffer.len() {
            let bytes = &self.buffer[self.position..self.position + 4];
            self.position += 4;
            Some(u32::from_le_bytes(bytes.try_into().unwrap()))
        } else {
            None
        }
    }

    pub fn read_slice(&mut self, start: usize, end: usize) -> Option<&[u8]> {
        if end <= self.buffer.len() && start <= end {
            self.position = end;
            Some(&self.buffer[start..end])
        } else {
            None
        }
    }
}

pub fn deserialize(path: &str) -> Result<Gltf, std::io::Error> {
    let bytes = std::fs::read(path)?;

    let mut reader = ByteArrayReader::new(bytes);

    // Check glTF magic number
    let magic = reader.read_u32();
    if magic != Some(GLTF_MAGIC) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid glTF magic number",
        ));
    }

    let version = reader.read_u32();
    if version != Some(2) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unsupported glTF version",
        ));
    }

    _ = reader.read_u32(); // Ignore length

    let json_chunk_length = reader.read_u32();
    if json_chunk_length.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected JSON chunk length",
        ));
    }
    let json_chunk_type = reader.read_u32();
    if json_chunk_type != Some(JSON_CHUNK_TYPE) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected JSON chunk type",
        ));
    }

    let gltf_model = {
        let json_chunk = reader.read_slice(
            reader.position,
            reader.position + json_chunk_length.unwrap() as usize,
        );
        if json_chunk.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected JSON chunk data",
            ));
        }
        serde_json::from_slice::<Gltf>(json_chunk.unwrap())?
    };

    let binary_chunk_length = reader.read_u32();
    if binary_chunk_length.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected binary chunk length",
        ));
    }
    let binary_chunk_type = reader.read_u32();
    if binary_chunk_type != Some(BINARY_CHUNK_TYPE) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected binary chunk type",
        ));
    }

    let binary_chunk = reader.read_slice(
        reader.position,
        reader.position + binary_chunk_length.unwrap() as usize,
    );
    if binary_chunk.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Expected binary chunk data",
        ));
    }

    let gltf_buffer = binary_chunk.unwrap().to_vec();

    // Attach the binary buffer data to the glTF model
    let mut gltf_model = gltf_model;
    if let Some(first_buffer) = gltf_model.buffers.get_mut(0) {
        first_buffer.data = Some(gltf_buffer);
    } else {
        gltf_model.buffers.push(Buffer {
            uri: None,
            byte_length: Some(gltf_buffer.len() as u32),
            data: Some(gltf_buffer),
        });
    }

    Ok(gltf_model)
}

struct PseudoMesh {
    pub primitive_offset: u32,
    pub primitive_count: u32,
}

pub fn parse(gltf: Gltf) -> Result<io::Model, std::io::Error> {
    let mut primitives = Vec::<io::MeshPrimitive>::with_capacity(2048);
    let mut pseudo_meshes = Vec::<PseudoMesh>::with_capacity(2048);

    let mut triangle_vertices = Vec::<gfx::mesh::GpuMeshVertex>::with_capacity(u16::MAX as usize);
    let wire_vertices = Vec::<gfx::mesh::GpuWireVertex>::with_capacity(u16::MAX as usize);
    let points_vertices = Vec::<gfx::mesh::GpuCloudVertex>::with_capacity(u16::MAX as usize);

    let mut mesh_index_mapping = std::collections::HashMap::<u32, u32>::new();

    let mut current_topology = None;

    for (mesh_index, mesh) in gltf.meshes.iter().enumerate() {
        let primitive_offset = primitives.len() as u32;

        for primitive in &mesh.primitives {
            let topology = {
                if let Some(ptopology) = primitive.topology() {
                    if current_topology.is_some() && current_topology != Some(ptopology) {
                        continue; // Skip inconsistent primitive topology
                    }
                    current_topology = Some(ptopology);
                    ptopology
                } else {
                    continue; // Skip unsupported primitive topology
                }
            };

            let positions_accessor = *primitive.attributes.get("POSITION").ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Expected POSITION attribute",
                )
            })?;
            let indices_accessor = primitive.indices;
            //let normals_accessor = primitive.attributes.get("NORMAL");
            //let uvs_accessor = primitive.attributes.get("TEXCOORD_0");
            //let colors_accessor = primitive.attributes.get("COLOR_0");

            let vertex_start = match topology {
                gfx::Topology::Triangles => triangle_vertices.len() as u32,
                gfx::Topology::Wires => wire_vertices.len() as u32,
                gfx::Topology::Points => points_vertices.len() as u32,
            };
            let vertex_count = if let Some(indices_accessor) = indices_accessor {
                gltf.read_count(indices_accessor)?
            } else {
                let vertex_count = gltf.read_count(positions_accessor)?;
                match topology {
                    gfx::Topology::Triangles => vertex_count / 3,
                    gfx::Topology::Wires => vertex_count / 2,
                    _ => vertex_count,
                }
            };

            for i in 0..vertex_count {
                match topology {
                    gfx::Topology::Triangles => {
                        let i0 = indices_accessor
                            .map(|ia| gltf.read_index(ia, i + 0))
                            .unwrap_or(Ok(i + 0))?;
                        let i1 = indices_accessor
                            .map(|ia| gltf.read_index(ia, i + 1))
                            .unwrap_or(Ok(i + 1))?;
                        let i2 = indices_accessor
                            .map(|ia| gltf.read_index(ia, i + 2))
                            .unwrap_or(Ok(i + 2))?;

                        let p0 = gltf.read_vec3(positions_accessor, i0)?;
                        let p1 = gltf.read_vec3(positions_accessor, i1)?;
                        let p2 = gltf.read_vec3(positions_accessor, i2)?;

                        let v0 = gfx::mesh::GpuMeshVertex {
                            position: p0,
                            texture: Vector2f::default(),
                            normal: Vector3f::default(),
                        };
                        let v1 = gfx::mesh::GpuMeshVertex {
                            position: p1,
                            texture: Vector2f::default(),
                            normal: Vector3f::default(),
                        };
                        let v2 = gfx::mesh::GpuMeshVertex {
                            position: p2,
                            texture: Vector2f::default(),
                            normal: Vector3f::default(),
                        };

                        triangle_vertices.push(v0);
                        triangle_vertices.push(v1);
                        triangle_vertices.push(v2);
                    }
                    _ => todo!(),
                }
            }

            primitives.push(io::MeshPrimitive {
                offset: vertex_start,
                count: vertex_count,
                material: None,
            });
        }

        mesh_index_mapping.insert(mesh_index as u32, pseudo_meshes.len() as u32);
        pseudo_meshes.push(PseudoMesh {
            primitive_offset,
            primitive_count: primitives.len() as u32 - primitive_offset,
        });
    }

    let _triangle_mesh_memory = std::rc::Rc::new(io::MeshMemory::Triangles(triangle_vertices));

    let mut meshes = Vec::<std::rc::Rc<io::Mesh>>::with_capacity(gltf.meshes.len());
    {
        for pseudo_mesh in &pseudo_meshes {
            let mesh = io::Mesh {
                primitives: primitives[pseudo_mesh.primitive_offset as usize
                    ..(pseudo_mesh.primitive_offset + pseudo_mesh.primitive_count) as usize]
                    .to_vec(),
            };
            meshes.push(std::rc::Rc::new(mesh));
        }
    }

    let mut nodes = Vec::<io::Node>::with_capacity(gltf.nodes.len());
    {
        for node in &gltf.nodes {
            let mut scale = Vector3f::new(1.0, 1.0, 1.0);
            if let Some(s) = &node.scale {
                scale = Vector3f::new(s[0], s[1], s[2]);
            }

            let mut rotation = Quaternionf::identity();
            if let Some(r) = &node.rotation {
                // glTF storage order: x, y, z, w
                rotation = Quaternionf::new(r[0], r[1], r[2], r[3]);
            }

            let mut translation = Vector3f::new(0.0, 0.0, 0.0);
            if let Some(t) = &node.translation {
                translation = Vector3f::new(t[0], t[1], t[2]);
            }

            if let Some(m) = &node.matrix {
                let transform = Matrix4f::from_slice(m);

                translation = transform.translation();

                let linear = transform.linear();
                let sx = linear.column(0).norm();
                let sy = linear.column(1).norm();
                let sz = linear.column(2).norm();
                scale = Vector3f::new(sx, sy, sz);

                rotation = Quaternionf::from_rotation(&transform.rotation());
            }

            // Construct final 4x4 transform
            let transform = Matrix4f::from_trs(translation, rotation, scale);

            let mesh_index = match &node.mesh {
                Some(i) => Some(match mesh_index_mapping.get(i) {
                    Some(&mesh_index) => Ok(mesh_index),
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Mesh index not found in mapping",
                    )),
                }?),
                None => None,
            };

            let node = io::Node {
                transform: transform,
                mesh: mesh_index,
            };
            nodes.push(node);

            // TESTING: Print scale, rotation, translation, transform
            println!("Scale: {:?}", scale);
            println!("Rotation: {:?}", rotation);
            println!("Translation: {:?}", translation);
        }
    }

    // TESTING: Print PseudoMesh info
    /*for pseudo_mesh in &pseudo_meshes {
        println!(
            "PseudoMesh: primitive_offset={}, primitive_count={}",
            pseudo_mesh.primitive_offset, pseudo_mesh.primitive_count
        );
        // TESTING: Print Primitive info
        for primitive in &primitives {
            println!(
                "Primitive: offset={}, count={}, material={:?}",
                primitive.offset, primitive.count, primitive.material
            );
        }
    }*/

    todo!()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gltf {
    pub asset: Option<Asset>,
    /// Required: No, require anyway
    #[serde(rename = "buffers")]
    pub buffers: Vec<Buffer>,
    /// Required: No, require anyway
    #[serde(rename = "bufferViews")]
    pub buffer_views: Vec<BufferView>,
    /// Required: No, require anyway
    #[serde(rename = "accessors")]
    pub accessors: Vec<Accessor>,
    /// Required: No, require anyway
    pub meshes: Vec<Mesh>,
    /// Required: No, require anyway
    pub nodes: Vec<Node>,
    /// Required: No
    pub scenes: Option<Vec<Scene>>,
}

impl Gltf {
    /*pub fn get_accessor(&self, index: Option<u32>) -> Option<Accessor> {
        self.accessors
            .as_ref()
            .and_then(|accessors| index.and_then(|i| accessors.get(i as usize).cloned()))
    }*/

    pub fn read_index(
        &self,
        accessor_index: u32,
        element_index: u32,
    ) -> Result<u32, std::io::Error> {
        if let Some(accessor) = &self.accessors.get(accessor_index as usize)
            && let Some(buffer_view) = &self.buffer_views.get(accessor.buffer_view as usize)
            && let Some(buffer) = &self.buffers.get(buffer_view.buffer as usize)
            && let Some(buffer_data) = &buffer.data
        {
            let data_start = (buffer_view.byte_offset.unwrap_or(0)
                + accessor.byte_offset.unwrap_or(0)
                + element_index * accessor.byte_stride()) as usize;

            return match accessor.component_type {
                5120 => Ok(i8::from_le_bytes(
                    buffer_data[data_start..data_start + 1]
                        .try_into()
                        .unwrap_or([0; 1]),
                ) as u32),
                5121 => Ok(u8::from_le_bytes(
                    buffer_data[data_start..data_start + 1]
                        .try_into()
                        .unwrap_or([0; 1]),
                ) as u32),
                5122 => Ok(i16::from_le_bytes(
                    buffer_data[data_start..data_start + 2]
                        .try_into()
                        .unwrap_or([0; 2]),
                ) as u32),
                5123 => Ok(u16::from_le_bytes(
                    buffer_data[data_start..data_start + 2]
                        .try_into()
                        .unwrap_or([0; 2]),
                ) as u32),
                5125 => Ok(u32::from_le_bytes(
                    buffer_data[data_start..data_start + 4]
                        .try_into()
                        .unwrap_or([0; 4]),
                ) as u32),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unsupported component type for index",
                )),
            };
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to read index from accessor",
        ))
    }

    pub fn read_count(&self, accessor_index: u32) -> Result<u32, std::io::Error> {
        if let Some(accessor) = &self.accessors.get(accessor_index as usize) {
            return Ok(accessor.count);
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to read count from accessor",
        ))
    }

    pub fn read_vec3(
        &self,
        accessor_index: u32,
        element_index: u32,
    ) -> Result<Vector3f, std::io::Error> {
        if let Some(accessor) = &self.accessors.get(accessor_index as usize)
            && let Some(buffer_view) = &self.buffer_views.get(accessor.buffer_view as usize)
            && let Some(buffer) = &self.buffers.get(buffer_view.buffer as usize)
            && let Some(buffer_data) = &buffer.data
        {
            if accessor.accessor_type != "VEC3" {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Expected accessor type VEC3",
                ));
            }

            if accessor.component_type != 5126 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Expected accessor component type 5126 (FLOAT)",
                ));
            }

            let data_start = (buffer_view.byte_offset.unwrap_or(0)
                + accessor.byte_offset.unwrap_or(0)
                + element_index * accessor.byte_stride()) as usize;

            let data_end = data_start + accessor.byte_stride() as usize;
            let data = &buffer_data[data_start..data_end];
            let x = f32::from_le_bytes(data[0..4].try_into().unwrap_or([0; 4]));
            let y = f32::from_le_bytes(data[4..8].try_into().unwrap_or([0; 4]));
            let z = f32::from_le_bytes(data[8..12].try_into().unwrap_or([0; 4]));

            return Ok(Vector3f::new(x, y, z));
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to read vec3 from accessor",
        ))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub version: Option<String>,
    pub generator: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Buffer {
    pub uri: Option<String>,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<u32>,

    // Internal (non-serialized) fields
    #[serde(skip)]
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BufferView {
    /// Required: Yes
    pub buffer: u32,
    /// Required: No, Default: 0
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u32>,
    /// Required: Yes
    #[serde(rename = "byteLength")]
    pub byte_length: u32,
    /// Required: No
    pub target: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Accessor {
    /// Required: No, require anyway
    #[serde(rename = "bufferView")]
    pub buffer_view: u32,
    /// Required: No, Default: 0
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u32>,
    /// Required: Yes
    #[serde(rename = "componentType")]
    pub component_type: u32,
    /// Required: Yes
    pub count: u32,
    /// Required: Yes
    #[serde(rename = "type")]
    pub accessor_type: String,
}

impl Accessor {
    pub fn byte_stride(&self) -> u32 {
        let bytes_per_component = match self.component_type {
            5120 => 1,
            5121 => 1,
            5122 => 2,
            5123 => 2,
            5125 => 4,
            5126 => 4,
            _ => 0,
        };
        let components_per_element = match self.accessor_type.as_str() {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            "MAT2" => 4,
            "MAT3" => 9,
            "MAT4" => 16,
            _ => 0,
        };

        bytes_per_component * components_per_element
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mesh {
    /// Required: Yes
    pub primitives: Vec<Primitive>,
    /// Required: No
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Primitive {
    /// Required: Yes
    pub attributes: std::collections::HashMap<String, u32>,
    /// Required: No
    pub indices: Option<u32>,
    /// Required: No
    pub material: Option<u32>,
    /// Required: No
    pub mode: Option<u32>,
}

impl Primitive {
    /// 0 = POINTS, 1 = LINES, 2 = LINE_LOOP, 3 = LINE_STRIP, 4 = TRIANGLES, 5 = TRIANGLE_STRIP, 6 = TRIANGLE_FAN
    ///
    /// Default: 4 (TRIANGLES)
    pub fn topology(&self) -> Option<gfx::Topology> {
        match self.mode {
            None => Some(gfx::Topology::Triangles),
            Some(0) => Some(gfx::Topology::Points),
            Some(1) => Some(gfx::Topology::Wires),
            Some(4) => Some(gfx::Topology::Triangles),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub name: Option<String>,
    pub mesh: Option<u32>,
    pub children: Option<Vec<u32>>,
    pub translation: Option<Vec<f32>>,
    pub rotation: Option<Vec<f32>>,
    pub scale: Option<Vec<f32>>,
    pub matrix: Option<Vec<f32>>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Option<Vec<u32>>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}
