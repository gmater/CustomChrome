pub const VENDOR_ID: u16 = 0x0C45;
pub const PRODUCT_ID_WIRED: u16 = 0x800A;
pub const DONGLE_VENDOR_ID: u16 = 0x05AC;
pub const PRODUCT_ID_DONGLE: u16 = 0x024F;

pub const PRODUCT_NAME_WIRED: &str = "AULA F99 MAX";
pub const PRODUCT_NAME_DONGLE: &str = "F99 MAX Dongle";

pub const USAGE_PAGE_CONTROL: u16 = 0xFF68;
pub const USAGE_PAGE_CONTROL_DONGLE: u16 = 0xFF60;
pub const USAGE_CONTROL: u16 = 0x61;
pub const USAGE_PAGE_SCREEN: u16 = 0xFF67;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 135;
pub const FRAME_RGBA_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT * 4;
pub const SCREEN_PACKET_LEN: usize = 4104;
pub const SCREEN_HEADER_LEN: usize = 256;
pub const MAX_HEADER_FRAMES: usize = SCREEN_HEADER_LEN - 1;

const PACKET_MAGIC: u8 = 0xAA;
pub const CMD_DEVICE_INFO: u8 = 0x10;
pub const CMD_READ_LIGHTING: u8 = 0x13;
pub const CMD_WRITE_LIGHTING: u8 = 0x23;
const CMD_SCREEN: u8 = 0x50;
/// Status reports the firmware sends unprompted; the official app never treats them as a reply.
pub const CMD_STATUS: u8 = 0x37;
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

/// A single-packet command; the official app always declares a full payload even when `data` is shorter.
pub fn request(cmd: u8, packet_len: usize, data: &[u8]) -> Vec<u8> {
    let mut packet = vec![0; packet_len];
    packet[0] = PACKET_MAGIC;
    packet[1] = cmd;
    packet[2] = (packet_len - 8) as u8;
    packet[8..8 + data.len()].copy_from_slice(data);
    packet
}

/// Replies start with 0x55 rather than the request's 0xAA, so only the command byte identifies them.
pub fn is_reply_to(cmd: u8, reply: &[u8]) -> bool {
    reply.len() > 8 && reply[1] == cmd
}

/// Copies a reply's payload into `target` at the offset the reply declares.
pub fn merge_reply(reply: &[u8], target: &mut [u8]) {
    let len = reply[2] as usize;
    let offset = reply[3] as usize | (reply[4] as usize) << 8;
    for i in 0..len {
        let (src, dst) = (8 + i, offset + i);
        if src < reply.len() && dst < target.len() {
            target[dst] = reply[src];
        }
    }
}

/// Splits a table transfer exactly like the official app: full-size chunks, with only a
/// non-first final chunk shortened to the remainder.
pub fn chunked_requests(cmd: u8, packet_len: usize, table: &[u8]) -> Vec<Vec<u8>> {
    let chunk = packet_len - 8;
    let count = table.len().div_ceil(chunk);
    (0..count)
        .map(|i| {
            let offset = i * chunk;
            let len = if i == count - 1 && i != 0 && table.len() % chunk != 0 { table.len() % chunk } else { chunk };
            let mut packet = vec![0; packet_len];
            packet[0] = PACKET_MAGIC;
            packet[1] = cmd;
            packet[2] = len as u8;
            packet[3..5].copy_from_slice(&(offset as u16).to_le_bytes());
            let data = &table[offset..(offset + chunk).min(table.len())];
            packet[8..8 + data.len()].copy_from_slice(data);
            packet
        })
        .collect()
}

pub const CMD_READ_KEYS: u8 = 0x12;
pub const CMD_WRITE_KEYS: u8 = 0x22;
pub const KEY_TABLE_LEN: usize = 512;
pub const FN_KEY_INDEX: u8 = 85;

pub type KeyEntry = [u8; 4];

