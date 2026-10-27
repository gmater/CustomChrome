use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};
use serde::Serialize;

use super::protocol::{self, KeyboardInfo};
use super::{Error, Result};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    Wired,
    Dongle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardSummary {
    pub name: &'static str,
    pub connection: Connection,
    pub product_id: u16,
    pub screen_upload: bool,
}

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const READ_SLICE_MS: i32 = 250;

pub fn detect() -> Result<Option<KeyboardSummary>> {
    let api = HidApi::new()?;
    let devices: Vec<_> = api.device_list().collect();
    if devices
        .iter()
        .any(|d| d.vendor_id() == protocol::VENDOR_ID && d.product_id() == protocol::PRODUCT_ID_WIRED)
    {
        return Ok(Some(KeyboardSummary {
            name: "AULA F99 MAX",
            connection: Connection::Wired,
            product_id: protocol::PRODUCT_ID_WIRED,
            screen_upload: true,
        }));
    }
    if devices
        .iter()
        .any(|d| d.vendor_id() == protocol::DONGLE_VENDOR_ID && d.product_id() == protocol::PRODUCT_ID_DONGLE)
    {
        return Ok(Some(KeyboardSummary {
            name: "AULA F99 MAX",
            connection: Connection::Dongle,
            product_id: protocol::PRODUCT_ID_DONGLE,
            screen_upload: false,
        }));
    }
    Ok(None)
}

fn open_wired(api: &HidApi, usage_page: u16, usage: Option<u16>, name: &'static str) -> Result<HidDevice> {
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == protocol::VENDOR_ID
                && d.product_id() == protocol::PRODUCT_ID_WIRED
                && d.usage_page() == usage_page
                && usage.is_none_or(|u| d.usage() == u)
        })
        .ok_or(Error::UnexpectedInterface(name))?;
    Ok(info.open_device(api)?)
}

fn require_wired(api: &HidApi) -> Result<()> {
    let has = |vid, pid| api.device_list().any(|d| d.vendor_id() == vid && d.product_id() == pid);
    if has(protocol::VENDOR_ID, protocol::PRODUCT_ID_WIRED) {
        Ok(())
    } else if has(protocol::DONGLE_VENDOR_ID, protocol::PRODUCT_ID_DONGLE) {
        Err(Error::WiredRequired)
    } else {
        Err(Error::NotConnected)
    }
}

fn output_len(device: &HidDevice) -> Option<usize> {
    let mut descriptor = [0u8; 4096];
    let len = device.get_report_descriptor(&mut descriptor).ok()?;
    protocol::output_report_len(&descriptor[..len])
}

fn write_report(device: &HidDevice, payload: &[u8]) -> Result<()> {
    let mut buf = Vec::with_capacity(payload.len() + 1);
    buf.push(0);
    buf.extend_from_slice(payload);
    device.write(&buf)?;
    Ok(())
}

fn wait_for(device: &HidDevice, mut accept: impl FnMut(&[u8]) -> bool) -> Result<Vec<u8>> {
    let deadline = Instant::now() + REPLY_TIMEOUT;
    let mut buf = vec![0u8; 8192];
    while Instant::now() < deadline {
        let n = device.read_timeout(&mut buf, READ_SLICE_MS)?;
        if n > 0 && accept(&buf[..n]) {
            return Ok(buf[..n].to_vec());
        }
    }
    Err(Error::Timeout)
}

fn read_info_with(api: &HidApi) -> Result<KeyboardInfo> {
    let control = open_wired(api, protocol::USAGE_PAGE_CONTROL, Some(protocol::USAGE_CONTROL), "control")?;
    let packet_len = output_len(&control).unwrap_or(64);
    if !(16..=1024).contains(&packet_len) {
        return Err(Error::UnexpectedInterface("control"));
    }
    write_report(&control, &protocol::device_info_request(packet_len))?;
    let reply = wait_for(&control, protocol::is_device_info_reply)?;
    Ok(protocol::parse_device_info(&reply))
}

pub fn read_info() -> Result<KeyboardInfo> {
    let api = HidApi::new()?;
    require_wired(&api)?;
    read_info_with(&api)
}

pub fn upload_screen(rgba: &[u8], delays_cs: &[u16], mut progress: impl FnMut(u8)) -> Result<u16> {
    if delays_cs.is_empty() || rgba.len() != delays_cs.len() * protocol::FRAME_RGBA_BYTES {
        return Err(Error::InvalidImage(format!(
            "expected {} bytes per {}x{} frame",
            protocol::FRAME_RGBA_BYTES,
            protocol::SCREEN_WIDTH,
            protocol::SCREEN_HEIGHT
        )));
    }

    let api = HidApi::new()?;
    require_wired(&api)?;
    let info = read_info_with(&api)?;
    if info.max_frames == 0 {
        return Err(Error::UnknownFrameLimit);
    }
    let frames = delays_cs
        .len()
        .min(info.max_frames as usize)
        .min(protocol::MAX_HEADER_FRAMES);

    let screen = open_wired(&api, protocol::USAGE_PAGE_SCREEN, None, "screen")?;
    if output_len(&screen) != Some(protocol::SCREEN_PACKET_LEN) {
        return Err(Error::UnexpectedInterface("screen"));
    }

    let header = protocol::screen_header(&delays_cs[..frames]);
    let pixels = protocol::rgba_to_rgb565_le(&rgba[..frames * protocol::FRAME_RGBA_BYTES]);
    let packets = protocol::screen_packets(&header, &pixels);

    progress(0);
    for (i, packet) in packets.iter().enumerate() {
        write_report(&screen, packet)?;
        wait_for(&screen, |reply| reply.starts_with(&protocol::ACK_SCREEN))?;
        progress(((i + 1) * 100 / packets.len()) as u8);
    }
    Ok(frames as u16)
}
