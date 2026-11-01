//! Named lights-and-keys profiles, and which program should bring each one back.
//! The file sits next to the exe as `profiles.json`. Nothing is sent online.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::aula::device;
use crate::aula::protocol::{self, KeyEntry, Lighting, Macro};
use crate::aula::Error;
use crate::diag;

const MAX_PROFILES: usize = 24;
const MAX_APPS: usize = 40;
const NAME_MAX: usize = 40;

/// Shell windows that steal the foreground without being a program the user opened.
const SHELL: &[&str] = &[
    "searchhost.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "applicationframehost.exe",
    "textinputhost.exe",
    "lockapp.exe",
    "dwm.exe",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: String,
    name: String,
    lighting: Lighting,
    keys: Vec<KeyEntry>,
    /// Missing on profiles saved before macros were included. Those leave the keyboard's macros alone.
    #[serde(default)]
    macros: Option<Vec<Macro>>,
    #[serde(default)]
    macro_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppLink {
    exe: String,
    label: String,
    profile_id: String,
    /// Full path of the program, used to load its real icon. Empty on older saves until the program is open.
    #[serde(default)]
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Store {
    #[serde(default)]
    auto: bool,
    #[serde(default)]
    default_profile: Option<String>,
    #[serde(default)]
    profiles: Vec<Profile>,
    #[serde(default)]
    apps: Vec<AppLink>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub exe: String,
    pub label: String,
    pub profile_id: String,
    pub path: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileState {
    pub auto: bool,
    pub default_profile: Option<String>,
    pub profiles: Vec<ProfileInfo>,
    pub apps: Vec<AppInfo>,
    pub applied_id: Option<String>,
    pub applied_exe: Option<String>,
    pub armed_id: Option<String>,
    pub has_macros: bool,
    pub macro_names: Vec<String>,
    pub restart_suggested: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApp {
    pub exe: String,
    pub title: String,
    pub path: String,
    pub icon: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Activity {
    phase: &'static str,
    profile_id: String,
    profile_name: String,
    exe: String,
    macro_names: Vec<String>,
    has_macros: bool,
    macros_changed: bool,
    restart: bool,
}

fn activity(phase: &'static str, id: &str, name: &str, exe: &str) -> Activity {
    Activity {
        phase,
        profile_id: id.to_string(),
        profile_name: name.to_string(),
        exe: exe.to_string(),
        macro_names: Vec::new(),
        has_macros: false,
        macros_changed: false,
        restart: false,
    }
}

fn finished(profile: &Profile, exe: &str, macros_changed: bool) -> Activity {
    let restart = macros_changed && profile.keys.iter().any(|entry| entry[0] == protocol::MACRO_KIND);
    Activity {
        phase: "applied",
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        exe: exe.to_string(),
        macro_names: profile.macro_names.clone(),
        has_macros: profile.macros.is_some(),
        macros_changed,
        restart,
    }
}

struct Runtime {
    armed: Option<String>,
    applied_id: Option<String>,
    applied_exe: Option<String>,
    has_macros: bool,
    macro_names: Vec<String>,
    restart_suggested: bool,
    pending_names: Vec<String>,
    stable_exe: String,
    stable_hits: u8,
    fail_key: String,
    fail_at: Option<Instant>,
}

static FILE: Mutex<()> = Mutex::new(());
static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    armed: None,
    applied_id: None,
    applied_exe: None,
    has_macros: false,
    macro_names: Vec::new(),
    restart_suggested: false,
    pending_names: Vec::new(),
    stable_exe: String::new(),
    stable_hits: 0,
    fail_key: String::new(),
    fail_at: None,
});

fn store_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("profiles.json")))
        .unwrap_or_else(|| PathBuf::from("profiles.json"))
}

fn our_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "customchrome.exe".into())
}

fn load() -> Store {
    let path = store_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Store::default();
    };
    match serde_json::from_str(&text) {
        Ok(mut store) => {
            if separate_programs(&mut store) {
                let _ = save(&store);
            }
            store
        }
        Err(err) => {
            diag::error(format!("profiles.json could not be read ({err}); keeping a copy and starting fresh"));
            let _ = fs::rename(&path, path.with_extension("json.bad"));
            Store::default()
        }
    }
}