/// Physical F99 MAX keys (matrix index, factory HID usage). Fn is listed but never written.
pub const F99_KEYS: &[(u8, u8)] = &[
    (0, 0x29), (1, 0x3A), (2, 0x3B), (3, 0x3C), (4, 0x3D), (5, 0x3E), (6, 0x3F), (7, 0x40), (8, 0x41),
    (9, 0x42), (10, 0x43), (11, 0x44), (12, 0x45), (106, 0x4C),
    (16, 0x35), (17, 0x1E), (18, 0x1F), (19, 0x20), (20, 0x21), (21, 0x22), (22, 0x23), (23, 0x24),
    (24, 0x25), (25, 0x26), (26, 0x27), (27, 0x2D), (28, 0x2E), (92, 0x2A), (104, 0x4A),
    (29, 0x53), (30, 0x54), (31, 0x55), (109, 0x56),
    (32, 0x2B), (33, 0x14), (34, 0x1A), (35, 0x08), (36, 0x15), (37, 0x17), (38, 0x1C), (39, 0x18),
    (40, 0x0C), (41, 0x12), (42, 0x13), (43, 0x2F), (44, 0x30), (60, 0x31), (105, 0x4B),
    (45, 0x5F), (46, 0x60), (47, 0x61), (110, 0x57),
    (48, 0x39), (49, 0x04), (50, 0x16), (51, 0x07), (52, 0x09), (53, 0x0A), (54, 0x0B), (55, 0x0D),
    (56, 0x0E), (57, 0x0F), (58, 0x33), (59, 0x34), (76, 0x28), (108, 0x4E),
    (61, 0x5C), (62, 0x5D), (63, 0x5E),
    (64, 0xE1), (65, 0x1D), (66, 0x1B), (67, 0x06), (68, 0x19), (69, 0x05), (70, 0x11), (71, 0x10),
    (72, 0x36), (73, 0x37), (74, 0x38), (75, 0xE5), (90, 0x52),
    (77, 0x59), (78, 0x5A), (79, 0x5B), (95, 0x58),
    (80, 0xE0), (81, 0xE3), (82, 0xE2), (83, 0x2C), (84, 0xE6), (FN_KEY_INDEX, 0xAF), (88, 0x50),
    (89, 0x51), (91, 0x4F), (93, 0x62), (94, 0x63),
];

pub fn factory_entry(index: u8) -> Option<KeyEntry> {
    F99_KEYS.iter().find(|(i, _)| *i == index).map(|&(_, hid)| [2, 0, hid, 0])
}

pub fn is_remappable(index: u8) -> bool {
    index != FN_KEY_INDEX && factory_entry(index).is_some()
}

const MOUSE_BUTTONS: [u8; 5] = [1, 2, 4, 8, 16];

/// Accepts only the assignment kinds CustomChrome offers: keys, modifiers, media and mouse.
pub fn is_valid_entry(e: &KeyEntry) -> bool {
    match *e {
        [2, 0, code, 0] => (0x04..=0xA4).contains(&code) || (0xE0..=0xE7).contains(&code),
        [2, modifier, 0, 0] => (0xE0..=0xE7).contains(&modifier),
        [3, lo, hi, 0] => u16::from_le_bytes([lo, hi]) != 0,
        [1, 1, button, 0] => MOUSE_BUTTONS.contains(&button),
        [1, 3, 1 | 255, 0] => true,
        [MACRO_KIND, index, mode, repeats] => {
            (index as usize) < MACRO_COUNT_MAX && MACRO_MODES.contains(&mode) && (mode == 1) == (repeats > 0)
        }
        _ => false,
    }
}

pub const CMD_READ_MACROS: u8 = 0x15;
pub const CMD_WRITE_MACROS: u8 = 0x25;
pub const MACRO_AREA_LEN: usize = 1024;
const MACRO_INDEX_LEN: usize = 400;
pub const MACRO_COUNT_MAX: usize = MACRO_INDEX_LEN / 4;
/// 4-byte words left after the index table; each macro takes one for its header plus one per event.
pub const MACRO_WORDS: usize = (MACRO_AREA_LEN - MACRO_INDEX_LEN) / 4;

const EVENT_DOWN: u8 = 0x80;
const EVENT_KEYBOARD: u8 = 0x20;
const EVENT_BASE: u8 = 0x10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroEvent {
    pub code: u8,
    pub down: bool,
    pub mouse: bool,
    /// Milliseconds to wait after this event.
    pub delay: u16,
}

pub type Macro = Vec<MacroEvent>;

pub fn macro_words(macros: &[Macro]) -> usize {
    macros.iter().map(|m| 1 + m.len()).sum()
}

pub fn is_valid_macro_event(e: &MacroEvent) -> bool {
    if e.mouse {
        matches!(e.code, 1 | 2 | 4)
    } else {
        (0x04..=0xA4).contains(&e.code) || (0xE0..=0xE7).contains(&e.code)
    }
}

