//! Read-only: `dump <cmd hex> <len>` reads an area with the official chunked read and prints
//! the non-zero 4-byte entries. Refuses write commands (0x20 and up).

use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = u8::from_str_radix(args.next().expect("cmd").trim_start_matches("0x"), 16).expect("hex cmd");
    let total: usize = args.next().expect("len").parse().expect("len");
    assert!((0x10..0x20).contains(&cmd), "read commands only");

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

    let chunk = 24;
    let count = total.div_ceil(chunk);
    let mut area = vec![0u8; total];
    for i in 0..count {
        let offset = i * chunk;
        let len = if i == count - 1 && i != 0 && total % chunk != 0 { total % chunk } else { chunk };
        let mut pkt = vec![0u8; 33];
        pkt[1..6].copy_from_slice(&[0xAA, cmd, len as u8, offset as u8, (offset >> 8) as u8]);
        dev.write(&pkt).expect("write");
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut buf = [0u8; 8192];
        let mut got = false;
        while Instant::now() < deadline {
            let n = dev.read_timeout(&mut buf, 100).unwrap_or(0);
            if n > 8 && buf[1] == cmd && buf[3] == pkt[4] && buf[4] == pkt[5] {
                if i == 0 {
                    println!("reply header {:02X?}", &buf[..8]);
                }
                let l = buf[2] as usize;
                for k in 0..l.min(n - 8) {
                    if offset + k < total {
                        area[offset + k] = buf[8 + k];
                    }
                }
                got = true;
                break;
            }
        }
        if !got {
            println!("chunk {i}: no reply");
        }
    }
    for (index, e) in area.chunks(4).enumerate() {
        if e.iter().any(|&b| b != 0) {
            println!("{index:3}: {:02X?}", e);
        }
    }
}