fn save(store: &Store) -> Result<(), Error> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::Profile(e.to_string()))?;
    }
    let data = serde_json::to_vec_pretty(store).map_err(|e| Error::Profile(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &data).map_err(|e| Error::Profile(e.to_string()))?;
    replace_file(&tmp, &path).map_err(|e| Error::Profile(e.to_string()))
}

fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = fs::remove_file(to);
    fs::rename(from, to)
}

fn with_store<T>(change: impl FnOnce(&mut Store) -> Result<T, Error>) -> Result<T, Error> {
    let _file = FILE.lock().unwrap_or_else(|p| p.into_inner());
    let mut store = load();
    let result = change(&mut store)?;
    save(&store)?;
    Ok(result)
}

fn state_from(store: &Store, runtime: &Runtime) -> ProfileState {
    ProfileState {
        auto: store.auto,
        default_profile: store.default_profile.clone(),
        profiles: store.profiles.iter().map(|p| ProfileInfo { id: p.id.clone(), name: p.name.clone() }).collect(),
        apps: store
            .apps
            .iter()
            .map(|a| AppInfo {
                exe: a.exe.clone(),
                label: a.label.clone(),
                profile_id: a.profile_id.clone(),
                path: a.path.clone(),
                icon: String::new(),
            })
            .collect(),
        applied_id: runtime.applied_id.clone(),
        applied_exe: runtime.applied_exe.clone(),
        armed_id: runtime.armed.clone(),
        has_macros: runtime.has_macros,
        macro_names: runtime.macro_names.clone(),
        restart_suggested: runtime.restart_suggested,
    }
}

pub fn state() -> ProfileState {
    let running = visible_windows();
    let _file = FILE.lock().unwrap_or_else(|p| p.into_inner());
    let mut store = load();
    let mut changed = false;
    for link in &mut store.apps {
        if link.path.is_empty() {
            if let Some(win) = running.iter().find(|win| win.exe.eq_ignore_ascii_case(&link.exe) && !win.path.is_empty()) {
                link.path = win.path.clone();
                changed = true;
            }
        }
    }
    if changed {
        let _ = save(&store);
    }
    let runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    let mut view = state_from(&store, &runtime);
    drop(runtime);
    drop(_file);
    for app in &mut view.apps {
        app.icon = icon_for(&app.path);
    }
    view
}

fn clean_name(name: &str) -> Result<String, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Profile("Give the profile a name.".into()));
    }
    if name.chars().count() > NAME_MAX {
        return Err(Error::Profile(format!("Use a name of {NAME_MAX} characters or fewer.")));
    }
    Ok(name.to_string())
}

fn clean_exe(raw: &str) -> Result<String, Error> {
    let name = raw.trim().rsplit(['\\', '/']).next().unwrap_or("").trim();
    let lower = name.to_ascii_lowercase();
    if name.is_empty()
        || name.len() > 120
        || name.contains("..")
        || name.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        || !lower.ends_with(".exe")
    {
        return Err(Error::Profile("Choose a program, shown as its .exe name.".into()));
    }
    if lower == our_exe().to_ascii_lowercase() {
        return Err(Error::Profile("Choose a program other than CustomChrome.".into()));
    }
    Ok(name.to_string())
}

fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("p{ms}{n}")
}

fn separate_programs(store: &mut Store) -> bool {
    let shared: Vec<String> = store
        .profiles
        .iter()
        .filter(|profile| store.apps.iter().filter(|app| app.profile_id == profile.id).count() > 1)
        .map(|profile| profile.id.clone())
        .collect();
    if shared.is_empty() {
        return false;
    }
    for id in shared {
        let Some(profile) = store.profiles.iter().find(|profile| profile.id == id).cloned() else {
            continue;
        };
        let extra: Vec<usize> = store.apps.iter().enumerate().filter(|(_, app)| app.profile_id == id).map(|(index, _)| index).skip(1).collect();
        for index in extra {
            let mut copy = profile.clone();
            copy.id = new_id();
            let label = store.apps[index].label.clone();
            if !label.is_empty() {
                copy.name = label;
            }
            store.apps[index].profile_id = copy.id.clone();
            store.profiles.push(copy);
        }
    }
    true
}

fn is_manual(store: &Store, id: &str) -> bool {
    !store.apps.iter().any(|app| app.profile_id == id)
}

fn forget_applied() {
    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    runtime.applied_id = None;
    runtime.applied_exe = None;
    runtime.restart_suggested = false;
}

