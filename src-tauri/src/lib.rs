pub mod aula;
mod diag;
mod profiles;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use aula::protocol::{BoardSettings, ClockTime, KeyEntry, KeyboardInfo, Lighting};
use aula::{device, Error};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager};

/// One keyboard conversation at a time: interleaved commands would swap each other's replies.
pub(crate) struct DeviceLock(pub(crate) Mutex<()>);

/// Long enough to wait behind one slow read, without a page giving up and saying the keyboard is busy.
const LOCK_WAIT: Duration = Duration::from_secs(8);

/// Commands that change the keyboard; their successes are logged too, not just their failures.
const WRITES: [&str; 12] = [
    "write_lighting",
    "assign_key",
    "save_macro",
    "delete_macro",
    "reset_keys",
    "repair_keyboard",
    "write_board",
    "sync_clock",
    "factory_reset",
    "save_profile",
    "apply_profile",
    "save_program",
];

async fn exclusive<T: Send + 'static>(
    app: AppHandle,
    name: &'static str,
    task: impl FnOnce(&AppHandle) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    let started = Instant::now();
    let result = run_exclusive(app, task).await;
    let ms = started.elapsed().as_millis();
    match &result {
        Err(e) => diag::error(format!("{name} failed after {ms} ms: {e}")),
        Ok(_) if WRITES.contains(&name) || name == "upload_screen" => diag::info(format!("{name} ok ({ms} ms)")),
        Ok(_) => {}
    }
    result
}

async fn run_exclusive<T: Send + 'static>(
    app: AppHandle,
    task: impl FnOnce(&AppHandle) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let lock = app.state::<DeviceLock>();
        let deadline = Instant::now() + LOCK_WAIT;
        let _guard = loop {
            match lock.0.try_lock() {
                Ok(guard) => break guard,
                Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => return Err(Error::Busy),
            }
        };
        task(&app)
    })
    .await
    .map_err(|_| Error::Interrupted)?
}

#[tauri::command]
async fn detect_keyboard() -> Result<Option<device::KeyboardSummary>, Error> {
    static LAST: Mutex<Option<String>> = Mutex::new(None);
    let found = tauri::async_runtime::spawn_blocking(device::detect)
        .await
        .map_err(|_| Error::Interrupted)??;
    let now = match &found {
        Some(k) => format!("{} over {:?} (supported: {})", k.name, k.connection, k.supported),
        None => "no keyboard".to_string(),
    };
    if let Ok(mut last) = LAST.lock() {
        if last.as_deref() != Some(now.as_str()) {
            diag::info(format!("keyboard: {now}"));
            *last = Some(now);
        }
    }
    Ok(found)
}

#[tauri::command]
async fn read_board(app: AppHandle) -> Result<BoardSettings, Error> {
    exclusive(app, "read_board", |_| device::read_board()).await
}

#[tauri::command]
async fn write_board(app: AppHandle, settings: BoardSettings) -> Result<BoardSettings, Error> {
    diag::info(format!("write_board sleep {} min, 24h {}", settings.sleep_minutes, settings.hour24));
    exclusive(app, "write_board", move |_| device::write_board(settings)).await
}

#[tauri::command]
async fn sync_clock(app: AppHandle, time: ClockTime) -> Result<(), Error> {
    diag::info(format!(
        "sync_clock {:04}-{:02}-{:02} {:02}:{:02}:{:02} weekday {}",
        time.year, time.month, time.day, time.hour, time.minute, time.second, time.weekday
    ));
    exclusive(app, "sync_clock", move |_| device::sync_clock(&time)).await
}

#[tauri::command]
async fn factory_reset(app: AppHandle) -> Result<(), Error> {
    exclusive(app, "factory_reset", |_| device::factory_reset()).await
}

