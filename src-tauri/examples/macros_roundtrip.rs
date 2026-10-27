//! Writes one unassigned test macro (tap A) to an empty macro area (0x25), reads it back (0x15),
//! then restores the empty area. Refuses to run if the keyboard already holds macros.

use std::time::{Duration, Instant};

const TOTAL: usize = 1024;
const PACKET: usize = 32;

fn wait(dev: &hidapi::HidDevice, accept: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut buf = [0u8; 8192];
    while Instant::now() < deadline {
        let n = dev.read_timeout(&mut buf, 250).unwrap_or(0);
        if n > 1 && accept(&buf[..n]) {
            return Some(buf[..n].to_vec());
        }
    }
    None
}

fn packets(cmd: u8, area: &[u8]) -> Vec<Vec<u8>> {
    let chunk = PACKET - 8;
    let count = TOTAL.div_ceil(chunk);
    (0..count)
        .map(|i| {
            let offset = i * chunk;
            let len = if i == count - 1 && i != 0 && TOTAL % chunk != 0 { TOTAL % chunk } else { chunk };
            let mut pkt = vec![0u8; PACKET + 1];
            pkt[1..6].copy_from_slice(&[0xAA, cmd, len as u8, offset as u8, (offset >> 8) as u8]);
            let data = &area[offset..(offset + chunk).min(TOTAL)];
            pkt[9..9 + data.len()].copy_from_slice(data);
            pkt
        })
        .collect()
}

fn read(dev: &hidapi::HidDevice) -> Vec<u8> {
    let mut area = vec![0u8; TOTAL];
    for pkt in packets(0x15, &[0u8; TOTAL]) {
        dev.write(&pkt).expect("write");
        let r = wait(dev, |r| r.len() > 8 && r[1] == 0x15).expect("no read reply");
        let (l, o) = (r[2] as usize, r[3] as usize | (r[4] as usize) << 8);
        for k in 0..l.min(r.len() - 8) {
            if o + k < TOTAL {
                area[o + k] = r[8 + k];
            }
        }
    }
    area
}

fn write(dev: &hidapi::HidDevice, area: &[u8]) -> usize {
    let mut acks = 0;
    for pkt in packets(0x25, area) {
        dev.write(&pkt).expect("write");
        wait(dev, |r| r[1] != 0x37).expect("no write ack");
        acks += 1;
    }
    acks
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
    while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}

    let original = read(&dev);
    if original.iter().any(|&b| b != 0) {
        println!("keyboard already has macros; not touching them");
        return;
    }

    let mut test = vec![0u8; TOTAL];
    test[0..4].copy_from_slice(&400u32.to_le_bytes());
    test[400..412].copy_from_slice(&[4, 0, 0, 0, 20, 0, 0x04, 0xB0, 20, 0, 0x04, 0x30]);
    println!("test macro: {} write acks", write(&dev, &test));
    let back = read(&dev);
    println!("read back matches: {} (first bytes {:02X?} / {:02X?})", back == test, &back[0..4], &back[400..412]);

    println!("restore: {} write acks", write(&dev, &original));
    println!("restored to empty: {}", read(&dev) == original);
}
