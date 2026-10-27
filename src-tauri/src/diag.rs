//! Local diagnostics: a plain-text log and problem reports kept in `logs\` next to the exe.
//! Nothing leaves the PC; the files are there for whoever is helping the user.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_NAME: &str = "customchrome.log";
const LOG_MAX_BYTES: u64 = 1024 * 1024;
const TAIL_LINES: usize = 300;

static WRITE: Mutex<()> = Mutex::new(());

pub fn dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("logs")))
        .unwrap_or_else(|| PathBuf::from("logs"))
}

/// UTC timestamp without pulling in a date crate.
pub fn timestamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub fn log(level: &str, message: impl AsRef<str>) {
    let _guard = WRITE.lock();
    let dir = dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(LOG_NAME);
    if fs::metadata(&path).is_ok_and(|m| m.len() > LOG_MAX_BYTES) {
        let _ = fs::rename(&path, dir.join(format!("{LOG_NAME}.old")));
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{} {level:<5} {}", timestamp(), message.as_ref().replace('\n', " | "));
    }
}

pub fn info(message: impl AsRef<str>) {
    log("INFO", message);
}

pub fn warn(message: impl AsRef<str>) {
    log("WARN", message);
}

pub fn error(message: impl AsRef<str>) {
    log("ERROR", message);
}

fn tail() -> String {
    let text = fs::read_to_string(dir().join(LOG_NAME)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n")
}

/// Saves the user's description, the app's view of the keyboard and the recent log as one file.
pub fn save_report(description: &str, context: &str) -> std::io::Result<PathBuf> {
    let dir = dir().join("reports");
    fs::create_dir_all(&dir)?;
    let stamp = timestamp().replace([':', ' '], "-");
    let path = dir.join(format!("report-{stamp}.txt"));
    let body = format!(
        "CustomChrome {} problem report, {}\n\nWhat happened (from the user):\n{}\n\nKeyboard state:\n{}\n\nRecent log:\n{}\n",
        env!("CARGO_PKG_VERSION"),
        timestamp(),
        description.trim(),
        context,
        tail()
    );
    fs::write(&path, body)?;
    warn(format!("problem report saved: {}", path.display()));
    Ok(path)
}
