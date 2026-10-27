use std::time::{Duration, Instant};

use hidapi::{DeviceInfo, HidApi, HidDevice};
use serde::Serialize;

use super::protocol::{self, KeyEntry, KeyboardInfo, Lighting, Macro, LIGHTING_LEN};
use super::{Error, Result};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    Wired,
    Dongle,
}

impl Connection {
    fn of(d: &DeviceInfo) -> Option<Self> {
        match (d.vendor_id(), d.product_id()) {
            (protocol::VENDOR_ID, protocol::PRODUCT_ID_WIRED) => Some(Connection::Wired),
            (protocol::DONGLE_VENDOR_ID, protocol::PRODUCT_ID_DONGLE) => Some(Connection::Dongle),
            _ => None,
        }
    }

    fn expected_name(self) -> &'static str {
        match self {
            Connection::Wired => protocol::PRODUCT_NAME_WIRED,
            Connection::Dongle => protocol::PRODUCT_NAME_DONGLE,
        }
    }

    fn control_usage_page(self) -> u16 {
        match self {
            Connection::Wired => protocol::USAGE_PAGE_CONTROL,
            Connection::Dongle => protocol::USAGE_PAGE_CONTROL_DONGLE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardSummary {
    pub name: String,
    pub connection: Connection,
    pub supported: bool,
    pub screen_upload: bool,
}

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const READ_SLICE_MS: i32 = 250;
const CHUNK_TIMEOUT: Duration = Duration::from_millis(1500);
const CHUNK_ATTEMPTS: usize = 4;
const AREA_ATTEMPTS: usize = 3;
const SETTLE: Duration = Duration::from_millis(150);

/// Prefers the cable when both the cable and the dongle are plugged in.
fn find(api: &HidApi) -> Option<(Connection, String)> {
    let mut found: Option<(Connection, String)> = None;
    for d in api.device_list() {
        let Some(connection) = Connection::of(d) else { continue };
        if found.as_ref().is_some_and(|(c, _)| *c == Connection::Wired) {
            break;
        }
        found = Some((connection, d.product_string().unwrap_or_default().trim().to_string()));
    }
    found
}

pub fn detect() -> Result<Option<KeyboardSummary>> {
    let api = HidApi::new()?;
    Ok(find(&api).map(|(connection, name)| {
        let supported = name == connection.expected_name();
        KeyboardSummary {
            name: if name.is_empty() { "Unknown keyboard".into() } else { name },
            connection,
            supported,
            screen_upload: supported && connection == Connection::Wired,
        }
    }))
}

fn connected(api: &HidApi) -> Result<Connection> {
    let (connection, name) = find(api).ok_or(Error::NotConnected)?;
    if name != connection.expected_name() {
        return Err(Error::UnsupportedModel(name));
    }
    Ok(connection)
}

fn open_interface(
    api: &HidApi,
    connection: Connection,
    usage_page: u16,
    usage: Option<u16>,
    name: &'static str,
) -> Result<HidDevice> {
    let info = api
        .device_list()
        .find(|d| {
            Connection::of(d) == Some(connection)
                && d.usage_page() == usage_page
                && usage.is_none_or(|u| d.usage() == u)
        })
        .ok_or(Error::UnexpectedInterface(name))?;
    Ok(info.open_device(api)?)
}

struct Control {
    device: HidDevice,
    packet_len: usize,
}

fn open_control(api: &HidApi, connection: Connection) -> Result<Control> {
    let device = open_interface(
        api,
        connection,
        connection.control_usage_page(),
        Some(protocol::USAGE_CONTROL),
        "control",
    )?;
    let packet_len = output_len(&device).ok_or(Error::UnexpectedInterface("control"))?;
    if !(32..=1024).contains(&packet_len) {
        return Err(Error::UnexpectedInterface("control"));
    }
    Ok(Control { device, packet_len })
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

fn wait_for(device: &HidDevice, accept: impl FnMut(&[u8]) -> bool) -> Result<Vec<u8>> {
    wait_within(device, REPLY_TIMEOUT, accept)
}

fn wait_within(device: &HidDevice, timeout: Duration, mut accept: impl FnMut(&[u8]) -> bool) -> Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut buf = vec![0u8; 8192];
    while Instant::now() < deadline {
        let n = device.read_timeout(&mut buf, READ_SLICE_MS)?;
        if n > 0 && accept(&buf[..n]) {
            return Ok(buf[..n].to_vec());
        }
    }
    Err(Error::Timeout)
}

impl Control {
    /// Discards reports queued before a request so an old one can't be mistaken for the reply.
    fn drain(&self) -> Result<()> {
        let mut stale = vec![0u8; 8192];
        for _ in 0..64 {
            if self.device.read_timeout(&mut stale, 0)? == 0 {
                break;
            }
        }
        Ok(())
    }

    fn transact(&self, cmd: u8, data: &[u8], accept: impl FnMut(&[u8]) -> bool) -> Result<Vec<u8>> {
        self.drain()?;
        write_report(&self.device, &protocol::request(cmd, self.packet_len, data))?;
        wait_for(&self.device, accept)
    }

    fn read_info(&self) -> Result<KeyboardInfo> {
        let reply = self.transact(protocol::CMD_DEVICE_INFO, &[], |r| {
            protocol::is_reply_to(protocol::CMD_DEVICE_INFO, r)
        })?;
        Ok(protocol::parse_device_info(&reply))
    }

    fn read_lighting_bytes(&self) -> Result<[u8; LIGHTING_LEN]> {
        let reply = self.transact(protocol::CMD_READ_LIGHTING, &[], |r| {
            protocol::is_reply_to(protocol::CMD_READ_LIGHTING, r)
        })?;
        let mut bytes = [0u8; LIGHTING_LEN];
        protocol::merge_reply(&reply, &mut bytes);
        Ok(bytes)
    }
}

impl Control {
    /// Sends one chunk and waits for the reply that echoes its command and offset, resending it
    /// when the keyboard misses it. Chunks are idempotent, so a resend is always safe.
    fn send_chunk(&self, packet: &[u8]) -> Result<Vec<u8>> {
        for _ in 0..CHUNK_ATTEMPTS {
            write_report(&self.device, packet)?;
            match wait_within(&self.device, CHUNK_TIMEOUT, |r| protocol::acknowledges(packet, r)) {
                Err(Error::Timeout) => continue,
                other => return other,
            }
        }
        Err(Error::Timeout)
    }

    fn read_area(&self, cmd: u8, len: usize) -> Result<Vec<u8>> {
        let mut area = vec![0u8; len];
        self.drain()?;
        for packet in protocol::chunked_requests(cmd, self.packet_len, &area.clone()) {
            let reply = self.send_chunk(&packet)?;
            protocol::merge_reply(&reply, &mut area);
        }
        Ok(area)
    }

    /// The keyboard wipes a table when its first chunk arrives, so a transfer that stops part way
    /// leaves keys dead. Every chunk must be acknowledged, and the table is read back to confirm.
    fn write_area(&self, cmd: u8, read_cmd: u8, area: &[u8]) -> Result<()> {
        std::thread::sleep(SETTLE);
        self.drain()?;
        let mut result = Ok(());
        for _ in 0..AREA_ATTEMPTS {
            result = protocol::chunked_requests(cmd, self.packet_len, area)
                .iter()
                .try_for_each(|packet| self.send_chunk(packet).map(drop));
            if result.is_ok() {
                std::thread::sleep(SETTLE);
                if self.read_area(read_cmd, area.len())? == area {
                    return Ok(());
                }
                result = Err(Error::WriteNotVerified);
            }
            std::thread::sleep(SETTLE);
            self.drain()?;
        }
        result
    }

    fn read_key_table(&self) -> Result<Vec<u8>> {
        self.read_area(protocol::CMD_READ_KEYS, protocol::KEY_TABLE_LEN)
    }

    fn write_key_table(&self, table: &[u8]) -> Result<()> {
        self.write_area(protocol::CMD_WRITE_KEYS, protocol::CMD_READ_KEYS, table)
    }

    fn read_macros(&self) -> Result<Vec<Macro>> {
        Ok(protocol::decode_macros(&self.read_area(protocol::CMD_READ_MACROS, protocol::MACRO_AREA_LEN)?))
    }

    fn write_macros(&self, macros: &[Macro]) -> Result<()> {
        let area = protocol::encode_macros(macros).ok_or(Error::InvalidMacro)?;
        self.write_area(protocol::CMD_WRITE_MACROS, protocol::CMD_READ_MACROS, &area)
    }
}

#[derive(Serialize)]
pub struct MacroState {
    pub macros: Vec<Macro>,
    pub keys: Vec<KeyEntry>,
}

pub fn read_macros() -> Result<MacroState> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    let macros = control.read_macros()?;
    Ok(MacroState { macros, keys: key_entries(&control.read_key_table()?) })
}

/// Replaces macro `index`, or appends a new one when `index` equals the macro count.
pub fn save_macro(index: usize, events: Macro) -> Result<MacroState> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    let mut macros = control.read_macros()?;
    match index.cmp(&macros.len()) {
        std::cmp::Ordering::Less => macros[index] = events,
        std::cmp::Ordering::Equal => macros.push(events),
        std::cmp::Ordering::Greater => return Err(Error::InvalidMacro),
    }
    control.write_macros(&macros)?;
    Ok(MacroState { macros, keys: key_entries(&control.read_key_table()?) })
}