#[tauri::command]
fn open_logs() -> Result<(), String> {
    let dir = diag::dir();
    let _ = std::fs::create_dir_all(&dir);
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn keyboard_info(app: AppHandle) -> Result<KeyboardInfo, Error> {
    exclusive(app, "keyboard_info", |_| device::read_info()).await
}

#[tauri::command]
async fn read_lighting(app: AppHandle) -> Result<Lighting, Error> {
    exclusive(app, "read_lighting", |_| device::read_lighting()).await
}

#[tauri::command]
async fn write_lighting(app: AppHandle, lighting: Lighting) -> Result<Lighting, Error> {
    exclusive(app, "write_lighting", move |_| device::write_lighting(&lighting)).await
}

#[tauri::command]
async fn read_keys(app: AppHandle) -> Result<Vec<KeyEntry>, Error> {
    exclusive(app, "read_keys", |_| device::read_keys()).await
}

#[tauri::command]
async fn assign_key(app: AppHandle, index: u8, entry: Option<KeyEntry>) -> Result<Vec<KeyEntry>, Error> {
    diag::info(format!("assign_key key {index} -> {entry:02X?}"));
    exclusive(app, "assign_key", move |_| device::assign_key(index, entry)).await
}

#[tauri::command]
async fn read_macros(app: AppHandle) -> Result<device::MacroState, Error> {
    exclusive(app, "read_macros", |_| device::read_macros()).await
}

#[tauri::command]
async fn save_macro(app: AppHandle, index: usize, events: aula::protocol::Macro) -> Result<device::MacroState, Error> {
    diag::info(format!("save_macro slot {index}, {} steps", events.len()));
    exclusive(app, "save_macro", move |_| device::save_macro(index, events)).await
}

#[tauri::command]
async fn delete_macro(app: AppHandle, index: usize) -> Result<device::MacroState, Error> {
    diag::info(format!("delete_macro slot {index}"));
    exclusive(app, "delete_macro", move |_| device::delete_macro(index)).await
}

#[tauri::command]
async fn check_keyboard(app: AppHandle) -> Result<device::Health, Error> {
    let health = exclusive(app, "check_keyboard", |_| device::check()).await?;
    if health.issues.is_empty() {
        diag::info(format!("health check: ok, {} macros, {} remapped keys", health.macros, health.remapped.len()));
    } else {
        diag::warn(format!("health check found problems: {:?}", health.issues));
    }
    Ok(health)
}

#[tauri::command]
async fn repair_keyboard(app: AppHandle) -> Result<device::Health, Error> {
    let health = exclusive(app, "repair_keyboard", |_| device::repair()).await?;
    diag::info(format!("after repair: {:?}", health.issues));
    Ok(health)
}

#[tauri::command]
fn log_event(level: String, message: String) {
    let message: String = message.chars().take(2000).collect();
    match level.as_str() {
        "error" => diag::error(format!("ui: {message}")),
        "warn" => diag::warn(format!("ui: {message}")),
        _ => diag::info(format!("ui: {message}")),
    }
}

#[tauri::command]
fn save_report(description: String, context: String) -> Result<String, String> {
    let description: String = description.chars().take(5000).collect();
    let context: String = context.chars().take(20_000).collect();
    diag::save_report(&description, &context)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn macro_capacity() -> usize {
    aula::protocol::MACRO_WORDS
}

#[tauri::command]
fn factory_keys() -> Vec<(u8, KeyEntry)> {
    aula::protocol::F99_KEYS
        .iter()
        .filter_map(|&(index, _)| aula::protocol::factory_entry(index).map(|e| (index, e)))
        .collect()
}

#[tauri::command]
async fn reset_keys(app: AppHandle) -> Result<Vec<KeyEntry>, Error> {
    exclusive(app, "reset_keys", |_| device::reset_keys()).await
}

#[tauri::command]
async fn upload_screen(app: AppHandle, request: Request<'_>) -> Result<u16, Error> {
    let InvokeBody::Raw(rgba) = request.body() else {
        return Err(Error::InvalidImage("expected raw frame data".into()));
    };
    let delays = request
        .headers()
        .get("x-delays")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::InvalidImage("missing frame delays".into()))?
        .split(',')
        .map(|d| d.trim().parse::<u16>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::InvalidImage("frame delays must be whole numbers".into()))?;
    let rgba = rgba.clone();

    exclusive(app, "upload_screen", move |app| {
        device::upload_screen(&rgba, &delays, |percent| {
            let _ = app.emit("upload-progress", percent);
        })
    })
    .await
}

#[tauri::command]
async fn apply_profile(app: AppHandle, id: String) -> Result<profiles::ProfileState, Error> {
    exclusive(app, "apply_profile", move |app| {
        let bundle = profiles::profile_for_apply(&id)?;
        let applied = device::apply_snapshot(&bundle.lighting, &bundle.keys, bundle.macros.as_deref())?;
        diag::info(format!(
            "profile '{}' applied by hand (keys rewritten: {}, macros rewritten: {})",
            bundle.name, applied.keys_changed, applied.macros_changed
        ));
        let restart = applied.macros_changed && bundle.keys.iter().any(|entry| entry[0] == aula::protocol::MACRO_KIND);
        profiles::note_applied(&id, None, bundle.macros.is_some(), &bundle.macro_names, restart);
        let _ = app.emit(
            "profile-activity",
            serde_json::json!({
                "phase": "applied",
                "profileId": id,
                "profileName": bundle.name,
                "exe": "",
                "macroNames": bundle.macro_names,
                "hasMacros": bundle.macros.is_some(),
                "macrosChanged": applied.macros_changed,
                "restart": restart,
            }),
        );
        Ok(profiles::state())
    })
    .await
}

#[tauri::command]
fn profile_state() -> profiles::ProfileState {
    profiles::state()
}

#[tauri::command]
async fn save_profile(app: AppHandle, name: String, macro_names: Vec<String>) -> Result<profiles::ProfileState, Error> {
    exclusive(app, "save_profile", move |_| {
        let snap = device::read_snapshot()?;
        profiles::save_current(&name, snap.lighting, snap.keys, snap.macros, macro_names)
    })
    .await
}

#[tauri::command]
fn rename_profile(id: String, name: String) -> Result<profiles::ProfileState, Error> {
    profiles::rename(&id, &name)
}

#[tauri::command]
fn delete_profile(id: String) -> Result<profiles::ProfileState, Error> {
    profiles::delete(&id)
}

#[tauri::command]
fn set_profile_auto(on: bool) -> Result<profiles::ProfileState, Error> {
    profiles::set_auto(on)
}

#[tauri::command]
fn set_default_profile(id: Option<String>) -> Result<profiles::ProfileState, Error> {
    profiles::set_default(id)
}

#[tauri::command]
fn assign_profile_app(exe: String, label: String, path: String, profile_id: String) -> Result<profiles::ProfileState, Error> {
    profiles::assign_app(&exe, &label, &path, &profile_id)
}

#[tauri::command]
fn unassign_profile_app(exe: String) -> Result<profiles::ProfileState, Error> {
    profiles::unassign_app(&exe)
}

#[tauri::command]
fn open_apps() -> Vec<profiles::OpenApp> {
    profiles::open_apps()
}

#[tauri::command]
async fn save_program(app: AppHandle, exe: String, label: String, path: String, macro_names: Vec<String>) -> Result<profiles::ProfileState, Error> {
    exclusive(app, "save_program", move |_| {
        let snap = device::read_snapshot()?;
        profiles::adopt_program(&exe, &label, &path, snap.lighting, snap.keys, snap.macros, macro_names)
    })
    .await
}

#[tauri::command]
fn arm_program_capture(macro_names: Vec<String>) -> Result<profiles::ProfileState, Error> {
    profiles::arm_new(macro_names)
}

#[tauri::command]
fn cancel_profile_capture() -> profiles::ProfileState {
    profiles::disarm()
}

pub fn run() {
    tauri::Builder::default()
        .manage(DeviceLock(Mutex::new(())))
        .invoke_handler(tauri::generate_handler![
            detect_keyboard,
            keyboard_info,
            read_lighting,
            write_lighting,
            read_keys,
            factory_keys,
            read_macros,
            save_macro,
            delete_macro,
            macro_capacity,
            assign_key,
            reset_keys,
            upload_screen,
            check_keyboard,
            repair_keyboard,
            read_board,
            write_board,
            sync_clock,
            factory_reset,
            open_logs,
            log_event,
            save_report,
            profile_state,
            save_profile,
            rename_profile,
            delete_profile,
            apply_profile,
            set_profile_auto,
            set_default_profile,
            assign_profile_app,
            unassign_profile_app,
            save_program,
            open_apps,
            arm_program_capture,
            cancel_profile_capture
        ])
        .setup(|app| {
            diag::info(format!("CustomChrome {} started", env!("CARGO_PKG_VERSION")));
            profiles::spawn(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running CustomChrome");
}