fn leaves_held(events: &[crate::aula::protocol::MacroEvent]) -> bool {
    let mut held = std::collections::HashSet::new();
    for event in events {
        let id = (event.mouse, event.code);
        if event.down {
            held.insert(id);
        } else {
            held.remove(&id);
        }
    }
    !held.is_empty()
}

fn fit_names(names: Vec<String>, count: usize) -> Vec<String> {
    let mut names: Vec<String> = names.into_iter().map(|name| name.trim().chars().take(NAME_MAX).collect()).collect();
    names.truncate(count);
    names.resize(count, String::new());
    names
}

pub fn save_current(name: &str, lighting: Lighting, keys: Vec<KeyEntry>, macros: Vec<Macro>, macro_names: Vec<String>) -> Result<ProfileState, Error> {
    let name = clean_name(name)?;
    for (i, entry) in keys.iter().enumerate() {
        let entry = protocol::canonical_entry(*entry);
        if protocol::is_remappable(i as u8) && !protocol::is_valid_entry(&entry) {
            return Err(Error::Profile(
                "A key on the keyboard is damaged, so this profile wasn't saved. Use Fix it first.".into(),
            ));
        }
        if entry[0] == protocol::MACRO_KIND && entry[1] as usize >= macros.len() {
            return Err(Error::Profile("A key plays a macro that isn't on the keyboard, so this profile wasn't saved.".into()));
        }
    }
    if protocol::encode_macros(&macros).is_none() {
        return Err(Error::Profile("A macro is empty, too big, or has a step the keyboard can't play, so this profile wasn't saved.".into()));
    }
    if let Some(index) = macros.iter().position(|events| leaves_held(events)) {
        return Err(Error::Profile(format!("Macro {} leaves a key held down. Release it in Macros before saving this profile.", index + 1)));
    }
    let macro_names = fit_names(macro_names, macros.len());
    with_store(|store| {
        let manual = store.profiles.iter().find(|profile| profile.name.eq_ignore_ascii_case(&name) && is_manual(store, &profile.id)).map(|profile| profile.id.clone());
        if manual.is_none() && store.profiles.iter().any(|profile| profile.name.eq_ignore_ascii_case(&name)) {
            return Err(Error::Profile("A program already uses that name. Pick another name for a manual profile.".into()));
        }
        if let Some(id) = manual {
            let existing = store.profiles.iter_mut().find(|profile| profile.id == id).unwrap();
            existing.lighting = lighting;
            existing.keys = keys;
            existing.macros = Some(macros);
            existing.macro_names = macro_names;
        } else {
            if store.profiles.len() >= MAX_PROFILES {
                return Err(Error::Profile(format!("You can save up to {MAX_PROFILES} profiles.")));
            }
            store.profiles.push(Profile {
                id: new_id(),
                name,
                lighting,
                keys,
                macros: Some(macros),
                macro_names,
            });
        }
        Ok(())
    })?;
    forget_applied();
    diag::info("profile saved");
    Ok(state())
}

pub fn rename(id: &str, name: &str) -> Result<ProfileState, Error> {
    let name = clean_name(name)?;
    with_store(|store| {
        if store.profiles.iter().any(|p| p.id != id && p.name.eq_ignore_ascii_case(&name)) {
            return Err(Error::Profile("Another profile already has that name.".into()));
        }
        let profile = store.profiles.iter_mut().find(|p| p.id == id).ok_or_else(|| Error::Profile("That profile is gone.".into()))?;
        profile.name = name;
        Ok(())
    })?;
    Ok(state())
}

pub fn delete(id: &str) -> Result<ProfileState, Error> {
    with_store(|store| {
        let before = store.profiles.len();
        store.profiles.retain(|p| p.id != id);
        if store.profiles.len() == before {
            return Err(Error::Profile("That profile is gone.".into()));
        }
        store.apps.retain(|a| a.profile_id != id);
        if store.default_profile.as_deref() == Some(id) {
            store.default_profile = None;
        }
        Ok(())
    })?;
    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    if runtime.applied_id.as_deref() == Some(id) {
        runtime.applied_id = None;
        runtime.applied_exe = None;
    }
    if runtime.armed.as_deref() == Some(id) {
        runtime.armed = None;
    }
    drop(runtime);
    Ok(state())
}

pub fn set_auto(on: bool) -> Result<ProfileState, Error> {
    with_store(|store| {
        store.auto = on;
        Ok(())
    })?;
    if on {
        forget_applied();
    }
    diag::info(format!("profile auto {}", if on { "on" } else { "off" }));
    Ok(state())
}