/// Deletes a macro, restoring keys that played it and renumbering keys bound to later macros.
pub fn delete_macro(index: usize) -> Result<MacroState> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    let mut macros = control.read_macros()?;
    if index >= macros.len() {
        return Err(Error::InvalidMacro);
    }
    macros.remove(index);
    let mut table = control.read_key_table()?;
    let before = table.clone();
    protocol::shift_macro_refs(&mut table, index as u8);
    if table != before {
        control.write_key_table(&table)?;
    }
    control.write_macros(&macros)?;
    Ok(MacroState { macros, keys: key_entries(&table) })
}

fn key_entries(table: &[u8]) -> Vec<KeyEntry> {
    table.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect()
}

pub fn read_keys() -> Result<Vec<KeyEntry>> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    Ok(key_entries(&control.read_key_table()?))
}

/// `entry: None` restores the key's factory assignment.
pub fn assign_key(index: u8, entry: Option<KeyEntry>) -> Result<Vec<KeyEntry>> {
    if !protocol::is_remappable(index) {
        return Err(Error::InvalidKey);
    }
    let entry = match entry {
        Some(e) if protocol::is_valid_entry(&e) => protocol::canonical_entry(e),
        Some(_) => return Err(Error::InvalidKey),
        None => protocol::factory_entry(index).ok_or(Error::InvalidKey)?,
    };
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    if entry[0] == protocol::MACRO_KIND && entry[1] as usize >= control.read_macros()?.len() {
        return Err(Error::InvalidMacro);
    }
    let mut table = control.read_key_table()?;
    let at = index as usize * 4;
    table[at..at + 4].copy_from_slice(&entry);
    control.write_key_table(&table)?;
    Ok(key_entries(&table))
}

