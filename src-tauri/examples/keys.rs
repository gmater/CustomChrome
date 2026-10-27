//! Read-only: dumps the key assignment table (command 0x12) in the official app's chunked layout.

use std::time::{Duration, Instant};

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
    let packet_len = 32usize;
    let chunk = packet_len - 8;
    let total = 512usize;
    let mut table = vec![0u8; total];
    let mut stale = [0u8; 8192];
    while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}
    for (i, offset) in (0..total).step_by(chunk).enumerate() {
        let len = chunk.min(total - offset);
        let mut pkt = vec![0u8; packet_len + 1];
        pkt[1] = 0xAA;
        pkt[2] = 0x12;
        pkt[3] = len as u8;
        pkt[4] = offset as u8;
        pkt[5] = (offset >> 8) as u8;
        dev.write(&pkt).expect("write");
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut buf = [0u8; 8192];
        let mut got = false;
        while Instant::now() < deadline {
            let n = dev.read_timeout(&mut buf, 250).unwrap_or(0);
            if n > 8 && buf[1] == 0x12 {
                let (l, o) = (buf[2] as usize, buf[3] as usize | (buf[4] as usize) << 8);
                if i == 0 || i == 21 {
                    println!("chunk {i} reply header {:02X?}", &buf[..8]);
                }
                for k in 0..l {
                    if o + k < total && 8 + k < n {
                        table[o + k] = buf[8 + k];
                    }
                }
                got = true;
                break;
            }
        }
        if !got {
            println!("chunk {i} at {offset}: no reply");
        }
    }
    for (index, entry) in table.chunks(4).enumerate() {
        if entry.iter().any(|&b| b != 0) {
            println!("key {index:3}: {:02X?}", entry);
        }
    }
}