pub fn set_default(id: Option<String>) -> Result<ProfileState, Error> {
    with_store(|store| {
        if let Some(id) = &id {
            if !store.profiles.iter().any(|profile| &profile.id == id) {
                return Err(Error::Profile("That profile is gone.".into()));
            }
            if !is_manual(store, id) {
                return Err(Error::Profile("Pick a manual profile for other programs, or leave the keyboard as it is.".into()));
            }
        }
        store.default_profile = id;
        Ok(())
    })?;
    forget_applied();
    Ok(state())
}

pub fn assign_app(exe: &str, label: &str, path: &str, profile_id: &str) -> Result<ProfileState, Error> {
    let exe = clean_exe(exe)?;
    let path = accepted_path(path, &exe);
    let label = {
        let label = label.trim().chars().take(80).collect::<String>();
        if label.is_empty() { exe.clone() } else { label }
    };
    with_store(|store| {
        if !store.profiles.iter().any(|p| p.id == profile_id) {
            return Err(Error::Profile("That profile is gone.".into()));
        }
        if let Some(link) = store.apps.iter_mut().find(|a| a.exe.eq_ignore_ascii_case(&exe)) {
            link.exe = exe;
            link.label = label;
            link.profile_id = profile_id.to_string();
            if !path.is_empty() {
                link.path = path;
            }
        } else {
            if store.apps.len() >= MAX_APPS {
                return Err(Error::Profile(format!("You can assign up to {MAX_APPS} programs.")));
            }
            store.apps.push(AppLink { exe, label, path, profile_id: profile_id.to_string() });
        }
        Ok(())
    })?;
    forget_applied();
    diag::info(format!("profile assigned to {profile_id}"));
    Ok(state())
}

fn accepted_path(path: &str, exe: &str) -> String {
    let path = path.trim();
    if path.is_empty() || path.len() > 520 || path.contains("..") || path.chars().any(|c| c.is_control()) {
        return String::new();
    }
    let file = path.rsplit(['\\', '/']).next().unwrap_or("");
    if file.eq_ignore_ascii_case(exe) { path.to_string() } else { String::new() }
}

pub fn unassign_app(exe: &str) -> Result<ProfileState, Error> {
    let exe = clean_exe(exe)?;
    with_store(|store| {
        let profile_id = store.apps.iter().find(|app| app.exe.eq_ignore_ascii_case(&exe)).map(|app| app.profile_id.clone());
        store.apps.retain(|app| !app.exe.eq_ignore_ascii_case(&exe));
        if let Some(profile_id) = profile_id {
            if is_manual(store, &profile_id) {
                store.profiles.retain(|profile| profile.id != profile_id);
                if store.default_profile.as_deref() == Some(profile_id.as_str()) {
                    store.default_profile = None;
                }
            }
        }
        Ok(())
    })?;
    forget_applied();
    Ok(state())
}

/// Saves what the keyboard is doing now as this program's own setup. A program never shares a profile with another program.
pub fn adopt_program(
    exe: &str,
    label: &str,
    path: &str,
    lighting: Lighting,
    keys: Vec<KeyEntry>,
    macros: Vec<Macro>,
    macro_names: Vec<String>,
) -> Result<ProfileState, Error> {
    let exe = clean_exe(exe)?;
    let path = accepted_path(path, &exe);
    let label = {
        let label = label.trim().chars().take(NAME_MAX).collect::<String>();
        if label.is_empty() { exe.clone() } else { label }
    };
    for (i, entry) in keys.iter().enumerate() {
        let entry = protocol::canonical_entry(*entry);
        if protocol::is_remappable(i as u8) && !protocol::is_valid_entry(&entry) {
            return Err(Error::Profile("A key on the keyboard is damaged, so this program wasn't saved. Use Fix it first.".into()));
        }
    }
    if protocol::encode_macros(&macros).is_none() {
        return Err(Error::Profile("A macro is empty, too big, or has a step the keyboard can't play.".into()));
    }
    let macro_names = fit_names(macro_names, macros.len());
    let saved = exe.clone();
    with_store(|store| {
        if let Some(profile_id) = store.apps.iter().find(|app| app.exe.eq_ignore_ascii_case(&exe)).map(|app| app.profile_id.clone()) {
            if let Some(link) = store.apps.iter_mut().find(|app| app.exe.eq_ignore_ascii_case(&exe)) {
                link.label = label.clone();
                if !path.is_empty() {
                    link.path = path.clone();
                }
            }
            let profile = store.profiles.iter_mut().find(|profile| profile.id == profile_id).ok_or_else(|| Error::Profile("That program's profile is gone.".into()))?;
            profile.name = label;
            profile.lighting = lighting;
            profile.keys = keys;
            profile.macros = Some(macros);
            profile.macro_names = macro_names;
        } else {
            if store.profiles.len() >= MAX_PROFILES || store.apps.len() >= MAX_APPS {
                return Err(Error::Profile(format!("You can save up to {MAX_APPS} programs.")));
            }
            let id = new_id();
            store.profiles.push(Profile {
                id: id.clone(),
                name: label.clone(),
                lighting,
                keys,
                macros: Some(macros),
                macro_names,
            });
            store.apps.push(AppLink { exe, label, path, profile_id: id });
        }
        Ok(())
    })?;
    forget_applied();
    diag::info(format!("program profile saved for {saved}"));
    Ok(state())
}

