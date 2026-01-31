#![allow(dead_code)]

use rustine::log;

mod network;
use network::{HardwareAddress, IPAdapter};

use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

struct ByteSliceReader<'a>(&'a [u8], usize);

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

struct ByteSliceWriter<'a>(&'a mut [u8], usize);

impl<'a> ByteSliceWriter<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self(slice, 0)
    }

    pub fn write_u8(&mut self, val: u8) {
        self.0[self.1] = val;
        self.1 += 1;
    }

    pub fn write_u16_le(&mut self, val: u16) {
        self.write_u8((val & 0x00FF) as u8);
        self.write_u8(((val & 0xFF00) >> 8) as u8);
    }

    pub fn write_u16_be(&mut self, val: u16) {
        self.write_u8(((val & 0xFF00) >> 8) as u8);
        self.write_u8((val & 0x00FF) as u8);
    }
}

#[repr(u16)]
#[allow(non_camel_case_types)]
enum GigECommand {
    DISCOVERY_CMD,
    DISCOVERY_ACK,
    BYE_CMD,
    BYE_ACK,
    PACKET_RESEND_CMD,
    PACKET_RESEND_ACK,
    READ_REGISTER_CMD,
    READ_REGISTER_ACK,
    WRITE_REGISTER_CMD,
    WRITE_REGISTER_ACK,
    READ_MEMORY_CMD,
    READ_MEMORY_ACK,
    WRITE_MEMORY_CMD,
    WRITE_MEMORY_ACK,
    PENDING_ACK,
}

impl GigECommand {
    pub fn from_u16(u: u16) -> Self {
        match u {
            0x0002 => GigECommand::DISCOVERY_CMD,
            0x0003 => GigECommand::DISCOVERY_ACK,
            0x0004 => GigECommand::BYE_CMD,
            0x0005 => GigECommand::BYE_ACK,
            0x0040 => GigECommand::PACKET_RESEND_CMD,
            0x0041 => GigECommand::PACKET_RESEND_ACK,
            0x0080 => GigECommand::READ_REGISTER_CMD,
            0x0081 => GigECommand::READ_REGISTER_ACK,
            0x0082 => GigECommand::WRITE_REGISTER_CMD,
            0x0083 => GigECommand::WRITE_REGISTER_ACK,
            0x0084 => GigECommand::READ_MEMORY_CMD,
            0x0085 => GigECommand::READ_MEMORY_ACK,
            0x0086 => GigECommand::WRITE_MEMORY_CMD,
            0x0087 => GigECommand::WRITE_MEMORY_ACK,
            0x0089 => GigECommand::PENDING_ACK,
            _ => panic!("Unknown GigECommand: {}", u),
        }
    }

    pub fn to_u16(&self) -> u16 {
        match self {
            GigECommand::DISCOVERY_CMD => 0x0002,
            GigECommand::DISCOVERY_ACK => 0x0003,
            GigECommand::BYE_CMD => 0x0004,
            GigECommand::BYE_ACK => 0x0005,
            GigECommand::PACKET_RESEND_CMD => 0x0040,
            GigECommand::PACKET_RESEND_ACK => 0x0041,
            GigECommand::READ_REGISTER_CMD => 0x0080,
            GigECommand::READ_REGISTER_ACK => 0x0081,
            GigECommand::WRITE_REGISTER_CMD => 0x0082,
            GigECommand::WRITE_REGISTER_ACK => 0x0083,
            GigECommand::READ_MEMORY_CMD => 0x0084,
            GigECommand::READ_MEMORY_ACK => 0x0085,
            GigECommand::WRITE_MEMORY_CMD => 0x0086,
            GigECommand::WRITE_MEMORY_ACK => 0x0087,
            GigECommand::PENDING_ACK => 0x0089,
        }
    }
}

#[repr(u8)]
#[allow(non_camel_case_types)]
enum GigEPacketType {
    ACK,
    CMD,
    ERROR,
}

impl GigEPacketType {
    pub fn from_u8(u: u8) -> Self {
        match u {
            0x00 => GigEPacketType::ACK,
            0x42 => GigEPacketType::CMD,
            0x80 => GigEPacketType::ERROR,
            _ => panic!("Unknown GigEPacketType: {}", u),
        }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            GigEPacketType::ACK => 0x00,
            GigEPacketType::CMD => 0x42,
            GigEPacketType::ERROR => 0x80,
        }
    }
}

