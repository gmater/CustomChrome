mod aula;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use aula::protocol::{KeyboardInfo, Lighting};
use aula::{device, Error};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager};

/// One keyboard conversation at a time: interleaved commands would swap each other's replies.
struct DeviceLock(Mutex<()>);

/// Long enough to queue behind a lighting write, short enough to fail fast during a screen upload.
const LOCK_WAIT: Duration = Duration::from_secs(3);

async fn exclusive<T: Send + 'static>(
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
    tauri::async_runtime::spawn_blocking(device::detect)
        .await
        .map_err(|_| Error::Interrupted)?
}

#[tauri::command]
async fn keyboard_info(app: AppHandle) -> Result<KeyboardInfo, Error> {
    exclusive(app, |_| device::read_info()).await
}

#[tauri::command]
async fn read_lighting(app: AppHandle) -> Result<Lighting, Error> {
    exclusive(app, |_| device::read_lighting()).await
}

#[tauri::command]
async fn write_lighting(app: AppHandle, lighting: Lighting) -> Result<Lighting, Error> {
    exclusive(app, move |_| device::write_lighting(&lighting)).await
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

    exclusive(app, move |app| {
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
            upload_screen
        ])
        .run(tauri::generate_context!())
        .expect("error while running CustomChrome");
}