pub fn arm_new(names: Vec<String>) -> Result<ProfileState, Error> {
    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    runtime.armed = Some("*new".into());
    runtime.pending_names = names;
    runtime.stable_exe.clear();
    runtime.stable_hits = 0;
    drop(runtime);
    Ok(state())
}

pub fn disarm() -> ProfileState {
    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    runtime.armed = None;
    drop(runtime);
    state()
}

fn profile_by_id(store: &Store, id: &str) -> Result<Profile, Error> {
    store.profiles.iter().find(|p| p.id == id).cloned().ok_or_else(|| Error::Profile("That profile is gone.".into()))
}

pub struct ProfileBundle {
    pub name: String,
    pub lighting: Lighting,
    pub keys: Vec<KeyEntry>,
    pub macros: Option<Vec<Macro>>,
    pub macro_names: Vec<String>,
}

pub fn profile_for_apply(id: &str) -> Result<ProfileBundle, Error> {
    let _file = FILE.lock().unwrap_or_else(|p| p.into_inner());
    let store = load();
    let profile = profile_by_id(&store, id)?;
    Ok(ProfileBundle {
        name: profile.name,
        lighting: profile.lighting,
        keys: profile.keys,
        macros: profile.macros,
        macro_names: profile.macro_names,
    })
}

pub fn note_applied(id: &str, exe: Option<String>, has_macros: bool, names: &[String], restart: bool) {
    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    runtime.applied_id = Some(id.to_string());
    runtime.applied_exe = exe;
    runtime.has_macros = has_macros;
    runtime.macro_names = names.to_vec();
    runtime.restart_suggested = restart;
    runtime.fail_at = None;
}

/// Which saved profile should be on the keyboard for this program, if automatic switching is on.
fn target<'a>(store: &'a Store, exe: &str) -> Option<&'a Profile> {
    if let Some(link) = store.apps.iter().find(|a| a.exe.eq_ignore_ascii_case(exe)) {
        if let Some(profile) = store.profiles.iter().find(|p| p.id == link.profile_id) {
            return Some(profile);
        }
    }
    store.default_profile.as_ref().and_then(|id| store.profiles.iter().find(|p| &p.id == id))
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(700));
        if let Err(err) = watch(&app) {
            diag::warn(format!("profile watch: {err}"));
        }
    });
}