pub fn reset_keys() -> Result<Vec<KeyEntry>> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    let mut table = control.read_key_table()?;
    for &(index, _) in protocol::F99_KEYS.iter().filter(|(i, _)| protocol::is_remappable(*i)) {
        let at = index as usize * 4;
        table[at..at + 4].copy_from_slice(&protocol::factory_entry(index).unwrap());
    }
    control.write_key_table(&table)?;
    Ok(key_entries(&table))
}

pub fn read_info() -> Result<KeyboardInfo> {
    let api = HidApi::new()?;
    open_control(&api, connected(&api)?)?.read_info()
}

pub fn read_lighting() -> Result<Lighting> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    Ok(Lighting::from_bytes(&control.read_lighting_bytes()?))
}

pub fn write_lighting(lighting: &Lighting) -> Result<Lighting> {
    let api = HidApi::new()?;
    let control = open_control(&api, connected(&api)?)?;
    let current = control.read_lighting_bytes()?;
    if !Lighting::is_supported_effect(lighting.effect) && lighting.effect != current[0] {
        return Err(Error::UnsupportedEffect);
    }
    let bytes = lighting.apply_to(&current);
    control.transact(protocol::CMD_WRITE_LIGHTING, &bytes, |r| {
        r.len() > 1 && r[1] != protocol::CMD_STATUS
    })?;
    Ok(Lighting::from_bytes(&bytes))
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
    if connected(&api)? != Connection::Wired {
        return Err(Error::WiredRequired);
    }
    let info = open_control(&api, Connection::Wired)?.read_info()?;
    if info.max_frames == 0 {
        return Err(Error::UnknownFrameLimit);
    }
    let frames = delays_cs
        .len()
        .min(info.max_frames as usize)
        .min(protocol::MAX_HEADER_FRAMES);

    let screen = open_interface(&api, Connection::Wired, protocol::USAGE_PAGE_SCREEN, None, "screen")?;
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
