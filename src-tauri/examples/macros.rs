//! Read-only: dumps the macro area (command 0x15) and decodes its index table.

use std::time::{Duration, Instant};

const TOTAL: usize = 1024;

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
    let chunk = 24usize;
    let mut area = vec![0u8; TOTAL];
    let mut stale = [0u8; 8192];
    while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}
    let count = TOTAL.div_ceil(chunk);
    for i in 0..count {
        let offset = i * chunk;
        let len = if i == count - 1 && TOTAL % chunk != 0 { TOTAL % chunk } else { chunk };
        let mut pkt = vec![0u8; 33];
        pkt[1..6].copy_from_slice(&[0xAA, 0x15, len as u8, offset as u8, (offset >> 8) as u8]);
        dev.write(&pkt).expect("write");
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut buf = [0u8; 8192];
        let mut got = false;
        while Instant::now() < deadline {
            let n = dev.read_timeout(&mut buf, 250).unwrap_or(0);
            if n > 8 && buf[1] == 0x15 {
                let (l, o) = (buf[2] as usize, buf[3] as usize | (buf[4] as usize) << 8);
                if i == 0 || i == count - 1 {
                    println!("chunk {i} reply header {:02X?}", &buf[..8]);
                }
                for k in 0..l {
                    if o + k < TOTAL && 8 + k < n {
                        area[o + k] = buf[8 + k];
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
    for m in 0..100 {
        let addr = u32::from_le_bytes(area[m * 4..m * 4 + 4].try_into().unwrap()) as usize;
        if addr == 0 {
            println!("{m} macros stored");
            break;
        }
        let n = if addr + 2 <= TOTAL { (area[addr] as usize | (area[addr + 1] as usize) << 8) / 2 } else { 0 };
        let end = (addr + 4 + n * 4).min(TOTAL);
        println!("macro {m} @ {addr}: {n} events {:02X?}", &area[addr.min(TOTAL)..end]);
    }
    let nonzero = area.iter().filter(|&&b| b != 0).count();
    println!("non-zero bytes in area: {nonzero}");
}
