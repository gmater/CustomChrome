pub const VENDOR_ID: u16 = 0x0C45;
pub const PRODUCT_ID_WIRED: u16 = 0x800A;
pub const DONGLE_VENDOR_ID: u16 = 0x05AC;
pub const PRODUCT_ID_DONGLE: u16 = 0x024F;

pub const USAGE_PAGE_CONTROL: u16 = 0xFF68;
pub const USAGE_CONTROL: u16 = 0x61;
pub const USAGE_PAGE_SCREEN: u16 = 0xFF67;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 135;
pub const FRAME_RGBA_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT * 4;
pub const SCREEN_PACKET_LEN: usize = 4104;
pub const SCREEN_HEADER_LEN: usize = 256;
pub const MAX_HEADER_FRAMES: usize = SCREEN_HEADER_LEN - 1;

const PACKET_MAGIC: u8 = 0xAA;
const CMD_DEVICE_INFO: u8 = 0x10;
const CMD_SCREEN: u8 = 0x50;
const SCREEN_CHUNK: usize = 4096;
const SCREEN_FLASH_ADDRESS: usize = 0x65_0000;
const DEVICE_INFO_LEN: usize = 32;

pub const ACK_SCREEN: [u8; 2] = [0x55, 0x41];

pub fn rgba_to_rgb565_le(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 2);
    for px in rgba.chunks_exact(4) {
        let (r, g, b) = (px[0] as u16, px[1] as u16, px[2] as u16);
        let value = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// The firmware stores one delay byte per frame except the last, in units of 2 ms.
pub fn screen_header(delays_cs: &[u16]) -> [u8; SCREEN_HEADER_LEN] {
    let frames = delays_cs.len().min(MAX_HEADER_FRAMES);
    let mut header = [0xFF; SCREEN_HEADER_LEN];
    header[0] = frames as u8;
    for (i, delay) in delays_cs.iter().take(frames.saturating_sub(1)).enumerate() {
        header[i + 1] = delay.saturating_mul(5).clamp(1, 255) as u8;
    }
    header[frames] = 0;
    header
}

fn screen_packet(index: usize, total_chunks: usize, data: &[u8]) -> Vec<u8> {
    let mut packet = vec![0; SCREEN_PACKET_LEN];
    packet[0] = PACKET_MAGIC;
    packet[1] = CMD_SCREEN;
    packet[2..4].copy_from_slice(&(index as u16).to_le_bytes());
    packet[4..6].copy_from_slice(&(total_chunks as u16).to_le_bytes());
    packet[6..8].copy_from_slice(&((SCREEN_FLASH_ADDRESS / SCREEN_CHUNK) as u16).to_le_bytes());
    packet[8..8 + data.len()].copy_from_slice(data);
    packet
}

/// Chunking matches the official app: the chunk count covers the pixel data only,
/// and the 256-byte header shifts everything after the first chunk.
pub fn screen_packets(header: &[u8; SCREEN_HEADER_LEN], pixels: &[u8]) -> Vec<Vec<u8>> {
    let total = pixels.len().div_ceil(SCREEN_CHUNK);
    (0..total)
        .map(|index| {
            let data = if index == 0 {
                let mut first = header.to_vec();
                first.extend_from_slice(&pixels[..pixels.len().min(SCREEN_CHUNK - SCREEN_HEADER_LEN)]);
                first
            } else {
                let start = SCREEN_CHUNK * index - SCREEN_HEADER_LEN;
                pixels[start..(start + SCREEN_CHUNK).min(pixels.len())].to_vec()
            };
            screen_packet(index, total, &data)
        })
        .collect()
}

pub fn device_info_request(packet_len: usize) -> Vec<u8> {
    let mut packet = vec![0; packet_len];
    packet[0] = PACKET_MAGIC;
    packet[1] = CMD_DEVICE_INFO;
    packet[2] = (packet_len - 8) as u8;
    packet
}

pub fn is_device_info_reply(reply: &[u8]) -> bool {
    reply.len() > 8 && reply[0] == PACKET_MAGIC && reply[1] == CMD_DEVICE_INFO
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardInfo {
    pub firmware: String,
    pub battery: u8,
    pub max_frames: u16,
}

pub fn parse_device_info(reply: &[u8]) -> KeyboardInfo {
    let mut info = [0u8; DEVICE_INFO_LEN];
    let len = reply[2] as usize;
    let offset = reply[3] as usize | (reply[4] as usize) << 8;
    for i in 0..len {
        let (src, dst) = (8 + i, offset + i);
        if src < reply.len() && dst < DEVICE_INFO_LEN {
            info[dst] = reply[src];
        }
    }
    let version = (info[8] & 0x0F) as u32 + 10 * ((info[8] & 0xF0) >> 4) as u32 + 100 * info[9] as u32;
    KeyboardInfo {
        firmware: format!("{:.2}", version as f64 / 100.0),
        battery: info[17],
        max_frames: u16::from_le_bytes([info[22], info[23]]),
    }
}

/// Total size of the first output report in a HID report descriptor, in bytes.
pub fn output_report_len(descriptor: &[u8]) -> Option<usize> {
    let (mut report_size, mut report_count, mut bits) = (0u32, 0u32, 0u32);
    let mut i = 0;
    while i < descriptor.len() {
        let prefix = descriptor[i];
        if prefix == 0xFE {
            let Some(&long_len) = descriptor.get(i + 1) else { break };
            i += 3 + long_len as usize;
            continue;
        }
        let len = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let Some(bytes) = descriptor.get(i + 1..i + 1 + len) else { break };
        let value = bytes
            .iter()
            .enumerate()
            .fold(0u32, |acc, (k, b)| acc | (*b as u32) << (8 * k));
        match prefix & 0xFC {
            0x74 => report_size = value,
            0x94 => report_count = value,
            0x90 => bits += report_size * report_count,
            _ => {}
        }
        i += 1 + len;
    }
    (bits > 0).then_some((bits / 8) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_matches_official_encoding() {
        assert_eq!(rgba_to_rgb565_le(&[255, 255, 255, 255]), vec![0xFF, 0xFF]);
        assert_eq!(rgba_to_rgb565_le(&[255, 0, 0, 255]), vec![0x00, 0xF8]);
        assert_eq!(rgba_to_rgb565_le(&[0, 255, 0, 255]), vec![0xE0, 0x07]);
        assert_eq!(rgba_to_rgb565_le(&[0, 0, 255, 255]), vec![0x1F, 0x00]);
    }

    #[test]
    fn header_stores_count_and_all_but_last_delay() {
        let header = screen_header(&[10, 4, 7]);
        assert_eq!(header[0], 3);
        assert_eq!(&header[1..4], &[50, 20, 0]);
        assert_eq!(header[4], 0xFF);
    }

    #[test]
    fn header_clamps_long_delays() {
        assert_eq!(screen_header(&[200, 1])[1], 255);
    }

    #[test]
    fn packets_match_official_layout() {
        let pixels: Vec<u8> = (0..SCREEN_WIDTH * SCREEN_HEIGHT * 2 * 2).map(|i| (i % 251) as u8).collect();
        let header = screen_header(&[10, 10]);
        let packets = screen_packets(&header, &pixels);

        assert_eq!(packets.len(), pixels.len().div_ceil(4096));
        let first = &packets[0];
        assert_eq!(first.len(), SCREEN_PACKET_LEN);
        assert_eq!(&first[0..8], &[0xAA, 0x50, 0, 0, packets.len() as u8, 0, 0x50, 0x06]);
        assert_eq!(&first[8..8 + 256], &header[..]);
        assert_eq!(&first[8 + 256..8 + 4096], &pixels[..3840]);

        let second = &packets[1];
        assert_eq!(&second[2..4], &[1, 0]);
        assert_eq!(&second[8..8 + 4096], &pixels[3840..3840 + 4096]);
    }

    #[test]
    fn device_info_reads_frame_limit_and_firmware() {
        let mut reply = vec![0u8; 64];
        reply[0] = 0xAA;
        reply[1] = 0x10;
        reply[2] = 56;
        reply[8 + 8] = 0x23;
        reply[8 + 9] = 1;
        reply[8 + 17] = 87;
        reply[8 + 22] = 0x2C;
        reply[8 + 23] = 0x01;
        let info = parse_device_info(&reply);
        assert_eq!(info.firmware, "1.23");
        assert_eq!(info.battery, 87);
        assert_eq!(info.max_frames, 300);
    }

    #[test]
    fn output_report_len_reads_vendor_descriptor() {
        let descriptor = [
            0x06, 0x67, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x75, 0x08, 0x96, 0x08, 0x10, 0x91, 0x02, 0xC0,
        ];
        assert_eq!(output_report_len(&descriptor), Some(4104));
    }
}
