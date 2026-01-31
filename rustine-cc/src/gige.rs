#![allow(dead_code)]

mod client;
pub use client::*;

mod demosaic;
pub use demosaic::*;

use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::atomic;
use std::time::Duration;

use rustine::io::{ByteSliceReader, ByteSliceWriter};

use crate::network::{HardwareAddress, IPAdapter};

pub static REQUEST_ID: atomic::AtomicU16 = atomic::AtomicU16::new(1);

pub const GVCP_PORT: u16 = 3956;
pub const GVCP_CAPABILITIES_REGISTER: u32 = 0x00000934;
pub const GVCP_CONTROL_ACCESS_REGISTER: u32 = 0x00000a00;
pub const GVCP_HEARTBEAT_TIMEOUT_REGISTER: u32 = 0x00000938;

const GVCP_BROADCAST_ADDR: SocketAddrV4 =
    SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), GVCP_PORT);

/*
public static class Constants
{
    public const int DiscoveryTimeout = 500 * 2000;

    public static class GVCP
    {
        public const int Port = 3956;

        public const uint MaximumDataSize = 512;

        public const uint ControlAccessRegister = 0x00000a00;
        public const uint ControlAccessOn = 1 << 1;
        public const uint ControlAccessOff = 0;
        public const uint ControlAccessExclusive = 1 << 0;

        public const uint StreamSourcePortRegister = 0x00000D1C;

        public const uint CapabilitiesRegister = 0x00000934;

        public const uint HeartbeatTimeoutRegister = 0x00000938;

        public const uint TIMESTAMP_CONTROL_ADDRESS = 0x00000944;
        public const uint TIMESTAMP_LATCHED_VALUE_HIGH_ADDRESS = 0x0000093c;
        public const uint TIMESTAMP_LATCHED_VALUE_LOW_ADDRESS = 0x00000940;

        public const uint TIMESTAMP_TICK_FREQUENCY_HIGH_ADDRESS = 0x0000093c;
        public const uint TIMESTAMP_TICK_FREQUENCY_LOW_ADDRESS = 0x00000940;

        public const uint XmlUrl0Address = 0x00000200;
        public const uint XmlUrl1Address = 0x00000400;
        public const uint XmlUrlSize = 512;

        public const uint PersistentIPRegister = 0x0000064c;
        public const uint PersistentSubnetMaskRegister = 0x0000065c;
        public const uint PersistentDefaultGatewayRegister = 0x0000066c;

        public const uint PersistentIPConfigurationRegister = 0x00000014;

        public const ushort ForceIPPacketDataSize = 56;
    }
}
*/

#[repr(u16)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GVSPPacketStatus {
    SUCCESS = 0x0000,
    RESEND = 0x0100,
    PACKET_UNAVAILABLE = 0x800c,
}

impl GVSPPacketStatus {
    pub fn from_u16(u: u16) -> Self {
        match u {
            0x0000 => GVSPPacketStatus::SUCCESS,
            0x0100 => GVSPPacketStatus::RESEND,
            0x800c => GVSPPacketStatus::PACKET_UNAVAILABLE,
            _ => panic!("Unknown GVSPPacketStatus: {:#06x}", u),
        }
    }