fn watch(app: &AppHandle) -> Result<(), Error> {
    let Some(front) = foreground() else {
        return Ok(());
    };
    if front.exe.eq_ignore_ascii_case(&our_exe()) {
        let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
        runtime.stable_exe.clear();
        runtime.stable_hits = 0;
        return Ok(());
    }
    if SHELL.iter().any(|name| front.exe.eq_ignore_ascii_case(name)) {
        return Ok(());
    }

    let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    if front.exe.eq_ignore_ascii_case(&runtime.stable_exe) {
        runtime.stable_hits = runtime.stable_hits.saturating_add(1);
    } else {
        runtime.stable_exe = front.exe.clone();
        runtime.stable_hits = 1;
    }
    if runtime.stable_hits < 2 {
        return Ok(());
    }
    let armed = runtime.armed.clone();
    drop(runtime);

    if let Some(_) = armed {
        let lock = app.state::<crate::DeviceLock>();
        let Ok(guard) = lock.0.try_lock() else {
            return Ok(());
        };
        let snap = match device::read_snapshot() {
            Ok(snap) => snap,
            Err(err) => {
                drop(guard);
                let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
                runtime.armed = None;
                runtime.pending_names.clear();
                runtime.fail_at = Some(Instant::now());
                diag::warn(format!("program capture stopped, keyboard did not answer: {err}"));
                let _ = app.emit("profile-activity", activity("failed", "", &front.exe, &front.exe));
                return Ok(());
            }
        };
        drop(guard);
        let names = {
            let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
            runtime.armed = None;
            std::mem::take(&mut runtime.pending_names)
        };
        let label = if front.title.is_empty() { front.exe.clone() } else { front.title.clone() };
        if let Err(err) = adopt_program(&front.exe, &label, &front.path, snap.lighting, snap.keys, snap.macros, names) {
            diag::warn(format!("program capture could not be saved: {err}"));
            return Ok(());
        }
        let _ = app.emit("profile-activity", activity("captured", "", &label, &front.exe));
        diag::info(format!("program capture saved {}", front.exe));
    }

    let store = {
        let _file = FILE.lock().unwrap_or_else(|p| p.into_inner());
        load()
    };
    if !store.auto {
        return Ok(());
    }
    let Some(profile) = target(&store, &front.exe) else {
        return Ok(());
    };
    let runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
    if runtime.applied_id.as_deref() == Some(profile.id.as_str()) && runtime.applied_exe.as_deref() == Some(front.exe.as_str()) {
        return Ok(());
    }
    let fail_key = format!("{}:{}", profile.id, front.exe.to_ascii_lowercase());
    if runtime.fail_key == fail_key && runtime.fail_at.is_some_and(|at| at.elapsed() < Duration::from_secs(60)) {
        return Ok(());
    }
    drop(runtime);

    let lock = app.state::<crate::DeviceLock>();
    let Ok(_guard) = lock.0.try_lock() else {
        return Ok(());
    };
    let _ = app.emit("profile-activity", activity("applying", &profile.id, &profile.name, &front.exe));
    match device::apply_snapshot(&profile.lighting, &profile.keys, profile.macros.as_deref()) {
        Ok(applied) => {
            diag::info(format!(
                "profile '{}' applied for {} (keys rewritten: {}, macros rewritten: {})",
                profile.name, front.exe, applied.keys_changed, applied.macros_changed
            ));
            let restart = applied.macros_changed && profile.keys.iter().any(|entry| entry[0] == protocol::MACRO_KIND);
            note_applied(&profile.id, Some(front.exe.clone()), profile.macros.is_some(), &profile.macro_names, restart);
            let _ = app.emit("profile-activity", finished(profile, &front.exe, applied.macros_changed));
        }
        Err(err) => {
            diag::warn(format!("profile '{}' for {} failed: {err}", profile.name, front.exe));
            let mut runtime = RUNTIME.lock().unwrap_or_else(|p| p.into_inner());
            runtime.fail_key = fail_key;
            runtime.fail_at = Some(Instant::now());
            let _ = app.emit("profile-activity", activity("failed", &profile.id, &profile.name, &front.exe));
        }
    }
    Ok(())
}

pub fn open_apps() -> Vec<OpenApp> {
    let ours = our_exe();
    let mut apps = visible_windows();
    apps.retain(|app| !app.exe.eq_ignore_ascii_case(&ours) && !SHELL.iter().any(|name| app.exe.eq_ignore_ascii_case(name)));
    let mut seen = Vec::new();
    apps.retain(|app| {
        let key = app.exe.to_ascii_lowercase();
        if seen.iter().any(|s: &String| s == &key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    apps.truncate(MAX_APPS);
    apps.into_iter()
        .map(|app| OpenApp { icon: icon_for(&app.path), exe: app.exe, title: app.title, path: app.path })
        .collect()
}

fn visible_windows() -> Vec<WindowApp> {
    #[cfg(windows)]
    {
        let mut found = Vec::new();
        unsafe {
            EnumWindows(collect_window, &mut found as *mut Vec<WindowApp> as isize);
        }
        found
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

struct Front {
    exe: String,
    title: String,
    path: String,
}

struct WindowApp {
    exe: String,
    title: String,
    path: String,
}

#[cfg(windows)]
fn foreground() -> Option<Front> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return None;
    }
    let image = image_of(hwnd)?;
    Some(Front { exe: image.0, title: window_title(hwnd), path: image.1 })
}

#[cfg(not(windows))]
fn foreground() -> Option<Front> {
    None
}

#[cfg(windows)]
extern "system" fn collect_window(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
    let found = unsafe { &mut *(lparam as *mut Vec<WindowApp>) };
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let title = window_title(hwnd);
    if title.is_empty() {
        return 1;
    }
    if let Some((exe, path)) = image_of(hwnd) {
        found.push(WindowApp { exe, title, path });
    }
    1
}

#[cfg(windows)]
fn window_title(hwnd: *mut std::ffi::c_void) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize]).chars().filter(|c| !c.is_control()).take(80).collect()
}