/// Lays macros out like the official app: an index of little-endian addresses, then each
/// macro as `[2 × event count, 0, 0]` followed by `[delay lo, delay hi, code, flags]` events.
pub fn encode_macros(macros: &[Macro]) -> Option<Vec<u8>> {
    if macros.len() > MACRO_COUNT_MAX
        || macro_words(macros) > MACRO_WORDS
        || macros.iter().any(|m| m.is_empty() || !m.iter().all(is_valid_macro_event))
    {
        return None;
    }
    let mut area = vec![0u8; MACRO_AREA_LEN];
    let mut addr = MACRO_INDEX_LEN;
    for (i, events) in macros.iter().enumerate() {
        area[i * 4..i * 4 + 4].copy_from_slice(&(addr as u32).to_le_bytes());
        area[addr..addr + 2].copy_from_slice(&((events.len() * 2) as u16).to_le_bytes());
        for (j, e) in events.iter().enumerate() {
            let at = addr + 4 + j * 4;
            let flags = EVENT_BASE | if e.down { EVENT_DOWN } else { 0 } | if e.mouse { 0 } else { EVENT_KEYBOARD };
            area[at..at + 2].copy_from_slice(&e.delay.to_le_bytes());
            area[at + 2] = e.code;
            area[at + 3] = flags;
        }
        addr += 4 + events.len() * 4;
    }
    Some(area)
}

pub fn decode_macros(area: &[u8]) -> Vec<Macro> {
    let word = |at: usize| area.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let mut macros = Vec::new();
    for i in 0..MACRO_COUNT_MAX {
        let addr = match word(i * 4) {
            Some(0) | None => break,
            Some(a) => a,
        };
        let Some(header) = area.get(addr..addr + 2) else { break };
        let count = u16::from_le_bytes([header[0], header[1]]) as usize / 2;
        let events = (0..count)
            .map_while(|j| area.get(addr + 4 + j * 4..addr + 8 + j * 4))
            .map(|b| MacroEvent {
                delay: u16::from_le_bytes([b[0], b[1]]),
                code: b[2],
                down: b[3] & EVENT_DOWN != 0,
                mouse: b[3] & EVENT_KEYBOARD == 0,
            })
            .collect();
        macros.push(events);
    }
    macros
}

pub const MACRO_KIND: u8 = 6;
pub const MACRO_MODES: [u8; 3] = [0, 1, 2];

/// Rewrites macro assignments after macro `removed` is deleted: its keys go back to factory
/// and keys bound to later macros follow them down one slot.
pub fn shift_macro_refs(table: &mut [u8], removed: u8) {
    for (index, entry) in table.chunks_exact_mut(4).enumerate() {
        if entry[0] != MACRO_KIND {
            continue;
        }
        if entry[1] == removed {
            let restored = factory_entry(index as u8).unwrap_or([0; 4]);
            entry.copy_from_slice(&restored);
        } else if entry[1] > removed {
            entry[1] -= 1;
        }
    }
}

pub const LIGHTING_LEN: usize = 16;
pub const LIGHTING_LEVEL_MAX: u8 = 6;
const EFFECT_LAST_BUILT_IN: u8 = 0x13;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Lighting {
    pub effect: u8,
    pub color: [u8; 3],
    pub rainbow: bool,
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
}

impl Lighting {
    pub fn from_bytes(b: &[u8; LIGHTING_LEN]) -> Self {
        let level = |v: u8| v.clamp(1, LIGHTING_LEVEL_MAX);
        Lighting {
            effect: b[0],
            color: [b[1], b[2], b[3]],
            rainbow: b[8] != 0,
            brightness: level(b[9]),
            speed: level(b[10]),
            direction: b[11],
        }
    }

    pub fn is_supported_effect(effect: u8) -> bool {
        effect <= EFFECT_LAST_BUILT_IN
    }

