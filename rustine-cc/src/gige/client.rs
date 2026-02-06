#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{SocketAddrV4, UdpSocket};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex, atomic};

use crate::genicam;
use crate::gige::*;
use crate::mjpeg::*;

use rustine::{Mailbox, io::ByteSliceReader, log};

struct Connection {
    pub socket: UdpSocket,
    pub remote_address: SocketAddrV4,
    pub local_address: SocketAddrV4,
}

pub struct GigEClient {
    device: GigEDevice,
}

impl Drop for GigEClient {
    fn drop(&mut self) {}
}

fn unsupported<T>(msg: impl Into<String>) -> std::io::Result<T> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        msg.into(),
    ))
}

fn not_found_err(msg: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, msg.into())
}

fn invalid_data_err<TE: ToString>(err: TE) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
}

impl GigEClient {
    const CMD_MAX_RETRIES: usize = 3;
    const CMD_RECV_TIMEOUT_MS: u64 = 100;

    const STREAM_CHANNEL_RECV_BUFFER: usize = 4 * 1024 * 1024;
    const STREAM_CHANNEL_DESTINATION_ADDRESS: u32 = 0x0D18;
    const STREAM_CHANNEL_PORT_HOST: u32 = 0x0D00;
    const STREAM_CHANNEL_PACKET_SIZE: u32 = 0x0D04;

    const GVCP_XML_URL_SIZE: u32 = 512;
    const GVCP_XML_0_URL_ADDRESS: u32 = 0x00000200;

    pub fn new(device: GigEDevice) -> Self {
        GigEClient { device }
    }

