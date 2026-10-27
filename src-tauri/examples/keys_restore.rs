//! Recovery: writes a complete factory key table (physical keys, Fn, volume knob; zeros elsewhere)
//! with per-chunk retries and logging, then reads it back to verify.

use std::time::{Duration, Instant};

const PACKET: usize = 32;
const TOTAL: usize = 512;

const F99_KEYS: &[(usize, u8)] = &[
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
    (80, 0xE0), (81, 0xE3), (82, 0xE2), (83, 0x2C), (84, 0xE6), (85, 0xAF), (88, 0x50),
    (89, 0x51), (91, 0x4F), (93, 0x62), (94, 0x63),
];

fn reply(dev: &hidapi::HidDevice, timeout: Duration, accept: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut buf = [0u8; 8192];
    while Instant::now() < deadline {
        let n = dev.read_timeout(&mut buf, 100).unwrap_or(0);
        if n > 1 && accept(&buf[..n]) {
            return Some(buf[..n].to_vec());
        }
    }
    None
}

fn packet(cmd: u8, i: usize, area: &[u8]) -> Vec<u8> {
    let chunk = PACKET - 8;
    let count = area.len().div_ceil(chunk);
    let offset = i * chunk;
    let len = if i == count - 1 && i != 0 && area.len() % chunk != 0 { area.len() % chunk } else { chunk };
    let mut pkt = vec![0u8; PACKET + 1];
    pkt[1..6].copy_from_slice(&[0xAA, cmd, len as u8, offset as u8, (offset >> 8) as u8]);
    let data = &area[offset..(offset + chunk).min(area.len())];
    pkt[9..9 + data.len()].copy_from_slice(data);
    pkt
}

fn main() {
    let api = hidapi::HidApi::new().expect("hidapi");
    let info = api
        .device_list()
        .find(|d| {
            matches!((d.vendor_id(), d.product_id(), d.usage_page()), (0x0C45, 0x800A, 0xFF68) | (0x05AC, 0x024F, 0xFF60))
                && d.usage() == 0x61
        })
        .expect("no control interface");
    let dev = info.open_device(&api).expect("open");
    let mut stale = [0u8; 8192];
    while dev.read_timeout(&mut stale, 50).unwrap_or(0) > 0 {}

    let mut table = vec![0u8; TOTAL];
    for &(i, hid) in F99_KEYS {
        table[i * 4..i * 4 + 4].copy_from_slice(&[2, 0, hid, 0]);
    }
    table[13 * 4..14 * 4].copy_from_slice(&[3, 0xE9, 0, 0]);
    table[14 * 4..15 * 4].copy_from_slice(&[3, 0xEA, 0, 0]);

    let count = TOTAL.div_ceil(PACKET - 8);
    for i in 0..count {
        let pkt = packet(0x22, i, &table);
        let mut ok = false;
        for attempt in 1..=5 {
            dev.write(&pkt).expect("write");
            let off = [pkt[4], pkt[5]];
            if let Some(r) = reply(&dev, Duration::from_secs(2), |r| r[1] == 0x22 && r[3] == off[0] && r[4] == off[1]) {
                if attempt > 1 {
                    println!("chunk {i} acked on attempt {attempt}: {:02X?}", &r[..8]);
                }
                ok = true;
                break;
            }
            println!("chunk {i}: no ack (attempt {attempt})");
        }
        if !ok {
            println!("giving up at chunk {i}");
            return;
        }
    }

    let mut back = vec![0u8; TOTAL];
    for i in 0..count {
        let pkt = packet(0x12, i, &vec![0u8; TOTAL]);
        dev.write(&pkt).expect("write");
        let r = reply(&dev, Duration::from_secs(3), |r| r.len() > 8 && r[1] == 0x12).expect("no read reply");
        let (l, o) = (r[2] as usize, r[3] as usize | (r[4] as usize) << 8);
        for k in 0..l.min(r.len() - 8) {
            if o + k < TOTAL {
                back[o + k] = r[8 + k];
            }
        }
    }
    println!("key table restored and verified: {}", back == table);
}