#[cfg(windows)]
fn image_of(hwnd: *mut std::ffi::c_void) -> Option<(String, String)> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(0x1000, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) };
    unsafe { CloseHandle(handle) };
    if ok == 0 || len == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let exe = path.rsplit(['\\', '/']).next()?.to_string();
    Some((exe, path))
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetWindowThreadProcessId(hwnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
    fn IsWindowVisible(hwnd: *mut std::ffi::c_void) -> i32;
    fn GetWindowTextW(hwnd: *mut std::ffi::c_void, buf: *mut u16, max: i32) -> i32;
    fn DestroyIcon(icon: *mut std::ffi::c_void) -> i32;
    fn DrawIconEx(dc: *mut std::ffi::c_void, x: i32, y: i32, icon: *mut std::ffi::c_void, cx: i32, cy: i32, step: u32, brush: *mut std::ffi::c_void, flags: u32) -> i32;
    fn GetDC(window: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn ReleaseDC(window: *mut std::ffi::c_void, dc: *mut std::ffi::c_void) -> i32;
    fn PrivateExtractIconsW(file: *const u16, index: i32, cx: i32, cy: i32, icons: *mut *mut std::ffi::c_void, ids: *mut u32, count: u32, flags: u32) -> u32;
    fn EnumWindows(cb: extern "system" fn(*mut std::ffi::c_void, isize) -> i32, lparam: isize) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    fn QueryFullProcessImageNameW(process: *mut std::ffi::c_void, flags: u32, name: *mut u16, size: *mut u32) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(dc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CreateDIBSection(dc: *mut std::ffi::c_void, info: *const BitmapInfoHeader, usage: u32, bits: *mut *mut std::ffi::c_void, section: *mut std::ffi::c_void, offset: u32) -> *mut std::ffi::c_void;
    fn SelectObject(dc: *mut std::ffi::c_void, object: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn DeleteObject(object: *mut std::ffi::c_void) -> i32;
    fn DeleteDC(dc: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_ppm: i32,
    y_ppm: i32,
    clr_used: u32,
    clr_important: u32,
}

fn icon_cache() -> &'static Mutex<HashMap<String, String>> {
    static ICONS: std::sync::OnceLock<Mutex<HashMap<String, String>>> = std::sync::OnceLock::new();
    ICONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn icon_for(path: &str) -> String {
    if path.is_empty() || !std::path::Path::new(path).is_file() {
        return String::new();
    }
    let mut cache = icon_cache().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(icon) = cache.get(path) {
        return icon.clone();
    }
    let icon = extract_icon(path).unwrap_or_default();
    cache.insert(path.to_string(), icon.clone());
    icon
}

fn extract_icon(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
        let mut icon = std::ptr::null_mut();
        let count = unsafe { PrivateExtractIconsW(wide.as_ptr(), 0, 48, 48, &mut icon, std::ptr::null_mut(), 1, 0) };
        if count == 0 || icon.is_null() {
            return None;
        }
        let rgba = unsafe { render_icon(icon, 48) };
        unsafe { DestroyIcon(icon) };
        let rgba = rgba?;
        Some(format!("data:image/png;base64,{}", base64(&png_rgba(48, 48, &rgba))))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(windows)]
unsafe fn render_icon(icon: *mut std::ffi::c_void, size: i32) -> Option<Vec<u8>> {
    let header = BitmapInfoHeader {
        size: 40,
        width: size,
        height: -size,
        planes: 1,
        bit_count: 32,
        compression: 0,
        size_image: 0,
        x_ppm: 0,
        y_ppm: 0,
        clr_used: 0,
        clr_important: 0,
    };
    let screen = GetDC(std::ptr::null_mut());
    let mut bits = std::ptr::null_mut();
    let dib = CreateDIBSection(screen, &header, 0, &mut bits, std::ptr::null_mut(), 0);
    if dib.is_null() || bits.is_null() {
        ReleaseDC(std::ptr::null_mut(), screen);
        return None;
    }
    let mem = CreateCompatibleDC(screen);
    let previous = SelectObject(mem, dib);
    std::ptr::write_bytes(bits, 0, (size * size * 4) as usize);
    DrawIconEx(mem, 0, 0, icon, size, size, 0, std::ptr::null_mut(), 3);
    let mut raw = vec![0u8; (size * size * 4) as usize];
    std::ptr::copy_nonoverlapping(bits as *const u8, raw.as_mut_ptr(), raw.len());
    SelectObject(mem, previous);
    DeleteDC(mem);
    DeleteObject(dib);
    ReleaseDC(std::ptr::null_mut(), screen);
    for pixel in raw.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        let alpha = pixel[3];
        if alpha > 0 && alpha < 255 {
            pixel[0] = ((pixel[0] as u16 * 255) / alpha as u16).min(255) as u8;
            pixel[1] = ((pixel[1] as u16 * 255) / alpha as u16).min(255) as u8;
            pixel[2] = ((pixel[2] as u16 * 255) / alpha as u16).min(255) as u8;
        }
    }
    Some(raw)
}

fn png_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(((width * 4 + 1) * height) as usize);
    let stride = (width * 4) as usize;
    for row in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgba[row * stride..(row + 1) * stride]);
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &zlib_store(&raw));
    png_chunk(&mut png, b"IEND", &[]);
    png
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for byte in kind.iter().chain(data) {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 { 0xEDB8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut rest = data;
    while !rest.is_empty() {
        let take = rest.len().min(65535);
        let last = take == rest.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&rest[..take]);
        rest = &rest[take..];
    }
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for byte in data {
        s1 = (s1 + *byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    out.extend_from_slice(&((s2 << 16) | s1).to_be_bytes());
    out
}

fn base64(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push(TABLE[(n & 63) as usize] as char);
        i += 3;
    }
    if i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str) -> Profile {
        Profile {
            id: id.into(),
            name: id.into(),
            lighting: Lighting { effect: 1, color: [1, 2, 3], rainbow: false, brightness: 4, speed: 3, direction: 0 },
            keys: Vec::new(),
            macros: None,
            macro_names: Vec::new(),
        }
    }

    #[test]
    fn linked_program_wins_over_the_fallback() {
        let store = Store {
            auto: true,
            default_profile: Some("everyday".into()),
            profiles: vec![profile("everyday"), profile("game")],
            apps: vec![AppLink { exe: "VALORANT.exe".into(), label: "VALORANT".into(), path: String::new(), profile_id: "game".into() }],
        };
        assert_eq!(target(&store, "valorant.exe").map(|p| p.id.as_str()), Some("game"));
        assert_eq!(target(&store, "notepad.exe").map(|p| p.id.as_str()), Some("everyday"));
    }

    #[test]
    fn no_fallback_leaves_other_programs_alone() {
        let store = Store {
            auto: true,
            default_profile: None,
            profiles: vec![profile("game")],
            apps: vec![AppLink { exe: "game.exe".into(), label: "Game".into(), path: String::new(), profile_id: "game".into() }],
        };
        assert!(target(&store, "chrome.exe").is_none());
    }

    #[test]
    fn notepad_icon_is_a_png() {
        let path = r"C:\Windows\System32\notepad.exe";
        if !std::path::Path::new(path).is_file() {
            return;
        }
        let icon = icon_for(path);
        assert!(icon.starts_with("data:image/png;base64,"), "{icon}");
        assert!(icon.len() > 200);
    }

    #[test]
    fn a_png_of_pixels_starts_with_the_png_signature() {
        let png = png_rgba(1, 1, &[255, 0, 0, 255]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn a_broken_link_uses_the_fallback() {
        let store = Store {
            auto: true,
            default_profile: Some("everyday".into()),
            profiles: vec![profile("everyday")],
            apps: vec![AppLink { exe: "old.exe".into(), label: "Old".into(), path: String::new(), profile_id: "missing".into() }],
        };
        assert_eq!(target(&store, "old.exe").map(|p| p.id.as_str()), Some("everyday"));
    }
}
