#![allow(dead_code)]

use std::{
    net::{SocketAddrV4, UdpSocket},
    sync::{Arc, Mutex, atomic},
};

use crate::gige::{
    GVCP_CAPABILITIES_REGISTER, GVCP_CONTROL_ACCESS_REGISTER, GVCP_HEARTBEAT_TIMEOUT_REGISTER,
    GVCP_PORT, GigECapabilities, GigECommand, GigEDevice, GigEPacket, GigEPacketFlags,
    GigEPacketType, REQUEST_ID,
};

use rustine::{Mailbox, io::ByteSliceReader, log};

pub enum ClientCommand {
    NOP,
    STARTUP,
    SHUTDOWN,
}

struct Connection {
    pub socket: UdpSocket,
    pub send_address: SocketAddrV4,
    pub recv_address: SocketAddrV4,
}

pub struct GigEClient {
    device: GigEDevice,
    command_queue: Arc<rustine::Mailbox<ClientCommand>>,
    stream_thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for GigEClient {
    fn drop(&mut self) {
        // Cleanup code here
    }
}

impl GigEClient {
    pub fn new(device: GigEDevice) -> Self {
        GigEClient {
            device,
            command_queue: rustine::Mailbox::new(Arc::new(rustine::AutoResetEvent::new())),
            stream_thread: None,
        }
    }

    pub fn queue(&self) -> Arc<rustine::Mailbox<ClientCommand>> {
        Arc::clone(&self.command_queue)
    }

    pub fn run(
        client: Arc<Mutex<GigEClient>>,
        _image_mailbox: Arc<Mailbox<(u32, rustine::io::Image)>>,
    ) {
        {
            let (device_model, device_serial) = {
                let client = client.lock().unwrap();
                (client.device.model.clone(), client.device.serial.clone())
            };
            log::set_current_thread_name(format!("gige::{}_{}", device_model, device_serial));
        }

        //loop {

        // Use hardcoded register addresses for acquisition control for now.
        const HACK_ACQUISITION_START_REGISTER_ADDRESS: u32 = 0x10300004;
        const HACK_ACQUISITION_STOP_REGISTER_ADDRESS: u32 = 0x10300008;
        const HACK_GEV_SCDA: u32 = 0x0D18;
        const HACK_GEV_SCPHOST_PORT: u32 = 0x0D00;
        const GVSP_RECV_PORT: u16 = 31896;

        let (adapter_address, device_address) = {
            let client = client.lock().unwrap();
            (client.device.adapter.address, client.device.address)
        };

        let socket =
            UdpSocket::bind(SocketAddrV4::new(adapter_address, 0)).expect("Failed to bind socket");
        let recv_address = SocketAddrV4::new(adapter_address, socket.local_addr().unwrap().port());
        let send_address = SocketAddrV4::new(device_address, GVCP_PORT);
        socket
            .connect(send_address)
            .expect("Failed to connect socket");

        let control_connection = Connection {
            socket,
            send_address,
            recv_address,
        };

        let capabilities = Self::read_register(&control_connection, GVCP_CAPABILITIES_REGISTER)
            .expect("Failed to read capabilities");
        let timeout = Self::read_register(&control_connection, GVCP_HEARTBEAT_TIMEOUT_REGISTER)
            .expect("Failed to read heartbeat timeout");

        log::info!("{:#?}", GigECapabilities::from_u32(capabilities));
        log::info!("Timeout: {}", timeout);

        Self::write_register(&control_connection, GVCP_CONTROL_ACCESS_REGISTER, 1)
            .expect("Failed to write control access register");

        Self::write_register(&control_connection, HACK_GEV_SCDA, adapter_address.into())
            .expect("Failed to write GEV_SCDA register");

        // Careful: HACK_GEV_SCPHOST_PORT register is a big endian struct register with the port in the upper 16 bits
        Self::write_register(
            &control_connection,
            HACK_GEV_SCPHOST_PORT,
            GVSP_RECV_PORT.into(),
        )
        .expect("Failed to write GEV_SCPHOST_PORT register");

        let stream_recv_address = SocketAddrV4::new(adapter_address, GVSP_RECV_PORT);
        let stream_socket =
            UdpSocket::bind(stream_recv_address).expect("Failed to bind stream socket");
        let _stream_connection = Connection {
            socket: stream_socket,
            send_address: SocketAddrV4::new(device_address, GVCP_PORT),
            recv_address: stream_recv_address,
        };

        Self::write_register(
            &control_connection,
            HACK_ACQUISITION_START_REGISTER_ADDRESS,
            1,
        )
        .expect("Failed to write acquisition start register");

        std::thread::sleep(std::time::Duration::from_secs(2));

        Self::write_register(
            &control_connection,
            HACK_ACQUISITION_STOP_REGISTER_ADDRESS,
            1,
        )
        .expect("Failed to write acquisition stop register");

        Self::write_register(&control_connection, GVCP_CONTROL_ACCESS_REGISTER, 0)
            .expect("Failed to write control access register");

        //}
    }

    fn read_register(connection: &Connection, address: u32) -> std::io::Result<u32> {
        let request_packet_data = address.to_be_bytes();
        let request_packet = GigEPacket::new(
            GigEPacketType::CMD,
            GigEPacketFlags::ACK_REQUIRED,
            GigECommand::READ_REGISTER_CMD,
            REQUEST_ID.fetch_add(1, atomic::Ordering::Relaxed),
            &request_packet_data,
        );

        let mut buf = [0u8; 1500];
        let response_len = Self::send_cmd_recv_ack(connection, &request_packet, &mut buf)?;
        let response_packet = GigEPacket::from_slice(&buf[..response_len])?;

        let mut response_data_reader = ByteSliceReader::new(response_packet.data);

        let register_value = response_data_reader.read_u32_be()?;
        Ok(register_value)
    }

    fn write_register(connection: &Connection, address: u32, value: u32) -> std::io::Result<()> {
        let mut data = [0u8; 8];
        data[..4].copy_from_slice(&address.to_be_bytes());
        data[4..].copy_from_slice(&value.to_be_bytes());

        let request_packet = GigEPacket::new(
            GigEPacketType::CMD,
            GigEPacketFlags::ACK_REQUIRED,
            GigECommand::WRITE_REGISTER_CMD,
            REQUEST_ID.fetch_add(1, atomic::Ordering::Relaxed),
            &data,
        );

        let mut buf = [0u8; 1500];
        Self::send_cmd_recv_ack(connection, &request_packet, &mut buf)?;
        Ok(())
    }

    fn send_cmd_recv_ack(
        connection: &Connection,
        send_packet: &GigEPacket,
        buf: &mut [u8],
    ) -> std::io::Result<usize> {
        const MAX_RETRIES: usize = 3;
        let mut retries = 0;

        let send_len = send_packet.to_slice(buf)?;

        connection
            .socket
            .send_to(&buf[..send_len], connection.send_address)?;
        connection
            .socket
            .set_read_timeout(Some(std::time::Duration::from_millis(100)))?;

        loop {
            if let Ok(recv_len) = connection.socket.recv(buf) {
                let recv_packet = GigEPacket::from_slice(&buf[..recv_len])?;

                let is_ack = recv_packet.t == GigEPacketType::ACK;
                let is_same_request = send_packet.id == recv_packet.id;

                if is_ack && is_same_request {
                    return Ok(recv_len);
                }

                // Oh no, received a packet that is not an ACK
                // or does not match the request ID, try receiving again
            }

            retries += 1;
            if retries >= MAX_RETRIES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Maximum retries reached",
                ));
            }
        }
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn start_stream(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
