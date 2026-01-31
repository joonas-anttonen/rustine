#![allow(dead_code)]

use std::ffi::CStr;
use std::fs;
use std::net::Ipv4Addr;
use std::ptr;
use libc::{freeifaddrs, getifaddrs, ifaddrs, sockaddr_in, AF_INET};

#[derive(Debug, Clone)]
pub struct IPAdapter {
    pub address: Ipv4Addr,
    pub address_mask: Ipv4Addr,
    pub mtu: u32,
    pub name: String,
    pub hwaddr: HardwareAddress,
}

impl IPAdapter {
    pub fn get_adapters() -> Vec<IPAdapter> {
        let mut result = Vec::new();

        unsafe {
            let mut ifap: *mut ifaddrs = ptr::null_mut();
            if getifaddrs(&mut ifap) != 0 {
                return result;
            }

            let mut cur = ifap;
            while !cur.is_null() {
                let ifa = &*cur;
                if !ifa.ifa_addr.is_null() && (ifa.ifa_addr as *const libc::sockaddr).as_ref().map_or(false, |s| s.sa_family as i32 == AF_INET) {
                    // interface name
                    let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();

                    // extract IPv4 address
                    let sa: libc::sockaddr_in = *(ifa.ifa_addr as *const sockaddr_in);
                    let addr_u32 = u32::from_be(sa.sin_addr.s_addr);
                    let address = Ipv4Addr::from(addr_u32);

                    // netmask from ifa_netmask if available
                    let address_mask = if !ifa.ifa_netmask.is_null() {
                        let nm = *(ifa.ifa_netmask as *const sockaddr_in);
                        let mask_u32 = u32::from_be(nm.sin_addr.s_addr);
                        Ipv4Addr::from(mask_u32)
                    } else {
                        Ipv4Addr::from(0u32)
                    };

                    // Check interface sysfs for type and operstate
                    let base = format!("/sys/class/net/{}", name);
                    let operstate = fs::read_to_string(format!("{}/operstate", base))
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if operstate != "up" {
                        cur = ifa.ifa_next;
                        continue;
                    }

                    let iftype = fs::read_to_string(format!("{}/type", base))
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if iftype != "1" {
                        cur = ifa.ifa_next;
                        continue;
                    }

                    let mtu = fs::read_to_string(format!("{}/mtu", base))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .unwrap_or(0);

                    // read hardware (MAC) address if present
                    let hwaddr = fs::read_to_string(format!("{}/address", base))
                        .and_then(|s| HardwareAddress::from_str(&s))
                        .unwrap_or_default();

                    result.push(IPAdapter {
                        address,
                        address_mask,
                        mtu,
                        name,
                        hwaddr,
                    });
                }

                cur = (*cur).ifa_next;
            }

            freeifaddrs(ifap);
        }

        // sort by name to match original behavior
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct HardwareAddress {
    addr: [u8; 6],
}

impl HardwareAddress {
    pub const LENGTH_IN_BYTES: usize = 6;

    pub fn new() -> HardwareAddress {
        HardwareAddress { addr: [0xff; 6] }
    }

    pub fn from_slice(slice: &[u8]) -> std::io::Result<HardwareAddress> {
        if slice.len() < HardwareAddress::LENGTH_IN_BYTES {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let mut a = [0u8; 6];
        a.copy_from_slice(&slice[0..6]);
        Ok(HardwareAddress { addr: a })
    }

    pub fn from_str(s: &str) -> std::io::Result<HardwareAddress> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(|c| c == ':' || c == '-').collect();
        if parts.len() != HardwareAddress::LENGTH_IN_BYTES {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        let mut a = [0u8; 6];
        for i in 0..6 {
            a[i] = u8::from_str_radix(parts[i], 16).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        }
        Ok(HardwareAddress { addr: a })
    }
}

impl Default for HardwareAddress {
    fn default() -> Self {
        HardwareAddress::new()
    }
}

impl std::fmt::Display for HardwareAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.addr[0],
            self.addr[1],
            self.addr[2],
            self.addr[3],
            self.addr[4],
            self.addr[5]
        )
    }
}

