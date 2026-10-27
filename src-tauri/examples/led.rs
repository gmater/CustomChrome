//! Writes the main lighting block (0x23) and reads it back.
//! Usage: led <effect> <r> <g> <b> <rainbow 0|1> <brightness> <speed>

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
    let a: Vec<u8> = std::env::args().skip(1).map(|a| a.parse().expect("number")).collect();
    let api = hidapi::HidApi::new().expect("hidapi");
    let info = api
        .device_list()
        .find(|d| {
            matches!((d.vendor_id(), d.product_id(), d.usage_page()), (0x0C45, 0x800A, 0xFF68) | (0x05AC, 0x024F, 0xFF60))
                && d.usage() == 0x61
        })
        .expect("no control interface");
    let dev = info.open_device(&api).expect("open");
    let before = transact(&dev, 0x13, &[]).expect("read")[8..24].to_vec();
    println!("before {:02X?}", before);
    let mut b = before.clone();
    b[0] = a[0];
    b[1..4].copy_from_slice(&a[1..4]);
    b[4] = 0xFF;
    b[8] = a[4];
    b[9] = a[5];
    b[10] = a[6];
    b[14] = 0xAA;
    b[15] = 0x55;
    println!("ack    {:02X?}", transact(&dev, 0x23, &b));
    std::thread::sleep(Duration::from_millis(300));
    println!("after  {:02X?}", transact(&dev, 0x13, &[]).map(|r| r[8..24].to_vec()));
}
