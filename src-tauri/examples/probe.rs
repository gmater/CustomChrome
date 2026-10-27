//! Read-only check of the control interface: device info and lighting state.

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
    println!("{:?} page {:04X}", info.product_string(), info.usage_page());
    let dev = info.open_device(&api).expect("open");
    let mut desc = [0u8; 4096];
    let n = dev.get_report_descriptor(&mut desc).expect("descriptor");
    println!("descriptor {:02X?}", &desc[..n]);

    for cmd in [0x10u8, 0x13, 0x1B] {
        let mut stale = [0u8; 8192];
        while dev.read_timeout(&mut stale, 0).unwrap_or(0) > 0 {}
        let len = 32;
        let mut pkt = vec![0u8; len + 1];
        pkt[1] = 0xAA;
        pkt[2] = cmd;
        pkt[3] = (len - 8) as u8;
        dev.write(&pkt).expect("write");
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut buf = [0u8; 8192];
        while Instant::now() < deadline {
            let n = dev.read_timeout(&mut buf, 250).unwrap_or(0);
            if n > 0 {
                println!("cmd {cmd:02X} reply {:02X?}", &buf[..n]);
                if buf[1] == cmd {
                    break;
                }
            }
        }
    }
}
