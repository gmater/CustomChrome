//! Reads the key table (0x12), writes the identical bytes back (0x22) and reads again,
//! proving the write path and its acks without changing any key.

use std::time::{Duration, Instant};

const TOTAL: usize = 512;

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

fn packets(cmd: u8, packet_len: usize, table: &[u8]) -> Vec<Vec<u8>> {
    let chunk = packet_len - 8;
    (0..TOTAL)
        .step_by(chunk)
        .map(|offset| {
            let len = chunk.min(TOTAL - offset);
            let mut pkt = vec![0u8; packet_len + 1];
            pkt[1..6].copy_from_slice(&[0xAA, cmd, len as u8, offset as u8, (offset >> 8) as u8]);
            pkt[9..9 + len].copy_from_slice(&table[offset..offset + len]);
            pkt
        })
        .collect()
}

fn read(dev: &hidapi::HidDevice, packet_len: usize) -> Vec<u8> {
    let mut table = vec![0u8; TOTAL];
    for pkt in packets(0x12, packet_len, &[0u8; TOTAL]) {
        dev.write(&pkt).expect("write");
        let r = wait(dev, |r| r[1] == 0x12).expect("no read reply");
        let (l, o) = (r[2] as usize, r[3] as usize | (r[4] as usize) << 8);
        for k in 0..l.min(r.len().saturating_sub(8)) {
            if o + k < TOTAL {
                table[o + k] = r[8 + k];
            }
        }
    }
    table
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
    let packet_len = 32;
    let mut stale = [0u8; 8192];
    while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}

    let before = read(&dev, packet_len);
    let mut acks = Vec::new();
    for pkt in packets(0x22, packet_len, &before) {
        dev.write(&pkt).expect("write");
        let r = wait(&dev, |r| r[1] != 0x37).expect("no write ack");
        acks.push(r[..8].to_vec());
    }
    println!("{} write acks, first {:02X?}, last {:02X?}", acks.len(), acks[0], acks[acks.len() - 1]);
    let after = read(&dev, packet_len);
    println!("table unchanged after write-back: {}", before == after);
}
