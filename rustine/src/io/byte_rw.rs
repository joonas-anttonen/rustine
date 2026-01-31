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

pub struct ByteSliceReader<'a>(&'a [u8], usize);

impl<'a> ByteSliceReader<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self(slice, 0)
    }

    pub fn read_count(&mut self, count: usize) -> std::io::Result<&'a [u8]> {
        if self.1 + count > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let start = self.1;
        self.1 += count;
        Ok(&self.0[start..self.1])
    }

    pub fn read_to_end(&mut self) -> std::io::Result<&'a [u8]> {
        let start = self.1;
        self.1 = self.0.len();
        Ok(&self.0[start..])
    }

    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        if self.1 >= self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let val = self.0[self.1];
        self.1 += 1;
        Ok(val)
    }

    pub fn read_u16_le(&mut self) -> std::io::Result<u16> {
        if self.1 + 2 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let low = self.0[self.1] as u16;
        let high = self.0[self.1 + 1] as u16;
        self.1 += 2;
        Ok((high << 8) | low)
    }

    pub fn read_u16_be(&mut self) -> std::io::Result<u16> {
        if self.1 + 2 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let high = self.0[self.1] as u16;
        let low = self.0[self.1 + 1] as u16;
        self.1 += 2;
        Ok((high << 8) | low)
    }

    pub fn read_u32_le(&mut self) -> std::io::Result<u32> {
        if self.1 + 4 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let b0 = self.0[self.1] as u32;
        let b1 = self.0[self.1 + 1] as u32;
        let b2 = self.0[self.1 + 2] as u32;
        let b3 = self.0[self.1 + 3] as u32;
        self.1 += 4;
        Ok((b3 << 24) | (b2 << 16) | (b1 << 8) | b0)
    }

    pub fn read_u32_be(&mut self) -> std::io::Result<u32> {
        if self.1 + 4 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let b3 = self.0[self.1] as u32;
        let b2 = self.0[self.1 + 1] as u32;
        let b1 = self.0[self.1 + 2] as u32;
        let b0 = self.0[self.1 + 3] as u32;
        self.1 += 4;
        Ok((b3 << 24) | (b2 << 16) | (b1 << 8) | b0)
    }

    pub fn read_u64_le(&mut self) -> std::io::Result<u64> {
        if self.1 + 8 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let b0 = self.0[self.1] as u64;
        let b1 = self.0[self.1 + 1] as u64;
        let b2 = self.0[self.1 + 2] as u64;
        let b3 = self.0[self.1 + 3] as u64;
        let b4 = self.0[self.1 + 4] as u64;
        let b5 = self.0[self.1 + 5] as u64;
        let b6 = self.0[self.1 + 6] as u64;
        let b7 = self.0[self.1 + 7] as u64;
        self.1 += 8;
        Ok((b7 << 56)
            | (b6 << 48)
            | (b5 << 40)
            | (b4 << 32)
            | (b3 << 24)
            | (b2 << 16)
            | (b1 << 8)
            | b0)
    }

    pub fn read_u64_be(&mut self) -> std::io::Result<u64> {
        if self.1 + 8 > self.0.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let b7 = self.0[self.1] as u64;
        let b6 = self.0[self.1 + 1] as u64;
        let b5 = self.0[self.1 + 2] as u64;
        let b4 = self.0[self.1 + 3] as u64;
        let b3 = self.0[self.1 + 4] as u64;
        let b2 = self.0[self.1 + 5] as u64;
        let b1 = self.0[self.1 + 6] as u64;
        let b0 = self.0[self.1 + 7] as u64;
        self.1 += 8;
        Ok((b7 << 56)
            | (b6 << 48)
            | (b5 << 40)
            | (b4 << 32)
            | (b3 << 24)
            | (b2 << 16)
            | (b1 << 8)
            | b0)
    }

    pub fn read_utf8(&mut self, count: usize) -> std::io::Result<&'a str> {
        let bytes = self.read_count(count)?;

        // Trim trailing null bytes
        let bytes = if let Some(pos) = bytes.iter().position(|&b| b == 0) {
            &bytes[..pos]
        } else {
            bytes
        };

        std::str::from_utf8(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

pub struct ByteSliceWriter<'a>(&'a mut [u8], usize);

impl<'a> ByteSliceWriter<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self(slice, 0)
    }

    pub fn position(&self) -> usize {
        self.1
    }

    pub fn write_u8(&mut self, val: u8) -> std::io::Result<()> {
        self.0[self.1] = val;
        self.1 += 1;
        Ok(())
    }

    pub fn write_u16_le(&mut self, val: u16) -> std::io::Result<()> {
        self.write_u8((val & 0x00FF) as u8)?;
        self.write_u8(((val & 0xFF00) >> 8) as u8)?;
        Ok(())
    }

    pub fn write_u16_be(&mut self, val: u16) -> std::io::Result<()> {
        self.write_u8(((val & 0xFF00) >> 8) as u8)?;
        self.write_u8((val & 0x00FF) as u8)?;
        Ok(())
    }
}