impl std::fmt::Debug for HardwareAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HardwareAddress({})", self)
    }

}

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

    public enum GVSPPacketStatus : ushort
    {
        SUCCESS = 0x0000,
        RESEND = 0x0100,
        PACKET_UNAVAILABLE = 0x800c
    }

    [Flags]
    public enum GVSPFormat : byte
    {
        LEADER = 0x01,
        TRAILER = 0x02,
        PAYLOAD = 0x03,
        ALL_IN = 0x04,
    }

    public enum GVSPPayloadType : ushort
    {
        IMAGE = 0x0001,
        RAWDATA = 0x0002,
        FILE = 0x0003,
        CHUNK_DATA = 0x0004,
        EXTENDED_CHUNK_DATA = 0x0005, /* Deprecated */
        JPEG = 0x0006,
        JPEG2000 = 0x0007,
        H264 = 0x0008,
        MULTIZONE_IMAGE = 0x0009,
        IMAGE_EXTENDED_CHUNK = 0x4001,
    }

    public enum GVSPPixelFormat : uint
    {
        /* Grey pixel formats */

        MONO_8 = 0x01080001u,
        MONO_8_SIGNED = 0x01080002u,

        MONO_10 = 0x01100003u,
        MONO_10_PACKED = 0x010c0004u,

        MONO_12 = 0x01100005u,
        MONO_12_PACKED = 0x010c0006u,

        MONO_14 = 0x01100025u,

        MONO_16 = 0x01100007u,

        BAYER_GR_8 = 0x01080008u,
        BAYER_RG_8 = 0x01080009u,
        BAYER_GB_8 = 0x0108000au,
        BAYER_BG_8 = 0x0108000bu,

        BAYER_GR_10 = 0x0110000cu,
        BAYER_RG_10 = 0x0110000du,
        BAYER_GB_10 = 0x0110000eu,
        BAYER_BG_10 = 0x0110000fu,

        BAYER_GR_12 = 0x01100010u,
        BAYER_RG_12 = 0x01100011u,
        BAYER_GB_12 = 0x01100012u,
        BAYER_BG_12 = 0x01100013u,

        BAYER_GR_16 = 0x0110002eu,
        BAYER_RG_16 = 0x0110002fu,
        BAYER_GB_16 = 0x01100030u,
        BAYER_BG_16 = 0x01100031u,

        BAYER_BG_10P = 0x010a0052u,
        BAYER_GB_10P = 0x010a0054u,
        BAYER_GR_10P = 0x010a0056u,
        BAYER_RG_10P = 0x010a0058u,

        BAYER_BG_12P = 0x010c0053u,
        BAYER_GB_12P = 0x010c0055u,
        BAYER_GR_12P = 0x010c0057u,
        BAYER_RG_12P = 0x010c0059u,

        BAYER_GR_12_PACKED = 0x010c002au,
        BAYER_RG_12_PACKED = 0x010c002bu,
        BAYER_GB_12_PACKED = 0x010c002cu,
        BAYER_BG_12_PACKED = 0x010c002du,

        BAYER_GR_10_PACKED = 0x010c0026u,
        BAYER_RG_10_PACKED = 0x010c0027u,
        BAYER_GB_10_PACKED = 0x010c0028u,
        BAYER_BG_10_PACKED = 0x010c0029u,

        /* Color pixel formats */

        RGB_8_PACKED = 0x02180014u,
        BGR_8_PACKED = 0x02180015u,

        RGBA_8_PACKED = 0x02200016u,
        BGRA_8_PACKED = 0x02200017u,

        RGB_10_PACKED = 0x02300018u,
        BGR_10_PACKED = 0x02300019u,

        RGB_12_PACKED = 0x0230001au,
        BGR_12_PACKED = 0x0230001bu,

        YUV_411_PACKED = 0x020c001eu,
        YUV_422_PACKED = 0x0210001fu,
        YUV_444_PACKED = 0x02180020u,

        RGB_8_PLANAR = 0x02180021u,
        RGB_10_PLANAR = 0x02300022u,
        RGB_12_PLANAR = 0x02300023u,
        RGB_16_PLANAR = 0x02300024u,
    }

    public enum GigEPacketType : byte
    {
        ACK = 0x00,
        CMD = 0x42,
        ERROR = 0x80,
        UNKNOWN_ERROR = 0x8f
    }

    public enum GigECommand : ushort
    {
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

    public enum GigEStatus
    {
        /// <summary>
        /// Command executed successfully.
        /// </summary>
        SUCCESS = 0,

        /// <summary>
        /// Only applies to packet being resent.
        /// This flag is preferred over the GEV_STATUS_SUCCESS when the GVSP transmitter sends a resent packet.
        /// This can be used by aGVSP receiver to better monitor packet resend.
        /// Note that bit 15 of the GVSP packet flag field must be set for a retransmitted GVSP packet when block_id64 are used.
        /// </summary>
        PACKET_RESEND = 0x10,

        /// <summary>
        /// Command is not supported by the device.
        /// </summary>
        NOT_IMPLEMENTED = 0x01,
        /// <summary>
        /// At least one parameter provided in the command is invalid (or out of range) for the device.
        /// </summary>
        INVALID_PARAMETER = 0x02,
        /// <summary>
        /// An attempt was made to access a non-existent address space location.
        /// </summary>
        INVALID_ADDRESS = 0x03,
        /// <summary>
        /// The addressed register cannot be written to.
        /// </summary>
        WRITE_PROTECT = 0x04,
        /// <summary>
        /// A badly aligned address offset or data size was specified.
        /// </summary>
        BAD_ALIGNMENT = 0x05,
        /// <summary>
        /// An attempt was made to access an address location which is currently/momentary not accessible.
        /// This depends on the current state of the device, in particular the current privilege of the application.
        /// </summary>
        ACCESS_DENIED = 0x06,
        /// <summary>
        /// A required resource to service the request is not currently available. The request may be retried at a later time.
        /// </summary>
        BUSY = 0x07,
        /// <summary>
        /// The requested packet is not available anymore.
        /// </summary>
        PACKET_UNAVAILABLE = 0x0C,
        /// <summary>
        /// Internal memory of GVSP transmitteroverrun (typically for image acquisition).
        /// </summary>
        DATA_OVERRUN = 0x0D,
        /// <summary>
        /// The message header is not valid. Some of its fields do not match the specification.
        /// </summary>
        INVALID_HEADER = 0x0E,
        /// <summary>
        /// Generic error. Try to avoid and use a more descriptive status code from list above.
        /// </summary>
        ERROR = 0xFF,
    }

    [Flags]
    public enum GigECapabilities
    {
        NONE = 0,
        CONCATENATION = 1 << 0,
        WRITE_MEMORY = 1 << 1,
        PACKET_RESEND = 1 << 2,
        EVENT = 1 << 3,
        EVENT_DATA = 1 << 4,
        PENDING_ACK = 1 << 5,
        ACTION = 1 << 6,
        IEEEE1588_EXTENDED = 1 << 16,
        SCHEDULED_ACTION = 1 << 17,
        STATUS_CODE = 1 << 18,
        CAPABILITY_1588 = 1 << 19,
        UNCONDITIONAL_ACTION = 1 << 20,
        PRIMARY_APPLICATION_SWITCHOVER = 1 << 21,
        EXTENDED_STATUS_CODES = 1 << 22,
        DISCOVERY_ACK_DELAY_WRITABLE = 1 << 23,
        DISCOVERY_ACK_DELAY = 1 << 24,
        TEST_DATA = 1 << 25,
        MANIFEST_TABLE = 1 << 26,
        CCP_APPLICATION_SOCKET = 1 << 27,
        LINK_SPEED = 1 << 28,
        HEARTBEAT_DISABLE = 1 << 29,
        SERIAL_NUMBER = 1 << 30,
        NAME_REGISTER = 1 << 31,
    }

    [Flags]
    public enum GigEIPConfiguration : uint
    {
        INVALID = 0,
        STATIC = 1u << 0,
        DHCP = 1u << 1,
        LLA = 1u << 2,

        PAUSE_GEN = 1u << 62,
        PAUSE = 1u << 63,
    }

    [Flags]
    public enum GigEPacketFlags : byte
    {
        NONE = 0,
        ACK_REQUIRED = 1 << 0,
        EXTENDED_ID = 1 << 4,
        ALLOW_BROADCAST_ACK = 1 << 4,
    }

    public readonly struct GigEPacket
    {
        public const byte SizeOfHeader = 8;

        public readonly GigEPacketType Type;
        public readonly GigEPacketFlags Flags;

        public readonly GigECommand Command;
        public readonly ushort Size;
        public readonly ushort ID;

        public readonly ArraySegment<byte> Data;

        public GigEPacket(GigEPacketType type, GigEPacketFlags flags, GigECommand command, ushort id, ushort dataSize) : this(type, flags, command, dataSize, id, new ArraySegment<byte>()) { }
        public GigEPacket(GigEPacketType type, GigEPacketFlags flags, GigECommand command, ushort id, ArraySegment<byte> data) : this(type, flags, command, (ushort)data.Count, id, data) { }
        public GigEPacket(GigEPacketType type, GigEPacketFlags flags, GigECommand command, ushort id) : this(type, flags, command, 0, id, new ArraySegment<byte>()) { }
        public GigEPacket(GigEPacketType type, GigEPacketFlags flags, GigECommand command, ushort size, ushort id, ArraySegment<byte> data)
        {
            Type = type;
            Flags = flags;
            Command = command;
            Size = size;
            ID = id;
            Data = data;
        }

        public GigEPacket(ref ArraySegmentReader reader)
        {
            Type = (GigEPacketType)reader.ReadUInt8();
            Flags = (GigEPacketFlags)reader.ReadUInt8();

            Command = (GigECommand)reader.ReadUInt16BE();
            Size = reader.ReadUInt16BE();
            ID = reader.ReadUInt16BE();

            Data = reader.Read(Size);
        }

        public ArraySegment<byte> WriteTo(byte[] destination) { return WriteTo(new ArraySegment<byte>(destination)); }
        public ArraySegment<byte> WriteTo(ArraySegment<byte> destination)
        {
            ArraySegmentWriter writer = new ArraySegmentWriter(destination);
            int written = WriteTo(ref writer);
            return new ArraySegment<byte>(destination.Array, destination.Offset, written);
        }

        public int WriteTo(ref ArraySegmentWriter writer)
        {
            int written = 0;
            written += writer.WriteUInt8((byte)Type);
            written += writer.WriteUInt8((byte)Flags);
            written += writer.WriteUInt16BE((ushort)Command);
            written += writer.WriteUInt16BE((ushort)Size);
            written += writer.WriteUInt16BE((ushort)ID);

            if (Data.Count > 0)
            {
                written += writer.WriteBytes(Data);
            }

            return written;
        }
    }

    public readonly struct GigEDevice
    {
        const int MANUFACTURER_NAME_SIZE = 32;
        const int MODEL_NAME_SIZE = 32;
        const int DEVICE_VERSION_SIZE = 32;
        const int MANUFACTURER_INFORMATIONS_SIZE = 48;
        const int SERIAL_NUMBER_SIZE = 16;
        const int USER_DEFINED_NAME_SIZE = 16;

        public readonly GigEIPConfiguration SupportedIPConfiguration;
        public readonly GigEIPConfiguration CurrentIPConfiguration;

        public readonly IPAddress Address;
        public readonly IPAddress SubnetMask;
        public readonly IPAddress DefaultGateway;

        public readonly HardwareAddress HardwareAddress;

        public readonly string Vendor;
        public readonly string Model;
        public readonly string Version;
        public readonly string ManufacturerInformation;
        public readonly string Serial;
        public readonly string UserInformation;

        public readonly IPAdapter Adapter;

        public GigEDevice(IPAdapter adapter, GigEPacket packet)
        {
            Adapter = adapter;

            ArraySegmentReader reader = new ArraySegmentReader(packet.Data);

            reader.Read(8); // skip 4 + 4 flags

            reader.Read(2); // skip first 2 bytes of MAC
            HardwareAddress = new HardwareAddress(reader.Read(6));

            SupportedIPConfiguration = (GigEIPConfiguration)reader.ReadUInt32BE();
            CurrentIPConfiguration = (GigEIPConfiguration)reader.ReadUInt32BE();

            reader.Read(12);
            Address = new IPAddress(reader.ReadUInt32());
            reader.Read(12);
            SubnetMask = new IPAddress(reader.ReadUInt32());
            reader.Read(12);
            DefaultGateway = new IPAddress(reader.ReadUInt32());

            Vendor = reader.ReadASCIIString(MANUFACTURER_NAME_SIZE);
            Model = reader.ReadASCIIString(MODEL_NAME_SIZE);
            Version = reader.ReadASCIIString(DEVICE_VERSION_SIZE);
            ManufacturerInformation = reader.ReadASCIIString(MANUFACTURER_INFORMATIONS_SIZE);
            Serial = reader.ReadASCIIString(SERIAL_NUMBER_SIZE);
            UserInformation = reader.ReadASCIIString(USER_DEFINED_NAME_SIZE);
        }

        public override string ToString() { return $"{Address} | {Vendor} | {Model}"; }
    }
*/
