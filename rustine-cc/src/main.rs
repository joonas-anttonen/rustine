mod network;

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use network::IPAdapter;
// use std::os::unix::io::FromRawFd;
// use libc;

/*
static GigEDevice[] Discover()
{
    List<GigEDevice> discoveredDevices = new List<GigEDevice>();

    lock (discoveryBuffer)
    {
        IPAdapter[] adapters = IPAdapter.GetAdapters();
        for (int i = 0; i < adapters.Length; i++)
        {
            IPAdapter adapter = adapters[i];

            try
            {
                using (Socket socket = new Socket(adapter.Address.AddressFamily, SocketType.Dgram, ProtocolType.Udp))
                {
                    socket.EnableBroadcast = true;
                    socket.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, 1);
                    socket.Bind(new IPEndPoint(adapter.Address, 0));

                    GigEPacket packet = new GigEPacket(GigEPacketType.CMD, GigEPacketFlags.ACK_REQUIRED | GigEPacketFlags.ALLOW_BROADCAST_ACK, GigECommand.DISCOVERY_CMD, ushort.MaxValue);
                    ArraySegment<byte> packetSegment = packet.WriteTo(discoveryBuffer);

                    // send discovery broadcast
                    socket.SendTo(discoveryBuffer, packetSegment.Offset, packetSegment.Count, SocketFlags.None, DiscoveryEndpoint);

                    bool shouldStop = false;

                    while (!shouldStop)
                    {
                        shouldStop = !socket.Poll(Constants.DiscoveryTimeout, SelectMode.SelectRead);

                        if (!shouldStop)
                        {
                            EndPoint actualEndPoint = AnyEndpoint;
                            int received = socket.ReceiveFrom(discoveryBuffer, 0, discoveryBuffer.Length, SocketFlags.None, ref actualEndPoint);

                            ArraySegmentReader responseReader = new ArraySegmentReader(discoveryBuffer, received);
                            GigEPacket responsePacket = new GigEPacket(ref responseReader);

                            GigEDevice respondingDeviceInfo = new GigEDevice(adapter, responsePacket);
                            discoveredDevices.Add(respondingDeviceInfo);
                        }
                    }
                }
            }
            catch (SocketException) { }
        }
    }

    return discoveredDevices.ToArray();
}
*/

struct ByteSliceReader<'a>(&'a [u8], usize);

impl<'a> ByteSliceReader<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self(slice, 0)
    }

    pub fn read_u8(&mut self) -> u8 {
        let val = self.0[self.1];
        self.1 += 1;
        val
    }
    pub fn read_u16_le(&mut self) -> u16 {
        let low = self.read_u8() as u16;
        let high = self.read_u8() as u16;
        (high << 8) | low
    }
    pub fn read_u16_be(&mut self) -> u16 {
        let high = self.read_u8() as u16;
        let low = self.read_u8() as u16;
        (high << 8) | low
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
    UNKNOWN(u16),
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
            _ => GigECommand::UNKNOWN(u),
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
            GigECommand::UNKNOWN(u) => *u,
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

    pub fn from_slice(slice: &'a [u8]) -> Self {
        let mut reader = ByteSliceReader::new(slice);

        let t = GigEPacketType::from_u8(reader.read_u8());
        let flags = GigEPacketFlags(reader.read_u8());

        let command = GigECommand::from_u16(reader.read_u16_be());
        let size = reader.read_u16_be();
        let id = reader.read_u16_be();

        let data = &slice[8..];

        Self {
            t,
            flags,
            command,
            size,
            id,
            data,
        }
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

const GVCP_PORT: u16 = 3956;
const GVCP_BROADCAST_ADDR: SocketAddrV4 =
    SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), GVCP_PORT);

fn main() {
    let adapters: Vec<IPAdapter> = IPAdapter::get_adapters();
    if adapters.is_empty() {
        println!("No ethernet IPv4 adapters found or `ip` command missing.");
    } else {
        for a in adapters.iter() {
            if let Some(hw) = &a.hwaddr {
                println!(
                    "{} -> {} mask {} mtu {} hw {}",
                    a.name, a.address, a.address_mask, a.mtu, hw
                );
            } else {
                println!(
                    "{} -> {} mask {} mtu {}",
                    a.name, a.address, a.address_mask, a.mtu
                );
            }

            // Bind a safe `UdpSocket` to the adapter address on an ephemeral port
            {
                use std::net::{SocketAddrV4, UdpSocket};

                let bind_addr = SocketAddrV4::new(a.address, 0);

                match UdpSocket::bind(bind_addr) {
                    Ok(socket) => {
                        socket
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();

                        if let Err(e) = socket.set_broadcast(true) {
                            eprintln!("set_broadcast failed: {}", e);
                        }

                        match socket.local_addr() {
                            Ok(sa) => println!("Bound UDP socket to {} (adapter {})", sa, a.name),
                            Err(e) => eprintln!("local_addr error: {}", e),
                        }

                        let mut request_id = u16::MAX;

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

                        //for _ in 0..1000 {
                            if let Err(e) = socket.send_to(&buf, GVCP_BROADCAST_ADDR) {
                                eprintln!("send_to error: {}", e);
                            }
                        //}

                        match socket.recv_from(&mut buf) {
                            Ok((read_bytes, _src)) => {
                                println!("Received {} bytes", read_bytes);
                            }
                            Err(e) => {
                                eprintln!("recv_from error: {}", e);
                            }
                        }

                        let camera_addr =
                            SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 11), GVCP_PORT);

                        request_id = request_id.wrapping_add(1);
                        let cmd_packet = GigEPacket::new(
                            GigEPacketType::CMD,
                            GigEPacketFlags::ACK_REQUIRED,
                            GigECommand::READ_REGISTER_CMD,
                            request_id,
                            0,
                            &[],
                        );
                        cmd_packet.to_slice(&mut buf);

                        if let Err(e) = socket.send_to(&buf, camera_addr) {
                            eprintln!("send_to error: {}", e);
                        }

                        match socket.recv_from(&mut buf) {
                            Ok((read_bytes, _src)) => {
                                println!("Received {} bytes", read_bytes);
                            }
                            Err(e) => {
                                eprintln!("recv_from error: {}", e);
                            }
                        }

                        // socket will be closed when it goes out of this scope
                    }
                    Err(e) => {
                        eprintln!("failed to bind UDP socket to {}: {}", bind_addr, e);
                    }
                }
            }
        }
    }
}
