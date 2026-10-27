//! Experimental: send the light-bar command (0x2B) the official app uses on models with a light bar.
//! Usage: lightbar <mode> <r> <g> <b> [brightness] [speed]

use std::time::{Duration, Instant};

fn transact(dev: &hidapi::HidDevice, cmd: u8, data: &[u8]) -> Option<Vec<u8>> {
    let mut stale = [0u8; 8192];
    while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}
    let len = 32;
    let mut pkt = vec![0u8; len + 1];
    pkt[1] = 0xAA;
    pkt[2] = cmd;
    pkt[3] = (len - 8) as u8;
    pkt[9..9 + data.len()].copy_from_slice(data);
    dev.write(&pkt).expect("write");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut buf = [0u8; 8192];
    while Instant::now() < deadline {
        let n = dev.read_timeout(&mut buf, 250).unwrap_or(0);
        if n > 1 && buf[1] != 0x37 {
            return Some(buf[..n].to_vec());
        }
    }
    None
}

fn main() {
    let args: Vec<u8> = std::env::args().skip(1).map(|a| a.parse().expect("number")).collect();
    let api = hidapi::HidApi::new().expect("hidapi");
    let info = api
        .device_list()
        .find(|d| {
            matches!((d.vendor_id(), d.product_id(), d.usage_page()), (0x0C45, 0x800A, 0xFF68) | (0x05AC, 0x024F, 0xFF60))
                && d.usage() == 0x61
        })
        .expect("no control interface");
    let dev = info.open_device(&api).expect("open");

    println!("lighting   {:02X?}", transact(&dev, 0x13, &[]).map(|r| r[8..24].to_vec()));
    println!("bar before {:02X?}", transact(&dev, 0x1B, &[]).map(|r| r[8..].to_vec()));
    if args.len() >= 4 {
        let mut bar = [0u8; 24];
        bar[0] = args[0];
        bar[1..4].copy_from_slice(&args[1..4]);
        bar[9] = *args.get(4).unwrap_or(&5);
        bar[10] = *args.get(5).unwrap_or(&3);
        println!("write ack  {:02X?}", transact(&dev, 0x2B, &bar));
        std::thread::sleep(Duration::from_millis(300));
        println!("bar after  {:02X?}", transact(&dev, 0x1B, &[]).map(|r| r[8..].to_vec()));
    }
}