#[repr(C)]
struct GigEPacket<'a> {
    pub t: GigEPacketType,
    pub flags: GigEPacketFlags,
    pub command: GigECommand,
    pub size: u16,
    pub id: u16,
    pub data: &'a [u8],
}

impl<'a> GigEPacket<'a> {
    pub fn new(
        t: GigEPacketType,
        flags: GigEPacketFlags,
        command: GigECommand,
        id: u16,
        size: u16,
        data: &'a [u8],
    ) -> Self {
        Self {
            t,
            flags,
            command,
            size,
            id,
            data,
        }
    }

    pub fn from_slice(slice: &'a [u8]) -> std::io::Result<Self> {
        if slice.len() < 8 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        let mut reader = ByteSliceReader::new(slice);

        let t = GigEPacketType::from_u8(reader.read_u8()?);
        let flags = GigEPacketFlags(reader.read_u8()?);

        let command = GigECommand::from_u16(reader.read_u16_be()?);
        let size = reader.read_u16_be()?;
        let id = reader.read_u16_be()?;

        let data = &slice[8..];

        Ok(Self {
            t,
            flags,
            command,
            size,
            id,
            data,
        })
    }

    pub fn to_slice(&self, dst: &mut [u8]) {
        let mut writer = ByteSliceWriter::new(dst);

        writer.write_u8(self.t.to_u8());
        writer.write_u8(self.flags.0);
        writer.write_u16_be(self.command.to_u16());
        writer.write_u16_be(self.size);
        writer.write_u16_be(self.id);

        let data_len = self.data.len();
        for i in 0..data_len {
            writer.write_u8(self.data[i]);
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GigEPacketFlags(u8);

impl std::ops::BitOr for GigEPacketFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl GigEPacketFlags {
    pub const NONE: Self = Self(0);
    pub const ACK_REQUIRED: Self = Self(1 << 0);
    pub const ALLOW_BROADCAST_ACK: Self = Self(1 << 4);
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GigEIPConfiguration(u32);
impl GigEIPConfiguration {
    pub const INVALID: u32 = 0;
    pub const STATIC: u32 = 1 << 0;
    pub const DHCP: u32 = 1 << 1;
    pub const LLA: u32 = 1 << 2;
}

#[derive(Debug)]
struct GigEDevice {
    pub supported_ipconfiguration: GigEIPConfiguration,
    pub current_ipconfiguration: GigEIPConfiguration,
    pub address: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub default_gateway: Ipv4Addr,
    pub hardware_address: HardwareAddress,
    pub vendor: String,
    pub model: String,
    pub version: String,
    //pub manufacturer_information: String,
    pub serial: String,
    //pub user_information: String,
    pub adapter: IPAdapter,
}

impl GigEDevice {
    const MODEL_NAME_SIZE: usize = 32;
    const DEVICE_VERSION_SIZE: usize = 32;
    const MANUFACTURER_NAME_SIZE: usize = 32;
    const MANUFACTURER_INFORMATIONS_SIZE: usize = 48;
    const SERIAL_NUMBER_SIZE: usize = 16;
    const USER_DEFINED_NAME_SIZE: usize = 16;

    pub fn from_packet(packet: &GigEPacket, adapter: IPAdapter) -> std::io::Result<GigEDevice> {
        let mut reader = ByteSliceReader::new(packet.data);

        reader.read_count(10)?;
        let hwaddr = HardwareAddress::from_slice(reader.read_count(6)?)?;

        let supported_ipconfiguration = GigEIPConfiguration(reader.read_u32_be()?);
        let current_ipconfiguration = GigEIPConfiguration(reader.read_u32_be()?);

        reader.read_count(12)?;
        let address = Ipv4Addr::from(reader.read_u32_be()?);
        reader.read_count(12)?;
        let subnet_mask = Ipv4Addr::from(reader.read_u32_be()?);
        reader.read_count(12)?;
        let default_gateway = Ipv4Addr::from(reader.read_u32_be()?);

        let vendor = reader
            .read_utf8(GigEDevice::MANUFACTURER_NAME_SIZE)?
            .trim()
            .to_string();
        let model = reader
            .read_utf8(GigEDevice::MODEL_NAME_SIZE)?
            .trim()
            .to_string();
        let version = reader
            .read_utf8(GigEDevice::DEVICE_VERSION_SIZE)?
            .trim()
            .to_string();
        reader.read_count(GigEDevice::MANUFACTURER_INFORMATIONS_SIZE)?;
        let serial = reader
            .read_utf8(GigEDevice::SERIAL_NUMBER_SIZE)?
            .trim()
            .to_string();
        reader.read_count(GigEDevice::USER_DEFINED_NAME_SIZE)?;

        Ok(GigEDevice {
            supported_ipconfiguration,
            current_ipconfiguration,
            address,
            subnet_mask,
            default_gateway,
            hardware_address: hwaddr,
            vendor,
            model,
            version,
            //manufacturer_information,
            serial,
            //user_information,
            adapter,
        })
    }
}

const GVCP_PORT: u16 = 3956;
const GVCP_BROADCAST_ADDR: SocketAddrV4 =
    SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), GVCP_PORT);

fn discover(a: &IPAdapter) -> std::io::Result<Vec<std::io::Result<GigEDevice>>> {
    let mut discovered_devices: Vec<std::io::Result<GigEDevice>> = Vec::new();

    let bind_addr = SocketAddrV4::new(a.address, 0);
    //let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);

    match UdpSocket::bind(bind_addr) {
        Ok(socket) => {
            if let Err(e) = socket.set_broadcast(true) {
                return Err(std::io::Error::new(e.kind(), "UdpSocket::set_broadcast"));
            }

            if let Err(e) = socket.local_addr() {
                return Err(std::io::Error::new(e.kind(), "UdpSocket::local_addr"));
            }

            let request_id = 2;

            let discovery_packet = GigEPacket::new(
                GigEPacketType::CMD,
                GigEPacketFlags::ACK_REQUIRED | GigEPacketFlags::ALLOW_BROADCAST_ACK,
                GigECommand::DISCOVERY_CMD,
                request_id,
                0,
                &[],
            );

            let mut buf = [0u8; 1500];
            discovery_packet.to_slice(&mut buf);

            if let Err(e) = socket.send_to(&buf[..8], GVCP_BROADCAST_ADDR) {
                return Err(std::io::Error::new(e.kind(), "UdpSocket::send_to"));
            }

            let deadline = std::time::Instant::now() + Duration::from_secs(1);

            loop {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }

                if let Err(e) = socket.set_read_timeout(Some(remaining)) {
                    return Err(std::io::Error::new(e.kind(), "UdpSocket::set_read_timeout"));
                }

                match socket.recv_from(&mut buf) {
                    Ok((read_bytes, _src)) => match GigEPacket::from_slice(&buf[..read_bytes]) {
                        Ok(packet) => match GigEDevice::from_packet(&packet, a.clone()) {
                            Ok(device) => {
                                discovered_devices.push(Ok(device));
                            }
                            Err(e) => {
                                discovered_devices.push(Err(std::io::Error::new(
                                    e.kind(),
                                    "GigEDevice::from_packet",
                                )));
                            }
                        },
                        Err(e) => {
                            discovered_devices
                                .push(Err(std::io::Error::new(e.kind(), "GigEPacket::from_slice")));
                        }
                    },
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => continue,
                        _ => return Err(std::io::Error::new(e.kind(), "UdpSocket::recv_from")),
                    },
                }
            }

            Ok(discovered_devices)
        }
        Err(e) => Err(std::io::Error::new(e.kind(), "UdpSocket::bind")),
    }
}

fn main() {
    log::set_current_thread_name("main");
    log::add_listener(log::ConsoleListener::new(true));

    let adapters: Vec<IPAdapter> = IPAdapter::get_adapters();
    if adapters.is_empty() {
        log::warning!("No ethernet IPv4 adapters found.");
    } else {
        for a in &adapters {
            log::info!("Discovering from: {:#?}", a);

            match discover(a) {
                Ok(devices) => {
                    for device in devices {
                        match device {
                            Ok(d) => {
                                log::info!("Discovered device: {:#?}", d);
                            }
                            Err(e) => {
                                log::error!("Error discovering device: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error during discovery: {}", e);
                }
            }
        }
    }
}