    /// Writes this state over `current`, keeping the bytes CustomChrome doesn't manage.
    pub fn apply_to(&self, current: &[u8; LIGHTING_LEN]) -> [u8; LIGHTING_LEN] {
        let mut b = *current;
        b[0] = self.effect;
        b[1..4].copy_from_slice(&self.color);
        b[4] = 0xFF;
        b[8] = self.rainbow as u8;
        b[9] = self.brightness.clamp(1, LIGHTING_LEVEL_MAX);
        b[10] = self.speed.clamp(1, LIGHTING_LEVEL_MAX);
        b[11] = self.direction.min(1);
        b[14] = 0xAA;
        b[15] = 0x55;
        b
    }
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
    merge_reply(reply, &mut info);
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
    fn parses_replies_captured_from_f99_max_dongle() {
        let info_reply = [
            0x55, 0x10, 0x18, 0, 0, 0, 0, 0, 0x40, 0, 0, 0x0C, 0x45, 0x0C, 0x0A, 0x80, 0x22, 0x01, 0xFF, 0xFF, 0x11,
            0, 0x06, 0, 0x02, 0x38, 0, 0, 0, 0, 0x68, 0,
        ];
        assert!(is_reply_to(CMD_DEVICE_INFO, &info_reply));
        assert_eq!(
            parse_device_info(&info_reply),
            KeyboardInfo { firmware: "1.22".into(), battery: 56, max_frames: 104 }
        );

        let lighting_reply = [
            0x55, 0x13, 0x18, 0, 0, 0, 0, 0, 0x0D, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0x01, 0x06, 0x03, 0, 0, 0, 0xAA,
            0x55, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(is_reply_to(CMD_READ_LIGHTING, &lighting_reply));
        let mut bytes = [0u8; LIGHTING_LEN];
        merge_reply(&lighting_reply, &mut bytes);
        assert_eq!(
            Lighting::from_bytes(&bytes),
            Lighting { effect: 0x0D, color: [255; 3], rainbow: true, brightness: 6, speed: 3, direction: 0 }
        );
    }

    #[test]
    fn key_table_chunks_match_official_layout() {
        let table: Vec<u8> = (0..KEY_TABLE_LEN).map(|i| i as u8).collect();
        let packets = chunked_requests(CMD_WRITE_KEYS, 32, &table);
        assert_eq!(packets.len(), 22);
        assert_eq!(&packets[0][..5], &[0xAA, 0x22, 24, 0, 0]);
        assert_eq!(&packets[0][8..32], &table[..24]);
        assert_eq!(&packets[21][..5], &[0xAA, 0x22, 8, 0xF8, 0x01]);
        assert_eq!(&packets[21][8..16], &table[504..]);
        assert!(packets[21][16..].iter().all(|&b| b == 0));

        let reads = chunked_requests(CMD_READ_KEYS, 64, &[0; KEY_TABLE_LEN]);
        assert_eq!(reads.len(), 10);
        assert_eq!(&reads[9][..5], &[0xAA, 0x12, 8, 0xF8, 0x01]);
    }

    #[test]
    fn key_entries_are_validated() {
        assert!(is_valid_entry(&[2, 0, 0x04, 0]));
        assert!(is_valid_entry(&[2, 0xE0, 0, 0]));
        assert!(is_valid_entry(&[3, 0xCD, 0, 0]));
        assert!(is_valid_entry(&[1, 1, 2, 0]));
        assert!(is_valid_entry(&[1, 3, 255, 0]));
        assert!(!is_valid_entry(&[0, 0, 0, 0]));
        assert!(!is_valid_entry(&[6, 0, 0, 1]));
        assert!(!is_valid_entry(&[2, 0, 0x00, 0]));
        assert!(!is_valid_entry(&[1, 1, 3, 0]));
    }

    #[test]
    fn fn_key_is_never_remappable() {
        assert!(!is_remappable(FN_KEY_INDEX));
        assert!(is_remappable(33));
        assert!(!is_remappable(13));
        assert_eq!(factory_entry(59), Some([2, 0, 0x34, 0]));
        assert_eq!(F99_KEYS.len(), 97);
    }

    #[test]
    fn requests_match_official_layout() {
        let read = request(CMD_READ_LIGHTING, 32, &[]);
        assert_eq!(read.len(), 32);
        assert_eq!(&read[..8], &[0xAA, 0x13, 24, 0, 0, 0, 0, 0]);
        assert!(read[8..].iter().all(|&b| b == 0));

        let write = request(CMD_WRITE_LIGHTING, 64, &[7; LIGHTING_LEN]);
        assert_eq!(&write[..3], &[0xAA, 0x23, 56]);
        assert_eq!(&write[8..24], &[7; LIGHTING_LEN]);
        assert_eq!(write[24], 0);
    }

    #[test]
    fn lighting_round_trips_official_default() {
        let default = [11, 255, 255, 255, 255, 0, 0, 0, 1, 5, 3, 0, 0, 0, 170, 85];
        let lighting = Lighting::from_bytes(&default);
        assert_eq!(
            lighting,
            Lighting { effect: 11, color: [255; 3], rainbow: true, brightness: 5, speed: 3, direction: 0 }
        );
        assert_eq!(lighting.apply_to(&default), default);
    }

    #[test]
    fn lighting_write_clamps_and_keeps_unmanaged_bytes() {
        let current = [1, 0, 0, 0, 255, 9, 8, 7, 0, 1, 1, 0, 4, 3, 170, 85];
        let wanted = Lighting { effect: 7, color: [10, 20, 30], rainbow: false, brightness: 0, speed: 99, direction: 5 };
        assert_eq!(
            wanted.apply_to(&current),
            [7, 10, 20, 30, 255, 9, 8, 7, 0, 1, 6, 1, 4, 3, 170, 85]
        );
    }

    #[test]
    fn only_built_in_effects_are_allowed() {
        assert!(Lighting::is_supported_effect(0));
        assert!(Lighting::is_supported_effect(0x13));
        assert!(!Lighting::is_supported_effect(0x80));
    }

    #[test]
    fn output_report_len_reads_vendor_descriptor() {
        let descriptor = [
            0x06, 0x67, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x75, 0x08, 0x96, 0x08, 0x10, 0x91, 0x02, 0xC0,
        ];
        assert_eq!(output_report_len(&descriptor), Some(4104));
    }

    fn tap(code: u8, mouse: bool) -> [MacroEvent; 2] {
        [
            MacroEvent { code, down: true, mouse, delay: 10 },
            MacroEvent { code, down: false, mouse, delay: 300 },
        ]
    }

    #[test]
    fn macros_encode_like_official_app() {
        let macros = vec![tap(0x04, false).to_vec(), tap(1, true).to_vec()];
        let area = encode_macros(&macros).unwrap();
        assert_eq!(area.len(), MACRO_AREA_LEN);
        assert_eq!(&area[0..8], &[0x90, 1, 0, 0, 0x9C, 1, 0, 0]);
        assert_eq!(&area[400..412], &[4, 0, 0, 0, 10, 0, 0x04, 0xB0, 0x2C, 1, 0x04, 0x30]);
        assert_eq!(&area[412..424], &[4, 0, 0, 0, 10, 0, 1, 0x90, 0x2C, 1, 1, 0x10]);
        assert!(area[8..400].iter().all(|&b| b == 0));
        assert_eq!(decode_macros(&area), macros);
    }

    #[test]
    fn macros_reject_empty_invalid_or_oversized() {
        assert!(encode_macros(&[vec![]]).is_none());
        assert!(encode_macros(&[vec![MacroEvent { code: 3, down: true, mouse: true, delay: 0 }]]).is_none());
        let full = vec![MacroEvent { code: 0x04, down: true, mouse: false, delay: 0 }; MACRO_WORDS - 1];
        assert!(encode_macros(&[full.clone()]).is_some());
        assert!(encode_macros(&[full, vec![tap(0x05, false)[0]]]).is_none());
    }

    #[test]
    fn deleting_a_macro_fixes_key_assignments() {
        let mut table = vec![0u8; KEY_TABLE_LEN];
        table[33 * 4..34 * 4].copy_from_slice(&[6, 0, 0, 0]);
        table[34 * 4..35 * 4].copy_from_slice(&[6, 1, 1, 3]);
        table[35 * 4..36 * 4].copy_from_slice(&[6, 2, 2, 0]);
        shift_macro_refs(&mut table, 1);
        assert_eq!(&table[33 * 4..34 * 4], &[6, 0, 0, 0]);
        assert_eq!(&table[34 * 4..35 * 4], &factory_entry(34).unwrap());
        assert_eq!(&table[35 * 4..36 * 4], &[6, 1, 2, 0]);
    }

    #[test]
    fn macro_assignments_validate_mode_and_repeats() {
        assert!(is_valid_entry(&[6, 0, 0, 0]));
        assert!(is_valid_entry(&[6, 4, 1, 5]));
        assert!(is_valid_entry(&[6, 4, 2, 0]));
        assert!(!is_valid_entry(&[6, 4, 1, 0]));
        assert!(!is_valid_entry(&[6, 4, 3, 0]));
        assert!(!is_valid_entry(&[6, 100, 0, 0]));
    }
}
