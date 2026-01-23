#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

pub fn deserialize(path: &str) -> Result<crate::io::Model, std::io::Error> {
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

    let gltf_model = serde_json::from_slice::<Gltf>(json_chunk.unwrap())?;

    // Just panic with data from the glTF model for now
    panic!("{:#?}", gltf_model);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gltf {
    pub asset: Option<Asset>,
    #[serde(rename = "buffers")]
    pub buffers: Option<Vec<Buffer>>,
    #[serde(rename = "bufferViews")]
    pub buffer_views: Option<Vec<BufferView>>,
    #[serde(rename = "accessors")]
    pub accessors: Option<Vec<Accessor>>,
    pub meshes: Option<Vec<Mesh>>,
    pub nodes: Option<Vec<Node>>,
    pub scenes: Option<Vec<Scene>>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub version: Option<String>,
    pub generator: Option<String>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Buffer {
    pub uri: Option<String>,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<u32>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BufferView {
    pub buffer: Option<u32>,
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u32>,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<u32>,
    pub target: Option<u32>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Accessor {
    #[serde(rename = "bufferView")]
    pub buffer_view: Option<u32>,
    #[serde(rename = "byteOffset")]
    pub byte_offset: Option<u32>,
    #[serde(rename = "componentType")]
    pub component_type: Option<u32>,
    pub count: Option<u32>,
    #[serde(rename = "type")]
    pub accessor_type: Option<String>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mesh {
    pub primitives: Option<Vec<Primitive>>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub extras: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Primitive {
    pub attributes: Option<std::collections::HashMap<String, u32>>,
    pub indices: Option<u32>,
    pub material: Option<u32>,
    pub mode: Option<u32>,
    #[serde(flatten)]
    pub extras: Option<Value>,
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