    pub fn to_u16(&self) -> u16 {
        *self as u16
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GVSPFormat(pub u8);

impl std::ops::BitOr for GVSPFormat {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl GVSPFormat {
    pub const LEADER: Self = Self(0x01);
    pub const TRAILER: Self = Self(0x02);
    pub const PAYLOAD: Self = Self(0x03);
    pub const ALL_IN: Self = Self(0x04);

    pub fn from_u8(u: u8) -> Self {
        Self(u)
    }

    pub fn to_u8(&self) -> u8 {
        self.0
    }

    pub fn has(&self, flag: GVSPFormat) -> bool {
        (self.0 & flag.0) != 0
    }
}

#[repr(u16)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GVSPPayloadType {
    IMAGE = 0x0001,
    RAWDATA = 0x0002,
    FILE = 0x0003,
    CHUNK_DATA = 0x0004,
    EXTENDED_CHUNK_DATA = 0x0005,
    JPEG = 0x0006,
    JPEG2000 = 0x0007,
    H264 = 0x0008,
    MULTIZONE_IMAGE = 0x0009,
    IMAGE_EXTENDED_CHUNK = 0x4001,
}

impl GVSPPayloadType {
    pub fn from_u16(u: u16) -> Self {
        match u {
            0x0001 => GVSPPayloadType::IMAGE,
            0x0002 => GVSPPayloadType::RAWDATA,
            0x0003 => GVSPPayloadType::FILE,
            0x0004 => GVSPPayloadType::CHUNK_DATA,
            0x0005 => GVSPPayloadType::EXTENDED_CHUNK_DATA,
            0x0006 => GVSPPayloadType::JPEG,
            0x0007 => GVSPPayloadType::JPEG2000,
            0x0008 => GVSPPayloadType::H264,
            0x0009 => GVSPPayloadType::MULTIZONE_IMAGE,
            0x4001 => GVSPPayloadType::IMAGE_EXTENDED_CHUNK,
            _ => panic!("Unknown GVSPPayloadType: {:#06x}", u),
        }
    }

    pub fn to_u16(&self) -> u16 {
        *self as u16
    }
}

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GVSPPixelFormat {
    MONO_8 = 0x01080001u32,
    MONO_8_SIGNED = 0x01080002u32,

    MONO_10 = 0x01100003u32,
    MONO_10_PACKED = 0x010c0004u32,

    MONO_12 = 0x01100005u32,
    MONO_12_PACKED = 0x010c0006u32,

    MONO_14 = 0x01100025u32,

    MONO_16 = 0x01100007u32,

    BAYER_GR_8 = 0x01080008u32,
    BAYER_RG_8 = 0x01080009u32,
    BAYER_GB_8 = 0x0108000au32,
    BAYER_BG_8 = 0x0108000bu32,

    BAYER_GR_10 = 0x0110000cu32,
    BAYER_RG_10 = 0x0110000du32,
    BAYER_GB_10 = 0x0110000eu32,
    BAYER_BG_10 = 0x0110000fu32,

    BAYER_GR_12 = 0x01100010u32,
    BAYER_RG_12 = 0x01100011u32,
    BAYER_GB_12 = 0x01100012u32,
    BAYER_BG_12 = 0x01100013u32,

    BAYER_GR_16 = 0x0110002eu32,
    BAYER_RG_16 = 0x0110002fu32,
    BAYER_GB_16 = 0x01100030u32,
    BAYER_BG_16 = 0x01100031u32,

    BAYER_BG_10P = 0x010a0052u32,
    BAYER_GB_10P = 0x010a0054u32,
    BAYER_GR_10P = 0x010a0056u32,
    BAYER_RG_10P = 0x010a0058u32,

    BAYER_BG_12P = 0x010c0053u32,
    BAYER_GB_12P = 0x010c0055u32,
    BAYER_GR_12P = 0x010c0057u32,
    BAYER_RG_12P = 0x010c0059u32,

    BAYER_GR_12_PACKED = 0x010c002au32,
    BAYER_RG_12_PACKED = 0x010c002bu32,
    BAYER_GB_12_PACKED = 0x010c002cu32,
    BAYER_BG_12_PACKED = 0x010c002du32,

    BAYER_GR_10_PACKED = 0x010c0026u32,
    BAYER_RG_10_PACKED = 0x010c0027u32,
    BAYER_GB_10_PACKED = 0x010c0028u32,
    BAYER_BG_10_PACKED = 0x010c0029u32,

    RGB_8_PACKED = 0x02180014u32,
    BGR_8_PACKED = 0x02180015u32,

    RGBA_8_PACKED = 0x02200016u32,
    BGRA_8_PACKED = 0x02200017u32,

    RGB_10_PACKED = 0x02300018u32,
    BGR_10_PACKED = 0x02300019u32,

    RGB_12_PACKED = 0x0230001au32,
    BGR_12_PACKED = 0x0230001bu32,

    YUV_411_PACKED = 0x020c001eu32,
    YUV_422_PACKED = 0x0210001fu32,
    YUV_444_PACKED = 0x02180020u32,

    RGB_8_PLANAR = 0x02180021u32,
    RGB_10_PLANAR = 0x02300022u32,
    RGB_12_PLANAR = 0x02300023u32,
    RGB_16_PLANAR = 0x02300024u32,
}

impl GVSPPixelFormat {
    pub fn from_u32(u: u32) -> Self {
        match u {
            0x01080001 => GVSPPixelFormat::MONO_8,
            0x01080002 => GVSPPixelFormat::MONO_8_SIGNED,
            0x01100003 => GVSPPixelFormat::MONO_10,
            0x010c0004 => GVSPPixelFormat::MONO_10_PACKED,
            0x01100005 => GVSPPixelFormat::MONO_12,
            0x010c0006 => GVSPPixelFormat::MONO_12_PACKED,
            0x01100025 => GVSPPixelFormat::MONO_14,
            0x01100007 => GVSPPixelFormat::MONO_16,
            0x01080008 => GVSPPixelFormat::BAYER_GR_8,
            0x01080009 => GVSPPixelFormat::BAYER_RG_8,
            0x0108000a => GVSPPixelFormat::BAYER_GB_8,
            0x0108000b => GVSPPixelFormat::BAYER_BG_8,
            0x0110000c => GVSPPixelFormat::BAYER_GR_10,
            0x0110000d => GVSPPixelFormat::BAYER_RG_10,
            0x0110000e => GVSPPixelFormat::BAYER_GB_10,
            0x0110000f => GVSPPixelFormat::BAYER_BG_10,
            0x01100010 => GVSPPixelFormat::BAYER_GR_12,
            0x01100011 => GVSPPixelFormat::BAYER_RG_12,
            0x01100012 => GVSPPixelFormat::BAYER_GB_12,
            0x01100013 => GVSPPixelFormat::BAYER_BG_12,
            0x0110002e => GVSPPixelFormat::BAYER_GR_16,
            0x0110002f => GVSPPixelFormat::BAYER_RG_16,
            0x01100030 => GVSPPixelFormat::BAYER_GB_16,
            0x01100031 => GVSPPixelFormat::BAYER_BG_16,
            0x010a0052 => GVSPPixelFormat::BAYER_BG_10P,
            0x010a0054 => GVSPPixelFormat::BAYER_GB_10P,
            0x010a0056 => GVSPPixelFormat::BAYER_GR_10P,
            0x010a0058 => GVSPPixelFormat::BAYER_RG_10P,
            0x010c0053 => GVSPPixelFormat::BAYER_BG_12P,
            0x010c0055 => GVSPPixelFormat::BAYER_GB_12P,
            0x010c0057 => GVSPPixelFormat::BAYER_GR_12P,
            0x010c0059 => GVSPPixelFormat::BAYER_RG_12P,
            0x010c002a => GVSPPixelFormat::BAYER_GR_12_PACKED,
            0x010c002b => GVSPPixelFormat::BAYER_RG_12_PACKED,
            0x010c002c => GVSPPixelFormat::BAYER_GB_12_PACKED,
            0x010c002d => GVSPPixelFormat::BAYER_BG_12_PACKED,
            0x010c0026 => GVSPPixelFormat::BAYER_GR_10_PACKED,
            0x010c0027 => GVSPPixelFormat::BAYER_RG_10_PACKED,
            0x010c0028 => GVSPPixelFormat::BAYER_GB_10_PACKED,
            0x010c0029 => GVSPPixelFormat::BAYER_BG_10_PACKED,
            0x02180014 => GVSPPixelFormat::RGB_8_PACKED,
            0x02180015 => GVSPPixelFormat::BGR_8_PACKED,
            0x02200016 => GVSPPixelFormat::RGBA_8_PACKED,
            0x02200017 => GVSPPixelFormat::BGRA_8_PACKED,
            0x02300018 => GVSPPixelFormat::RGB_10_PACKED,
            0x02300019 => GVSPPixelFormat::BGR_10_PACKED,
            0x0230001a => GVSPPixelFormat::RGB_12_PACKED,
            0x0230001b => GVSPPixelFormat::BGR_12_PACKED,
            0x020c001e => GVSPPixelFormat::YUV_411_PACKED,
            0x0210001f => GVSPPixelFormat::YUV_422_PACKED,
            0x02180020 => GVSPPixelFormat::YUV_444_PACKED,
            0x02180021 => GVSPPixelFormat::RGB_8_PLANAR,
            0x02300022 => GVSPPixelFormat::RGB_10_PLANAR,
            0x02300023 => GVSPPixelFormat::RGB_12_PLANAR,
            0x02300024 => GVSPPixelFormat::RGB_16_PLANAR,
            _ => panic!("Unknown GVSPPixelFormat: {:#010x}", u),
        }
    }

    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}

#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GigEStatus {
    /// Indicates a successful operation.
    SUCCESS = 0x00,
    /// Only applies to packet being resent.
    /// This flag is preferred over the GEV_STATUS_SUCCESS when the GVSP transmitter sends a resent packet.
    /// This can be used by a GVSP receiver to better monitor packet resend.
    /// Note that bit 15 of the GVSP packet flag field must be set for a retransmitted GVSP packet when block_id64 are used.
    PACKET_RESEND = 0x10,
    /// Command is not supported by the device.
    NOT_IMPLEMENTED = 0x01,
    /// At least one parameter provided in the command is invalid (or out of range) for the device.
    INVALID_PARAMETER = 0x02,
    /// An attempt was made to access a non-existent address space location.
    INVALID_ADDRESS = 0x03,
    /// The attempted write operation was blocked due to write protection.
    WRITE_PROTECT = 0x04,
    /// The data was not aligned properly.
    BAD_ALIGNMENT = 0x05,
    /// Access to the requested resource was denied.
    ACCESS_DENIED = 0x06,
    /// The device is currently busy and cannot process the request.
    BUSY = 0x07,
    /// The requested packet is not available anymore.
    PACKET_UNAVAILABLE = 0x0C,
    /// Internal memory of GVSP transmitter overrun (typically for image acquisition).
    DATA_OVERRUN = 0x0D,
    /// The message header is not valid. Some of its fields do not match the specification.
    INVALID_HEADER = 0x0E,
    /// An unspecified error occurred.
    ERROR = 0xFF,
}

impl GigEStatus {
    pub fn from_u8(u: u8) -> Self {
        match u {
            0x00 => GigEStatus::SUCCESS,
            0x10 => GigEStatus::PACKET_RESEND,
            0x01 => GigEStatus::NOT_IMPLEMENTED,
            0x02 => GigEStatus::INVALID_PARAMETER,
            0x03 => GigEStatus::INVALID_ADDRESS,
            0x04 => GigEStatus::WRITE_PROTECT,
            0x05 => GigEStatus::BAD_ALIGNMENT,
            0x06 => GigEStatus::ACCESS_DENIED,
            0x07 => GigEStatus::BUSY,
            0x0C => GigEStatus::PACKET_UNAVAILABLE,
            0x0D => GigEStatus::DATA_OVERRUN,
            0x0E => GigEStatus::INVALID_HEADER,
            0xFF => GigEStatus::ERROR,
            _ => panic!("Unknown GigEStatus: {}", u),
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GigECapabilities(u32);
impl GigECapabilities {
    pub const NONE: GigECapabilities = GigECapabilities(0);
    pub const CONCATENATION: GigECapabilities = GigECapabilities(1 << 0);
    pub const WRITE_MEMORY: GigECapabilities = GigECapabilities(1 << 1);
    pub const PACKET_RESEND: GigECapabilities = GigECapabilities(1 << 2);
    pub const EVENT: GigECapabilities = GigECapabilities(1 << 3);
    pub const EVENT_DATA: GigECapabilities = GigECapabilities(1 << 4);
    pub const PENDING_ACK: GigECapabilities = GigECapabilities(1 << 5);
    pub const ACTION: GigECapabilities = GigECapabilities(1 << 6);
    pub const IEEEE1588_EXTENDED: GigECapabilities = GigECapabilities(1 << 16);
    pub const SCHEDULED_ACTION: GigECapabilities = GigECapabilities(1 << 17);
    pub const STATUS_CODE: GigECapabilities = GigECapabilities(1 << 18);
    pub const CAPABILITY_1588: GigECapabilities = GigECapabilities(1 << 19);
    pub const UNCONDITIONAL_ACTION: GigECapabilities = GigECapabilities(1 << 20);
    pub const PRIMARY_APPLICATION_SWITCHOVER: GigECapabilities = GigECapabilities(1 << 21);
    pub const EXTENDED_STATUS_CODES: GigECapabilities = GigECapabilities(1 << 22);
    pub const DISCOVERY_ACK_DELAY_WRITABLE: GigECapabilities = GigECapabilities(1 << 23);
    pub const DISCOVERY_ACK_DELAY: GigECapabilities = GigECapabilities(1 << 24);
    pub const TEST_DATA: GigECapabilities = GigECapabilities(1 << 25);
    pub const MANIFEST_TABLE: GigECapabilities = GigECapabilities(1 << 26);
    pub const CCP_APPLICATION_SOCKET: GigECapabilities = GigECapabilities(1 << 27);
    pub const LINK_SPEED: GigECapabilities = GigECapabilities(1 << 28);
    pub const HEARTBEAT_DISABLE: GigECapabilities = GigECapabilities(1 << 29);
    pub const SERIAL_NUMBER: GigECapabilities = GigECapabilities(1 << 30);
    pub const NAME_REGISTER: GigECapabilities = GigECapabilities(1 << 31);

    pub fn has(&self, capability: GigECapabilities) -> bool {
        (self.0 & capability.0) != 0
    }

    pub fn from_u32(u: u32) -> Self {
        GigECapabilities(u)
    }
}

impl std::fmt::Debug for GigECapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GigECapabilities")
            .field("WRITE_MEMORY", &(self.has(GigECapabilities::WRITE_MEMORY)))
            .field("PACKET_RESEND", &(self.has(GigECapabilities::PACKET_RESEND)))
            .field("PENDING_ACK", &(self.has(GigECapabilities::PENDING_ACK)))
            .finish()
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum GigECommand {
    DISCOVERY_CMD = 0x0002,
    DISCOVERY_ACK = 0x0003,
    BYE_CMD = 0x0004,
    BYE_ACK = 0x0005,
    PACKET_RESEND_CMD = 0x0040,
    PACKET_RESEND_ACK = 0x0041,
    READ_REGISTER_CMD = 0x0080,
    READ_REGISTER_ACK = 0x0081,
    WRITE_REGISTER_CMD = 0x0082,
    WRITE_REGISTER_ACK = 0x0083,
    READ_MEMORY_CMD = 0x0084,
    READ_MEMORY_ACK = 0x0085,
    WRITE_MEMORY_CMD = 0x0086,
    WRITE_MEMORY_ACK = 0x0087,
    PENDING_ACK = 0x0089,
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
        *self as u16
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum GigEPacketType {
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
pub struct GigEPacket<'a> {
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
        data: &'a [u8],
    ) -> Self {
        Self {
            t,
            flags,
            command,
            size: data.len() as u16,
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

    pub fn to_slice(&self, dst: &mut [u8]) -> std::io::Result<usize> {
        let mut writer = ByteSliceWriter::new(dst);

        writer.write_u8(self.t.to_u8())?;
        writer.write_u8(self.flags.0)?;
        writer.write_u16_be(self.command.to_u16())?;
        writer.write_u16_be(self.size)?;
        writer.write_u16_be(self.id)?;

        let data_len = self.data.len();
        for i in 0..data_len {
            writer.write_u8(self.data[i])?;
        }

        Ok(writer.position())
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
pub struct GigEDevice {
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

pub fn discover(a: &IPAdapter) -> std::io::Result<Vec<std::io::Result<GigEDevice>>> {
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

            let discovery_packet = GigEPacket::new(
                GigEPacketType::CMD,
                GigEPacketFlags::ACK_REQUIRED | GigEPacketFlags::ALLOW_BROADCAST_ACK,
                GigECommand::DISCOVERY_CMD,
                REQUEST_ID.fetch_add(1, atomic::Ordering::Relaxed),
                &[],
            );

            let mut buf = [0u8; 1500];
            discovery_packet.to_slice(&mut buf)?;

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
