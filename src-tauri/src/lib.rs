mod aula;

use std::sync::Mutex;

use aula::{device, protocol::KeyboardInfo, Error};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager};

struct UploadLock(Mutex<()>);

#[tauri::command]
async fn detect_keyboard() -> Result<Option<device::KeyboardSummary>, Error> {
    tauri::async_runtime::spawn_blocking(device::detect)
        .await
        .map_err(|_| Error::Interrupted)?
}

#[tauri::command]
async fn keyboard_info() -> Result<KeyboardInfo, Error> {
    tauri::async_runtime::spawn_blocking(device::read_info)
        .await
        .map_err(|_| Error::Interrupted)?
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

    tauri::async_runtime::spawn_blocking(move || {
        let lock = app.state::<UploadLock>();
        let _guard = lock.0.try_lock().map_err(|_| Error::Busy)?;
        device::upload_screen(&rgba, &delays, |percent| {
            let _ = app.emit("upload-progress", percent);
        })
    })
    .await
    .map_err(|_| Error::Interrupted)?
}

pub fn run() {
    tauri::Builder::default()
        .manage(UploadLock(Mutex::new(())))
        .invoke_handler(tauri::generate_handler![detect_keyboard, keyboard_info, upload_screen])
        .run(tauri::generate_context!())
        .expect("error while running CustomChrome");
}