    pub fn run(
        client: Arc<Mutex<GigEClient>>,
        image: Arc<rustine::gfx::Image>,
        image_mailbox: Arc<Mailbox<(u32, rustine::io::Image)>>,
        stream_cache: Option<Arc<FrameCache>>,
        exit_flag: &std::sync::atomic::AtomicBool,
    ) -> std::io::Result<()> {
        let (adapter, device_address) = {
            let client = client.lock().unwrap();

            log::set_current_thread_name(format!(
                "gige::{}_{}",
                client.device.model.clone(),
                client.device.serial.clone()
            ));

            (client.device.adapter.clone(), client.device.address)
        };

        let (control_connection, stream_connection) =
            Self::setup_sockets(&adapter, device_address)?;

        loop {
            if exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }

            match Self::run_connection(
                &image,
                &image_mailbox,
                &stream_cache,
                exit_flag,
                &adapter,
                &control_connection,
                &stream_connection,
            ) {
                Ok(()) => {}
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted => {
                        // Can try to reconnect if only a mild error -> fall through
                    }
                    _ => return Err(e),
                },
            }
        }
    }

    fn run_connection(
        image: &Arc<rustine::gfx::Image>,
        image_mailbox: &Arc<Mailbox<(u32, rustine::io::Image)>>,
        stream_cache: &Option<Arc<FrameCache>>,
        exit_flag: &atomic::AtomicBool,
        adapter: &IPAdapter,
        control_connection: &Connection,
        stream_connection: &Connection,
    ) -> std::io::Result<()> {
        Self::write_register(control_connection, GVCP_CONTROL_ACCESS_REGISTER, 1)?;

        let genicam = {
            let xml_url = Self::read_memory(
                control_connection,
                Self::GVCP_XML_0_URL_ADDRESS,
                Self::GVCP_XML_URL_SIZE,
            )?;

            // Trim trailing null bytes
            let xml_url = if let Some(pos) = xml_url.iter().position(|&b| b == 0) {
                &xml_url[..pos]
            } else {
                &xml_url
            };

            let xml_url = str::from_utf8(xml_url).map_err(invalid_data_err)?;

            // "Local:xxxx.zip;800f3374;12a5d"
            // Filename;Address;Size
            let xml_url_parts: Vec<&str> = xml_url.split(';').collect();
            //let xml_filename = xml_url_parts.get(0).unwrap_or(&"");
            let xml_address = xml_url_parts.get(1).unwrap_or(&"");
            let xml_size = xml_url_parts.get(2).unwrap_or(&"");

            let memory_address = u32::from_str_radix(xml_address, 16).map_err(invalid_data_err)?;
            let memory_size = u32::from_str_radix(xml_size, 16).map_err(invalid_data_err)?;

            let xml_archive_bytes =
                Self::read_memory(control_connection, memory_address, memory_size)?;

            let mut xml_archive = zip::ZipArchive::new(std::io::Cursor::new(xml_archive_bytes))?;
            let mut file = xml_archive.by_index(0)?;
            let mut xml_string = String::new();
            std::io::Read::read_to_string(&mut file, &mut xml_string)?;
            genicam::parse(&xml_string)?
        };

        let acquisition_start_cmd = genicam
            .get_command_by_name("AcquisitionStart")
            .ok_or_else(|| not_found_err("AcquisitionStart"))?;
        let acquisition_stop_cmd = genicam
            .get_command_by_name("AcquisitionStop")
            .ok_or_else(|| not_found_err("AcquisitionStop"))?;

        let frame_rate = genicam
            .get_feature_by_name("AcquisitionFrameRate")
            .ok_or_else(|| not_found_err("AcquisitionFrameRate"))?;
        log::warning!(
            "Acquisition FPS: {:?} {}",
            Self::read_number(control_connection, frame_rate)?,
            frame_rate.get_unit().unwrap_or("")
        );

        let exposure_time = genicam
            .get_feature_by_name("ExposureTime")
            .ok_or_else(|| not_found_err("ExposureTime"))?;
        log::warning!(
            "Exposure Time: {:?} {}",
            Self::read_number(control_connection, exposure_time)?,
            exposure_time.get_unit().unwrap_or("")
        );

        let exposure_auto = genicam
            .get_enumeration_by_name("ExposureAuto")
            .ok_or_else(|| not_found_err("ExposureAuto"))?;
        let exposure_auto_limit = genicam
            .get_enumeration_by_name("ExposureAutoLimitAuto")
            .ok_or_else(|| not_found_err("ExposureAutoLimitAuto"))?;
        let exposure_target_brightness = genicam
            .get_feature_by_name("TargetBrightness")
            .ok_or_else(|| not_found_err("TargetBrightness"))?;

        Self::write_number(control_connection, exposure_target_brightness, 64.0)?;
        Self::write_enumeration(control_connection, exposure_auto, "Continuous")?;

        log::warning!(
            "Exposure Auto: {:?} Limit: {:?} Target Brightness: {:?}",
            Self::read_enumeration(control_connection, exposure_auto)?,
            Self::read_enumeration(control_connection, exposure_auto_limit)?,
            Self::read_number(control_connection, exposure_target_brightness)?
        );

        let gain = genicam
            .get_feature_by_name("Gain")
            .ok_or_else(|| not_found_err("Gain"))?;
        let gain_auto = genicam
            .get_enumeration_by_name("GainAuto")
            .ok_or_else(|| not_found_err("GainAuto"))?;

        Self::write_enumeration(control_connection, gain_auto, "Continuous")?;

        log::warning!(
            "Gain: {:?} {} Auto: {:?}",
            Self::read_number(control_connection, gain)?,
            gain.get_unit().unwrap_or(""),
            Self::read_enumeration(control_connection, gain_auto)?
        );

        let heartbeat_timeout =
            Self::read_register(control_connection, GVCP_HEARTBEAT_TIMEOUT_REGISTER)?;
        let control_timeout = std::time::Duration::from_millis(heartbeat_timeout as u64 / 2);
        {
            Self::write_register(
                control_connection,
                Self::STREAM_CHANNEL_DESTINATION_ADDRESS,
                adapter.address.into(),
            )?;
            Self::write_register(
                control_connection,
                Self::STREAM_CHANNEL_PACKET_SIZE,
                adapter.mtu.min(9000),
            )?;
            Self::write_register(
                control_connection,
                Self::STREAM_CHANNEL_PORT_HOST,
                stream_connection.local_address.port().into(),
            )?;
        }
        Self::issue_command(control_connection, acquisition_start_cmd)?;
        Self::run_acquisition(
            image,
            image_mailbox,
            stream_cache,
            control_connection,
            control_timeout,
            stream_connection,
            exit_flag,
            &genicam,
        )?;
        Self::issue_command(control_connection, acquisition_stop_cmd)?;
        Self::write_register(control_connection, GVCP_CONTROL_ACCESS_REGISTER, 0)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_acquisition(
        image: &Arc<rustine::gfx::Image>,
        image_mailbox: &Arc<Mailbox<(u32, rustine::io::Image)>>,
        stream_cache: &Option<Arc<FrameCache>>,
        control_connection: &Connection,
        timeout: std::time::Duration,
        stream_connection: &Connection,
        exit_flag: &std::sync::atomic::AtomicBool,
        genicam: &genicam::GenICam,
    ) -> std::io::Result<()> {
        let mut buf = [0u8; 10_000];

        let mut frame_buf: Vec<u8> = Vec::new();

        let mut current_frame_start_time = std::time::Instant::now();
        let mut current_frame_id: Option<u64> = None;
        let mut current_packet_id = 0;
        let mut data_per_packet: Option<usize> = None;
        let mut frame_width = 0u32;
        let mut frame_height = 0u32;
        let mut frame_pixel_format = GVSPPixelFormat::BAYER_RG_8;

        let mut heartbeat_time = std::time::Instant::now();

        let mut frame_receive_times = rustine::RingBuffer::<f64>::new(100);
        let mut demosaic_times = rustine::RingBuffer::<f64>::new(100);

        loop {
            if exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }

            if std::time::Instant::now().saturating_duration_since(heartbeat_time) >= timeout {
                heartbeat_time = std::time::Instant::now();
                Self::read_register(control_connection, GVCP_HEARTBEAT_TIMEOUT_REGISTER)?;

                if let Some((min, max, mean)) = demosaic_times.min_max_mean() {
                    log::info!("Demosaic min: {:?}, max: {:?}, mean: {:?}", min, max, mean);
                }
                if let Some((min, max, mean)) = frame_receive_times.min_max_mean() {
                    log::info!("Frame min: {:?}, max: {:?}, mean: {:?}", min, max, mean);
                }

                let exposure_time = genicam
                    .get_feature_by_name("ExposureTime")
                    .ok_or_else(|| not_found_err("ExposureTime"))?;
                log::info!(
                    "Exposure Time: {:?} {}",
                    Self::read_number(control_connection, exposure_time)?,
                    exposure_time.get_unit().unwrap_or("")
                );
            }

            let recv_len = stream_connection.socket.recv(&mut buf)?;

            let mut packet_reader = ByteSliceReader::new(&buf[..recv_len]);

            let packet_status = GVSPPacketStatus::from_u16(packet_reader.read_u16_be()?);
            if packet_status != GVSPPacketStatus::SUCCESS {
                log::warning!("Packet status not success: {:?}", packet_status);
                continue;
            }

            let packet_block = packet_reader.read_u16_be()?;
            let packet_info = packet_reader.read_u32_be()?;
            let packet_format = GVSPFormat::from_u8(((packet_info & 0x7F000000) >> 24) as u8);

            let (packet_frame_id, packet_id) = {
                let has_extended_ids = (packet_info & 0x80000000) != 0;
                if has_extended_ids {
                    let frame_id = packet_reader.read_u64_be()?;
                    let packet_id = packet_reader.read_u32_be()?;
                    (frame_id, packet_id as usize)
                } else {
                    let frame_id = packet_block as u64;
                    let packet_id = packet_info & 0x00ffffff;
                    (frame_id, packet_id as usize)
                }
            };

            match packet_format {
                GVSPFormat::LEADER => {
                    if let Some(current_frame_id) = current_frame_id {
                        log::warning!(
                            "LEADER out of sequence: frame: {} packet :{}",
                            current_frame_id,
                            packet_frame_id
                        );
                    }

                    current_frame_id = Some(packet_frame_id);
                    current_packet_id = packet_id;
                    current_frame_start_time = std::time::Instant::now();
                    data_per_packet = None;

                    let _flags = packet_reader.read_u16_be()?;
                    let payload_type = GVSPPayloadType::from_u16(packet_reader.read_u16_be()?);
                    let _timestamp_high = packet_reader.read_u32_be()?;
                    let _timestamp_low = packet_reader.read_u32_be()?;
                    frame_pixel_format = GVSPPixelFormat::from_u32(packet_reader.read_u32_be()?);
                    frame_width = packet_reader.read_u32_be()?;
                    frame_height = packet_reader.read_u32_be()?;
                    let _x_offset = packet_reader.read_u32_be()?;
                    let _y_offset = packet_reader.read_u32_be()?;

                    let frame_buf_size = (frame_width
                        * frame_height
                        * (((frame_pixel_format.to_u32() >> 16) & 0xff) / 8))
                        as usize;

                    if frame_pixel_format != GVSPPixelFormat::BAYER_RG_8 {
                        return Err(std::io::Error::other(format!(
                            "Unsupported pixel format: {:?}",
                            frame_pixel_format
                        )));
                    }

                    if payload_type != GVSPPayloadType::IMAGE {
                        return Err(std::io::Error::other(format!(
                            "Unsupported payload type: {:?}",
                            payload_type
                        )));
                    }

                    if frame_buf.len() != frame_buf_size {
                        log::warning!(
                            "New frame buffer: {}x{} {:?} {}",
                            frame_width,
                            frame_height,
                            frame_pixel_format,
                            rustine::utilities::format_bytes_iec(frame_buf_size)
                        );
                        frame_buf.resize(frame_buf_size, 0);
                    }
                }
                GVSPFormat::TRAILER => {
                    if let Some(current_frame_id) = current_frame_id {
                        if current_frame_id != packet_frame_id {
                            log::warning!(
                                "TRAILER out of sequence: frame: {} packet :{}",
                                current_frame_id,
                                packet_frame_id
                            );
                            continue;
                        }

                        let packet_id_diff = packet_id - current_packet_id;
                        if packet_id_diff > 1 {
                            log::warning!(
                                "Packet gap: {} -> {} frame: {}",
                                current_packet_id,
                                packet_id,
                                packet_frame_id
                            );
                        }
                    }

                    let frame_receive_duration = current_frame_start_time.elapsed();
                    frame_receive_times.push(frame_receive_duration.as_secs_f64());

                    let demosaic_start = std::time::Instant::now();

                    let io_image = rustine::io::Image {
                        width: frame_width,
                        height: frame_height,
                        format: rustine::gfx::Format::R8G8B8A8_UNORM,
                        pixels: Self::convert_pixels(
                            frame_buf.as_mut_slice(),
                            frame_width as usize,
                            frame_height as usize,
                            frame_pixel_format,
                        )?, // TODO: Giga allocation!
                    };

                    let demosaic_duration = demosaic_start.elapsed();
                    demosaic_times.push(demosaic_duration.as_secs_f64());

                    if let Some(cache) = stream_cache.as_ref() {
                        cache.update(io_image.clone());
                    }
                    image_mailbox.push((image.id(), io_image));

                    current_frame_id = None;
                    data_per_packet = None;
                }
                GVSPFormat::PAYLOAD => {
                    if let Some(current_frame_id) = current_frame_id {
                        if current_frame_id != packet_frame_id {
                            log::warning!(
                                "PAYLOAD out of sequence: frame: {} packet :{}",
                                current_frame_id,
                                packet_frame_id
                            );
                            continue;
                        }

                        let packet_id_diff = packet_id - current_packet_id;
                        if packet_id_diff > 1 {
                            log::warning!(
                                "Packet gap: {} -> {} frame: {}",
                                current_packet_id,
                                packet_id,
                                packet_frame_id
                            );
                        }
                    }
                    current_packet_id = packet_id;

                    let payload_data_slice = packet_reader.read_to_end()?;

                    if data_per_packet.is_none() {
                        data_per_packet = Some(payload_data_slice.len());
                    }

                    let write_head = (packet_id - 1) * data_per_packet.unwrap_or(0);
                    frame_buf[write_head..write_head + payload_data_slice.len()]
                        .copy_from_slice(payload_data_slice);
                }
                _ => {
                    return Err(std::io::Error::other("Unknown packet"));
                }
            }
        }
    }

    fn convert_pixels(
        in_buf: &mut [u8],
        in_width: usize,
        in_height: usize,
        in_format: GVSPPixelFormat,
    ) -> std::io::Result<Vec<u8>> {
        match in_format {
            GVSPPixelFormat::BAYER_RG_8 => {
                //crate::demosaic_bayer_rg8(in_buf, in_width, in_height)
                Ok(rustine::io::ffmpeg::demosaic_bayer_rg8(
                    in_buf,
                    in_width as u32,
                    in_height as u32,
                )
                .map_err(std::io::Error::other)?)
            }
            _ => unsupported(format!("Unsupported pixel format: {:?}", in_format)),
        }
    }

    /// Issues a GenICam command by writing to the appropriate register.
    ///
    /// Fails if the command register is not a simple integer register.
    ///
    /// Fails if the command value is not a constant integer.
    fn issue_command(
        connection: &Connection,
        command: &genicam::GenICommand,
    ) -> std::io::Result<()> {
        let address = match &*command.value {
            genicam::GenIType::IntReg(reg) => match *reg.address {
                genicam::GenIType::ConstantInteger(addr) => addr,
                _ => {
                    log::error!(
                        "Unsupported address type in command value: {:#?}",
                        reg.address
                    );
                    return unsupported("Unsupported command address type");
                }
            },
            _ => {
                log::error!("{:#?}", command.value);
                return unsupported("Unsupported command pValue");
            }
        };

        Self::write_register(connection, address, command.cmd_value)
    }

    fn read_number(
        connection: &Connection,
        number_type: &genicam::GenIType,
    ) -> std::io::Result<f64> {
        match &number_type {
            genicam::GenIType::Float(f) => Self::read_float(connection, f),
            genicam::GenIType::Integer(i) => Self::read_integer(connection, i),
            genicam::GenIType::IntReg(r) => Self::read_int_reg(connection, r),
            _ => unsupported(format!("Unsupported number type: {:#?}", number_type)),
        }
    }

    fn write_number(
        connection: &Connection,
        number_type: &genicam::GenIType,
        value: f64,
    ) -> std::io::Result<()> {
        match &number_type {
            genicam::GenIType::Float(f) => Self::write_float(connection, f, value),
            genicam::GenIType::Integer(i) => Self::write_integer(connection, i, value),
            genicam::GenIType::IntReg(r) => Self::write_int_reg(connection, r, value),
            _ => unsupported(format!("Unsupported number type: {:#?}", number_type)),
        }
    }

    fn read_int_reg(
        connection: &Connection,
        int_reg_type: &genicam::GenIIntReg,
    ) -> std::io::Result<f64> {
        if let genicam::GenIType::ConstantInteger(length) = *int_reg_type.length
            && length != 4
        {
            return unsupported("Unsupported IntReg length");
        }
        if !int_reg_type.big_endian {
            return unsupported("Unsupported IntReg endianess");
        }
        let address = match *int_reg_type.address {
            genicam::GenIType::ConstantInteger(addr) => addr,
            _ => {
                log::error!(
                    "Unsupported address type in IntReg: {:#?}",
                    int_reg_type.address
                );
                return unsupported("Unsupported IntReg address type");
            }
        };

        let raw_value = Self::read_register(connection, address)?;
        Ok(raw_value as f64)
    }

    fn write_int_reg(
        connection: &Connection,
        int_reg_type: &genicam::GenIIntReg,
        value: f64,
    ) -> std::io::Result<()> {
        if let genicam::GenIType::ConstantInteger(length) = *int_reg_type.length
            && length != 4
        {
            return unsupported("Unsupported IntReg length");
        }
        if !int_reg_type.big_endian {
            return unsupported("Unsupported IntReg endianess");
        }
        let address = match *int_reg_type.address {
            genicam::GenIType::ConstantInteger(addr) => addr,
            _ => {
                log::error!(
                    "Unsupported address type in IntReg: {:#?}",
                    int_reg_type.address
                );
                return unsupported("Unsupported IntReg address type");
            }
        };

        Self::write_register(connection, address, value as u32)
    }

    fn read_enumeration<'a>(
        connection: &Connection,
        enumeration_type: &'a genicam::GenIEnumeration,
    ) -> std::io::Result<&'a str> {
        let value = Self::read_number(connection, &enumeration_type.value)?;

        match enumeration_type.value_to_name(value as u32) {
            Some(name) => Ok(name),
            None => unsupported("Enumeration value not found"),
        }
    }

    fn write_enumeration(
        connection: &Connection,
        enumeration_type: &genicam::GenIEnumeration,
        name: &str,
    ) -> std::io::Result<()> {
        let value = enumeration_type.name_to_value(name).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid enumeration name")
        })?;
        Self::write_number(connection, &enumeration_type.value, value as f64)
    }

    fn read_float(
        connection: &Connection,
        float_type: &genicam::GenIFloat,
    ) -> std::io::Result<f64> {
        match &*float_type.value {
            genicam::GenIType::Converter(conv) => Self::read_converter(connection, conv),
            _ => unsupported("Unsupported float type value"),
        }
    }

    fn write_float(
        connection: &Connection,
        float_type: &genicam::GenIFloat,
        value: f64,
    ) -> std::io::Result<()> {
        match &*float_type.value {
            genicam::GenIType::Converter(conv) => Self::write_converter(connection, conv, value),
            _ => unsupported("Unsupported float type value"),
        }
    }

    fn read_integer(
        connection: &Connection,
        integer_type: &genicam::GenIInteger,
    ) -> std::io::Result<f64> {
        match &*integer_type.value {
            genicam::GenIType::IntReg(ireg) => Self::read_int_reg(connection, ireg),
            genicam::GenIType::Converter(conv) => Self::read_converter(connection, conv),
            _ => unsupported("Unsupported integer type value"),
        }
    }

    fn write_integer(
        connection: &Connection,
        integer_type: &genicam::GenIInteger,
        value: f64,
    ) -> std::io::Result<()> {
        match &*integer_type.value {
            genicam::GenIType::IntReg(ireg) => Self::write_int_reg(connection, ireg, value),
            genicam::GenIType::Converter(conv) => Self::write_converter(connection, conv, value),
            _ => unsupported("Unsupported integer type value"),
        }
    }

    fn read_converter(
        connection: &Connection,
        converter_type: &genicam::GenIConverter,
    ) -> std::io::Result<f64> {
        let mut resolved_variables = HashMap::<String, f64>::new();

        for (name, variable) in &converter_type.variables {
            let value = Self::read_number(connection, variable)?;
            resolved_variables.insert(name.clone(), value);
        }

        let value = Self::read_number(connection, &converter_type.value)?;
        resolved_variables.insert("TO".to_string(), value);

        genicam::evaluate(&converter_type.expression_from, &resolved_variables)
    }

    fn write_converter(
        connection: &Connection,
        converter_type: &genicam::GenIConverter,
        value: f64,
    ) -> std::io::Result<()> {
        let mut resolved_variables = HashMap::<String, f64>::new();

        for (name, variable) in &converter_type.variables {
            let value = Self::read_number(connection, variable)?;
            resolved_variables.insert(name.clone(), value);
        }

        resolved_variables.insert("FROM".to_string(), value);

        genicam::evaluate(&converter_type.expression_to, &resolved_variables)?;
        Ok(())
    }

    fn read_memory(connection: &Connection, address: u32, length: u32) -> std::io::Result<Vec<u8>> {
        const MAXIMUM_READ_LENGTH: u32 = 512;
        const READ_ALIGN: u32 = std::mem::size_of::<u32>() as u32;

        let mut io_buffer = [0u8; 1500];

        let mut read_buffer = Vec::with_capacity(length as usize);
        let read_count = length.div_ceil(MAXIMUM_READ_LENGTH);

        for ir in 0..read_count {
            let read_head = ir * MAXIMUM_READ_LENGTH;
            let read_size = (length - read_head).min(MAXIMUM_READ_LENGTH);
            let read_size_aligned = read_size.div_ceil(READ_ALIGN) * READ_ALIGN;
            let read_offset = address + read_head;

            let mut request_packet_data = [0u8; 8];
            request_packet_data[..4].copy_from_slice(&read_offset.to_be_bytes());
            request_packet_data[4..].copy_from_slice(&read_size_aligned.to_be_bytes());
            let request_packet = GigEPacket::new(
                GigEPacketType::CMD,
                GigEPacketFlags::ACK_REQUIRED,
                GigECommand::READ_MEMORY_CMD,
                REQUEST_ID.fetch_add(1, atomic::Ordering::Relaxed),
                &request_packet_data,
            );

            let response_len =
                Self::send_cmd_recv_ack(connection, &request_packet, &mut io_buffer)?;
            let response_packet = GigEPacket::from_slice(&io_buffer[..response_len])?;
            let mut response_data_reader = ByteSliceReader::new(response_packet.data);
            response_data_reader.read_u32_be()?;
            read_buffer.extend_from_slice(response_data_reader.read_to_end()?);
        }

        Ok(read_buffer)
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

        let mut io_buffer = [0u8; 1500];

        let response_len = Self::send_cmd_recv_ack(connection, &request_packet, &mut io_buffer)?;
        let response_packet = GigEPacket::from_slice(&io_buffer[..response_len])?;

        let mut response_data_reader = ByteSliceReader::new(response_packet.data);

        let register_value = response_data_reader.read_u32_be()?;
        Ok(register_value)
    }

    fn write_register(connection: &Connection, address: u32, value: u32) -> std::io::Result<()> {
        let mut request_packet_data = [0u8; 8];
        request_packet_data[..4].copy_from_slice(&address.to_be_bytes());
        request_packet_data[4..].copy_from_slice(&value.to_be_bytes());

        let request_packet = GigEPacket::new(
            GigEPacketType::CMD,
            GigEPacketFlags::ACK_REQUIRED,
            GigECommand::WRITE_REGISTER_CMD,
            REQUEST_ID.fetch_add(1, atomic::Ordering::Relaxed),
            &request_packet_data,
        );

        let mut io_buffer = [0u8; 1500];
        Self::send_cmd_recv_ack(connection, &request_packet, &mut io_buffer)?;
        Ok(())
    }

    /// Sends a command packet and waits for an ACK response.
    ///
    /// Retry count according to `CMD_MAX_RETRIES`.
    ///
    /// Timeout according to `CMD_RECV_TIMEOUT_MS`.
    fn send_cmd_recv_ack(
        connection: &Connection,
        send_packet: &GigEPacket,
        io_buffer: &mut [u8],
    ) -> std::io::Result<usize> {
        let mut retries = 0;

        let send_len = send_packet.to_slice(io_buffer)?;

        connection.socket.send(&io_buffer[..send_len])?;
        connection
            .socket
            .set_read_timeout(Some(std::time::Duration::from_millis(
                Self::CMD_RECV_TIMEOUT_MS,
            )))?;

        loop {
            if let Ok(recv_len) = connection.socket.recv(io_buffer) {
                let recv_packet = GigEPacket::from_slice(&io_buffer[..recv_len])?;

                let is_ack = recv_packet.t == GigEPacketType::ACK;
                let is_same_request = send_packet.id == recv_packet.id;

                if is_ack && is_same_request {
                    return Ok(recv_len);
                }

                // Oh no, received a packet that is not an ACK
                // or does not match the request ID, try receiving again
            }

            retries += 1;
            if retries >= Self::CMD_MAX_RETRIES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "send_cmd_recv_ack: No ACK received",
                ));
            }
        }
    }

    fn setup_sockets(
        adapter: &IPAdapter,
        device_address: Ipv4Addr,
    ) -> std::io::Result<(Connection, Connection)> {
        let socket = UdpSocket::bind(SocketAddrV4::new(adapter.address, 0))?;
        let recv_address = SocketAddrV4::new(adapter.address, socket.local_addr()?.port());
        let send_address = SocketAddrV4::new(device_address, GVCP_PORT);
        socket.connect(send_address)?;
        let control_connection = Connection {
            socket,
            remote_address: send_address,
            local_address: recv_address,
        };
        let stream_local_address = SocketAddrV4::new(adapter.address, 0);
        let stream_remote_address = SocketAddrV4::new(device_address, 0);
        let stream_socket = UdpSocket::bind(stream_local_address)?;
        let stream_local_address =
            SocketAddrV4::new(adapter.address, stream_socket.local_addr()?.port());
        stream_socket.connect(stream_remote_address)?;
        stream_socket.set_read_timeout(Some(std::time::Duration::from_millis(1000)))?;
        let stream_connection = Connection {
            socket: stream_socket,
            remote_address: stream_remote_address,
            local_address: stream_local_address,
        };

        unsafe {
            let rcvbuf = Self::STREAM_CHANNEL_RECV_BUFFER as libc::c_int;
            let ret = libc::setsockopt(
                stream_connection.socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
            if ret != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok((control_connection, stream_connection))
    }
}
