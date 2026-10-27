pub mod aula;
mod diag;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use aula::protocol::{KeyEntry, KeyboardInfo, Lighting};
use aula::{device, Error};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager};

/// One keyboard conversation at a time: interleaved commands would swap each other's replies.
struct DeviceLock(Mutex<()>);

/// Long enough to queue behind a lighting write, short enough to fail fast during a screen upload.
const LOCK_WAIT: Duration = Duration::from_secs(3);

/// Commands that change the keyboard; their successes are logged too, not just their failures.
const WRITES: [&str; 6] = ["write_lighting", "assign_key", "save_macro", "delete_macro", "reset_keys", "repair_keyboard"];

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
            log_event,
            save_report
        ])
        .setup(|_| {
            diag::info(format!("CustomChrome {} started", env!("CARGO_PKG_VERSION")));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running CustomChrome");
}
