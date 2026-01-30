#![allow(dead_code)]

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

    pub fn read_slice(&mut self, length: usize) -> Option<&[u8]> {
        let start = self.position;
        let end = start + length;
        if end <= self.buffer.len() && start <= end {
            self.position = end;
            Some(&self.buffer[start..end])
        } else {
            None
        }
    }
}