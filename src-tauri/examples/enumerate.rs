fn main() {
    let api = hidapi::HidApi::new().expect("hidapi");
    for d in api.device_list() {
        if !matches!(d.vendor_id(), 0x0C45 | 0x05AC) {
            continue;
        }
        let output_len = d.open_device(&api).ok().and_then(|dev| {
            let mut buf = [0u8; 4096];
            let n = dev.get_report_descriptor(&mut buf).ok()?;
            Some(buf[..n].to_vec())
        });
        println!(
            "{:04X}:{:04X} if={} page={:04X} usage={:04X} product={:?} descriptor={:02X?}",
            d.vendor_id(),
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            d.product_string(),
            output_len.map(|v| v.len())
        );
    }
}
