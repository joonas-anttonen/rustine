#![allow(dead_code)]

use std::{
    net::{SocketAddrV4, UdpSocket},
    sync::{Arc, Mutex, atomic},
};

use crate::gige::{
    GVCP_CAPABILITIES_REGISTER, GVCP_CONTROL_ACCESS_REGISTER, GVCP_HEARTBEAT_TIMEOUT_REGISTER,
    GVCP_PORT, GVSPFormat, GVSPPacketStatus, GVSPPayloadType, GVSPPixelFormat, GigECapabilities,
    GigECommand, GigEDevice, GigEPacket, GigEPacketFlags, GigEPacketType, REQUEST_ID,
};

use rustine::{Mailbox, io::ByteSliceReader, log};

pub enum ClientCommand {
    NOP,
    STARTUP,
    SHUTDOWN,
}

struct Connection {
    pub socket: UdpSocket,
    pub remote_address: SocketAddrV4,
    pub local_address: SocketAddrV4,
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
        image: Arc<rustine::gfx::Image>,
        image_mailbox: Arc<Mailbox<(u32, rustine::io::Image)>>,
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
        const GVSP_RECV_PORT: u16 = 62078;

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
            remote_address: send_address,
            local_address: recv_address,
        };

        let capabilities = Self::read_register(&control_connection, GVCP_CAPABILITIES_REGISTER)
            .expect("Failed to read capabilities");
        let timeout = Self::read_register(&control_connection, GVCP_HEARTBEAT_TIMEOUT_REGISTER)
            .expect("Failed to read heartbeat timeout");
        let timeout = timeout / 2;

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

        let stream_local_address = SocketAddrV4::new(adapter_address, GVSP_RECV_PORT);
        let stream_remote_address = SocketAddrV4::new(device_address, 0);
        let stream_socket =
            UdpSocket::bind(stream_local_address).expect("Failed to bind stream socket");
        stream_socket
            .connect(stream_remote_address)
            .expect("Failed to connect stream socket");
        log::info!("Stream socket connected to {}", stream_local_address);
        stream_socket
            .set_read_timeout(Some(std::time::Duration::from_millis(1000)))
            .expect("Failed to set read timeout");

        let stream_connection = Connection {
            socket: stream_socket,
            remote_address: stream_remote_address,
            local_address: stream_local_address,
        };

        Self::write_register(
            &control_connection,
            HACK_ACQUISITION_START_REGISTER_ADDRESS,
            1,
        )
        .expect("Failed to write acquisition start register");

        let mut buf = [0u8; 1800];

        let mut frame_buf: Vec<u8> = Vec::new();

        let mut _current_frame_start_time = std::time::Instant::now();
        let mut current_frame_id: Option<u64> = None;
        let mut current_packet_id = 0;
        let mut data_per_packet: Option<usize> = None;
        let mut frame_width = 0u32;
        let mut frame_height = 0u32;
        let mut frame_pixel_format = GVSPPixelFormat::BAYER_RG_8;

        let mut heartbeat_time = std::time::Instant::now();

        loop {
            if std::time::Instant::now().saturating_duration_since(heartbeat_time)
                >= std::time::Duration::from_millis(timeout as u64)
            {
                heartbeat_time = std::time::Instant::now();
                Self::read_register(&control_connection, GVCP_HEARTBEAT_TIMEOUT_REGISTER)
                    .expect("Failed to read heartbeat timeout");
            }

            if let Ok(recv_len) = stream_connection.socket.recv(&mut buf) {
                let mut packet_reader = ByteSliceReader::new(&buf[..recv_len]);

                let _packet_status =
                    GVSPPacketStatus::from_u16(packet_reader.read_u16_be().unwrap());

                let packet_block = packet_reader.read_u16_be().unwrap();
                let packet_info = packet_reader.read_u32_be().unwrap();
                let packet_format = GVSPFormat::from_u8(((packet_info & 0x7F000000) >> 24) as u8);

                let (packet_frame_id, packet_id) = {
                    let has_extended_ids = (packet_info & 0x80000000) != 0;
                    if has_extended_ids {
                        let frame_id = packet_reader.read_u64_be().unwrap();
                        let packet_id = packet_reader.read_u32_be().unwrap();
                        (frame_id, packet_id as usize)
                    } else {
                        let frame_id = packet_block as u64;
                        let packet_id = (packet_info & 0x00ffffff) as u32;
                        (frame_id, packet_id as usize)
                    }
                };

                match packet_format {
                    GVSPFormat::LEADER => {
                        // Start of a new frame
                        if let Some(current_frame_id) = current_frame_id {
                            log::warning!("LEADER before TRAILER: {}", current_frame_id);
                        }

                        current_frame_id = Some(packet_frame_id);
                        current_packet_id = packet_id;
                        _current_frame_start_time = std::time::Instant::now();
                        data_per_packet = None;
                        frame_buf.fill(0xFE);

                        let _flags = packet_reader.read_u16_be().unwrap();
                        let _payload_type =
                            GVSPPayloadType::from_u16(packet_reader.read_u16_be().unwrap());
                        let _timestamp_high = packet_reader.read_u32_be().unwrap();
                        let _timestamp_low = packet_reader.read_u32_be().unwrap();
                        frame_pixel_format =
                            GVSPPixelFormat::from_u32(packet_reader.read_u32_be().unwrap());
                        frame_width = packet_reader.read_u32_be().unwrap();
                        frame_height = packet_reader.read_u32_be().unwrap();
                        let _x_offset = packet_reader.read_u32_be().unwrap();
                        let _y_offset = packet_reader.read_u32_be().unwrap();

                        let frame_buf_size = (frame_width
                            * frame_height
                            * (((frame_pixel_format.to_u32() >> 16) & 0xff) / 8))
                            as usize;

                        if frame_buf.len() != frame_buf_size {
                            log::warning!(
                                "Resizing pixel buffer: {} -> {}",
                                frame_buf.len(),
                                frame_buf_size
                            );
                            frame_buf.resize(frame_buf_size, 0);
                        }
                    }
                    GVSPFormat::TRAILER => {
                        if let Some(current_frame_id) = current_frame_id
                            && current_frame_id != packet_frame_id
                        {
                            log::warning!("TRAILER before LEADER: {}", packet_frame_id);
                        } else {
                            let packet_id_diff = packet_id - current_packet_id;
                            if packet_id_diff > 1 {
                                log::warning!(
                                    "Packet ID gap detected: {} -> {} frame: {}",
                                    current_packet_id,
                                    packet_id,
                                    packet_frame_id
                                );
                            }
                            current_packet_id = packet_id;

                            // Frame complete
                            let io_image = rustine::io::Image {
                                width: frame_width,
                                height: frame_height,
                                format: rustine::gfx::Format::R8G8B8A8_UNORM,
                                pixels: Self::convert_pixels(
                                    frame_buf.as_slice(),
                                    frame_width as usize,
                                    frame_height as usize,
                                    frame_pixel_format,
                                    rustine::gfx::Format::R8G8B8A8_UNORM,
                                ), // TODO: Giga allocation!
                            };

                            image_mailbox.push((image.id(), io_image));
                        }

                        current_frame_id = None;
                        data_per_packet = None;
                    }
                    GVSPFormat::PAYLOAD => {
                        let packet_id_diff = packet_id - current_packet_id;
                        if packet_id_diff > 1 {
                            log::warning!(
                                "Packet ID gap detected: {} -> {} frame: {}",
                                current_packet_id,
                                packet_id,
                                packet_frame_id
                            );
                        }
                        current_packet_id = packet_id;

                        let payload_data_slice = packet_reader.read_to_end().unwrap();

                        if data_per_packet.is_none() {
                            data_per_packet = Some(payload_data_slice.len());
                        }

                        let write_head = (packet_id - 1) * data_per_packet.unwrap();
                        frame_buf[write_head..write_head + payload_data_slice.len()]
                            .copy_from_slice(payload_data_slice);
                    }
                    _ => {
                        log::error!("Unknown packet format: {:?}", packet_format);
                        break;
                    }
                }

                continue;
            }

            log::error!("Failed to receive stream data");
            break;
        }

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

    fn convert_pixels(
        in_buf: &[u8],
        in_width: usize,
        in_height: usize,
        in_format: GVSPPixelFormat,
        out_format: rustine::gfx::Format,
    ) -> Vec<u8> {
        // Helper: simple bilinear demosaic for RGGB (BAYER_RG_8)
        fn demosaic_bayer_rg8(in_buf: &[u8], width: usize, height: usize) -> Vec<u8> {
            let mut out = vec![0u8; width * height * 4];

            let get_b = |x: isize, y: isize| -> Option<u8> {
                if x < 0 || y < 0 {
                    return None;
                }
                let ux = x as usize;
                let uy = y as usize;
                if ux >= width || uy >= height {
                    None
                } else {
                    let v = in_buf[uy * width + ux];
                    if v == 0xFE {
                        None
                    } else {
                        Some(v)
                    }
                }
            };

            for y in 0..height {
                for x in 0..width {


                    let px = x as isize;
                    let py = y as isize;

                    let mut r_sum: u32 = 0;
                    let mut g_sum: u32 = 0;
                    let mut b_sum: u32 = 0;
                    let mut r_cnt = 0u32;
                    let mut g_cnt = 0u32;
                    let mut b_cnt = 0u32;

                    // Assume BAYER_RG_8 == RGGB pattern:
                    // even row, even col: R
                    // even row, odd col:  G
                    // odd row, even col:  G
                    // odd row, odd col:   B
                    let is_row_even = (y % 2) == 0;
                    let is_col_even = (x % 2) == 0;

                    let center = get_b(px, py).unwrap_or(0) as u32;

                    if is_row_even && is_col_even {
                        // R pixel
                        r_sum += center;
                        r_cnt += 1;
                        // G: average of up/down/left/right
                        for (nx, ny) in &[(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                g_sum += v as u32;
                                g_cnt += 1;
                            }
                        }
                        // B: average of diagonals
                        for (nx, ny) in &[
                            (px - 1, py - 1),
                            (px + 1, py - 1),
                            (px - 1, py + 1),
                            (px + 1, py + 1),
                        ] {
                            if let Some(v) = get_b(*nx, *ny) {
                                b_sum += v as u32;
                                b_cnt += 1;
                            }
                        }
                    } else if is_row_even && !is_col_even {
                        // G pixel on R row
                        g_sum += center;
                        g_cnt += 1;
                        // R: average left/right
                        for (nx, ny) in &[(px - 1, py), (px + 1, py)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                r_sum += v as u32;
                                r_cnt += 1;
                            }
                        }
                        // B: average up/down
                        for (nx, ny) in &[(px, py - 1), (px, py + 1)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                b_sum += v as u32;
                                b_cnt += 1;
                            }
                        }
                    } else if !is_row_even && is_col_even {
                        // G pixel on B row
                        g_sum += center;
                        g_cnt += 1;
                        // R: average up/down
                        for (nx, ny) in &[(px, py - 1), (px, py + 1)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                r_sum += v as u32;
                                r_cnt += 1;
                            }
                        }
                        // B: average left/right
                        for (nx, ny) in &[(px - 1, py), (px + 1, py)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                b_sum += v as u32;
                                b_cnt += 1;
                            }
                        }
                    } else {
                        // odd row, odd col -> B pixel
                        b_sum += center;
                        b_cnt += 1;
                        // G: average of up/down/left/right
                        for (nx, ny) in &[(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
                            if let Some(v) = get_b(*nx, *ny) {
                                g_sum += v as u32;
                                g_cnt += 1;
                            }
                        }
                        // R: average of diagonals
                        for (nx, ny) in &[
                            (px - 1, py - 1),
                            (px + 1, py - 1),
                            (px - 1, py + 1),
                            (px + 1, py + 1),
                        ] {
                            if let Some(v) = get_b(*nx, *ny) {
                                r_sum += v as u32;
                                r_cnt += 1;
                            }
                        }
                    }

                    let r = if r_cnt > 0 {
                        (r_sum / r_cnt) as u8
                    } else {
                        0u8
                    };
                    let g = if g_cnt > 0 {
                        (g_sum / g_cnt) as u8
                    } else {
                        0u8
                    };
                    let b = if b_cnt > 0 {
                        (b_sum / b_cnt) as u8
                    } else {
                        0u8
                    };

                    let base = (y * width + x) * 4;
                    out[base] = r;
                    out[base + 1] = g;
                    out[base + 2] = b;
                    out[base + 3] = 0xffu8;
                }
            }

            out
        }

        match (in_format, out_format) {
            (GVSPPixelFormat::BAYER_RG_8, rustine::gfx::Format::R8G8B8A8_UNORM) => {
                demosaic_bayer_rg8(in_buf, in_width, in_height)
            }
            (_, rustine::gfx::Format::R8G8B8A8_UNORM) => {
                // Unknown/unsupported input format: return black image of requested size
                vec![0u8; in_width * in_height * 4]
            }
            _ => todo!(),
        }
    }

    /*
    protected unsafe static void Bayer_Simple(byte* bayer, byte* rgba, int sx, int sy, PixelFormat format)
        {
            int bayerStep = sx;
            int rgbStep = 4 * sx;
            int width = sx;
            int height = sy;
            int blue = format == PixelFormat.Bayer_8_BGGR || format == PixelFormat.Bayer_8_GBRG ? -1 : 1;
            bool start_with_green = format == PixelFormat.Bayer_8_GBRG || format == PixelFormat.Bayer_8_GRBG;
            int i, imax, iinc;

            /* add black border */
            imax = sx * sy * 4;
            for (i = sx * (sy - 1) * 4; i < imax; i++)
            {
                rgba[i] = 0;
            }
            iinc = (sx - 1) * 4;
            for (i = (sx - 1) * 4; i < imax; i += iinc)
            {
                rgba[i++] = 0;
                rgba[i++] = 0;
                rgba[i++] = 0;
                rgba[i++] = 0;
            }

            rgba += 1;
            width -= 1;
            height -= 1;

            for (; height-- > 0; bayer += bayerStep, rgba += rgbStep)
            {
                byte* bayerEnd = bayer + width;

                if (start_with_green)
                {
                    rgba[-blue] = bayer[1];
                    rgba[0] = (byte)((bayer[0] + bayer[bayerStep + 1] + 1) >> 1);
                    rgba[blue] = bayer[bayerStep];
                    rgba[2] = 0xff;
                    bayer++;
                    rgba += 4;
                }

                if (blue > 0)
                {
                    for (; bayer <= bayerEnd - 2; bayer += 2, rgba += 8)
                    {
                        rgba[-1] = bayer[0];
                        rgba[0] = (byte)((bayer[1] + bayer[bayerStep] + 1) >> 1);
                        rgba[1] = bayer[bayerStep + 1];
                        rgba[2] = 0xff;

                        rgba[3] = bayer[2];
                        rgba[4] = (byte)((bayer[1] + bayer[bayerStep + 2] + 1) >> 1);
                        rgba[5] = bayer[bayerStep + 1];
                        rgba[6] = 0xff;
                    }
                }
                else
                {
                    for (; bayer <= bayerEnd - 2; bayer += 2, rgba += 8)
                    {
                        rgba[2] = 0xff;
                        rgba[1] = bayer[0];
                        rgba[0] = (byte)((bayer[1] + bayer[bayerStep] + 1) >> 1);
                        rgba[-1] = bayer[bayerStep + 1];

                        rgba[6] = 0xff;
                        rgba[5] = bayer[2];
                        rgba[4] = (byte)((bayer[1] + bayer[bayerStep + 2] + 1) >> 1);
                        rgba[3] = bayer[bayerStep + 1];
                    }
                }

                if (bayer < bayerEnd)
                {
                    rgba[-blue] = bayer[0];
                    rgba[0] = (byte)((bayer[1] + bayer[bayerStep] + 1) >> 1);
                    rgba[blue] = bayer[bayerStep + 1];
                    rgba[2] = 0xff;
                    bayer++;
                    rgba += 4;
                }

                bayer -= width;
                rgba -= width * 4;

                blue = -blue;
                start_with_green = !start_with_green;
            }
        } */

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

        connection.socket.send(&buf[..send_len])?;
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
