//! Tauri shell: thin command layer over `ac-core`. All logic lives in the core crate.

use ac_core::assists::{self, AssistPreset, Assists};
use ac_core::content::{self, Car, Skin, Track, Weather};
use ac_core::audio::{self, AudioConfig};
use ac_core::controls::{self, ControlsConfig};
use ac_core::input::{self, InputDevice, Monitor};
use ac_core::launch;
use ac_core::mods::{self, ModPlan};
use ac_core::replays::{self, Replay};
use ac_core::setups::{self, SetupFile};
use ac_core::paths::{self, Resolved, Settings};
use ac_core::video::{self, DisplayMode, VideoConfig};
use ac_core::race::{self, RaceRequest};
use serde::Serialize;
use std::fs;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// Shared, mutable launcher settings.
struct AppState {
    settings: Mutex<Settings>,
    /// Running input monitor (live axis/button view on the Controls page).
    monitor: Mutex<Option<Monitor>>,
    /// Mod archives that were inspected and await confirmation (plan id -> plan).
    plans: Mutex<std::collections::HashMap<u64, ModPlan>>,
    next_plan: std::sync::atomic::AtomicU64,
}

#[derive(Serialize)]
struct StateDto {
    settings: Settings,
    resolved: Resolved,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchInfo {
    command: String,
    log_path: Option<String>,
}

/// Let the webview load previews from the game's content folder (and only that).
fn allow_content(app: &AppHandle, resolved: &Resolved) {
    if let Some(dir) = resolved.content_dir() {
        let _ = app.asset_protocol_scope().allow_directory(dir, true);
    }
}

fn snapshot(app: &AppHandle, state: &State<AppState>) -> StateDto {
    let settings = state.settings.lock().unwrap().clone();
    let resolved = paths::resolve(&settings);
    allow_content(app, &resolved);
    StateDto { settings, resolved }
}

fn content_dir(state: &State<AppState>) -> Result<std::path::PathBuf, String> {
    let settings = state.settings.lock().unwrap().clone();
    paths::resolve(&settings)
        .content_dir()
        .filter(|d| d.is_dir())
        .ok_or_else(|| "Game content folder not found. Check Settings.".to_string())
}

#[tauri::command]
fn get_state(app: AppHandle, state: State<AppState>) -> StateDto {
    snapshot(&app, &state)
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<StateDto, String> {
    settings.save().map_err(|e| format!("Could not save settings: {e}"))?;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_fullscreen(!settings.windowed_launcher);
    }
    *state.settings.lock().unwrap() = settings;
    Ok(snapshot(&app, &state))
}

/// F11: toggle the launcher's own fullscreen.
#[tauri::command]
fn toggle_fullscreen(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let now = w.is_fullscreen().unwrap_or(false);
        let _ = w.set_fullscreen(!now);
    }
}

#[tauri::command]
fn list_cars(state: State<AppState>) -> Result<Vec<Car>, String> {
    Ok(content::list_cars(&content_dir(&state)?))
}

#[tauri::command]
fn list_skins(state: State<AppState>, car: String) -> Result<Vec<Skin>, String> {
    Ok(content::list_skins(&content_dir(&state)?, &car))
}

#[tauri::command]
fn list_tracks(state: State<AppState>) -> Result<Vec<Track>, String> {
    Ok(content::list_tracks(&content_dir(&state)?))
}

#[tauri::command]
fn list_weather(state: State<AppState>) -> Result<Vec<Weather>, String> {
    Ok(content::list_weather(&content_dir(&state)?))
}

#[tauri::command]
fn assist_presets() -> Vec<AssistPreset> {
    assists::presets()
}

/// Write race.ini + assists.ini and start the game.
#[tauri::command]
fn launch_race(app: AppHandle, state: State<AppState>, request: RaceRequest, assists: Assists) -> Result<LaunchInfo, String> {
    let settings = state.settings.lock().unwrap().clone();
    let resolved = paths::resolve(&settings);
    if resolved.has_errors() {
        return Err(resolved
            .issues
            .iter()
            .filter(|i| i.level == paths::Level::Error)
            .map(|i| i.message.clone())
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let cfg = resolved.cfg_dir().ok_or("No user config folder")?;
    let game = resolved.game_dir.clone().ok_or("No game folder")?;
    race::write(&cfg, &game, &request).map_err(|e| format!("Could not write race.ini: {e}"))?;
    assists::write(&cfg, &game, &assists).map_err(|e| format!("Could not write assists.ini: {e}"))?;

    start_game(app, &settings, &resolved)
}

/// Spawn the game, hide the launcher while it runs, and report when it exits.
fn start_game(app: AppHandle, settings: &Settings, resolved: &Resolved) -> Result<LaunchInfo, String> {
    let cmd = launch::build_command(settings, resolved).map_err(|e| e.to_string())?;
    let command = launch::describe(&cmd);
    let mut child = launch::spawn(settings, resolved).map_err(|e| format!("Could not start Proton: {e}"))?;

    // Get out of the way while the game runs (restored when it exits). We hide the
    // window rather than minimize it: tiling compositors such as Hyprland have no minimize.
    let hidden = !settings.stay_open_on_launch;
    if hidden {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    // Tell the UI when the game exits (successfully or not) and bring the launcher back.
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|s| s.code());
        if hidden {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
        let _ = app.emit("game-exited", code);
    });
    Ok(LaunchInfo { command, log_path: launch::launch_log_path().map(|p| p.to_string_lossy().into_owned()) })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DevicesDto {
    devices: Vec<InputDevice>,
    /// Event nodes we could not open (permissions); explains an empty list.
    unreadable: usize,
}

#[tauri::command]
fn list_input_devices() -> DevicesDto {
    let (devices, unreadable) = input::list_devices();
    DevicesDto { devices, unreadable }
}

/// Start streaming `input-state` events for every detected device.
#[tauri::command]
fn start_input_monitor(app: AppHandle, state: State<AppState>) {
    let (devices, _) = input::list_devices();
    let monitor = input::monitor(&devices, move |snap| {
        let _ = app.emit("input-state", snap);
    });
    *state.monitor.lock().unwrap() = Some(monitor); // dropping the old one stops its threads
}

#[tauri::command]
fn stop_input_monitor(state: State<AppState>) {
    *state.monitor.lock().unwrap() = None;
}

fn cfg_and_game(state: &State<AppState>) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let settings = state.settings.lock().unwrap().clone();
    let r = paths::resolve(&settings);
    Ok((r.cfg_dir().ok_or("No user config folder")?, r.game_dir.ok_or("No game folder")?))
}

#[tauri::command]
fn read_controls(state: State<AppState>) -> Result<ControlsConfig, String> {
    let (cfg, game) = cfg_and_game(&state)?;
    Ok(controls::read_config(&controls::load_text(&cfg, &game)))
}

#[tauri::command]
fn save_controls(state: State<AppState>, config: ControlsConfig) -> Result<(), String> {
    let (cfg, game) = cfg_and_game(&state)?;
    controls::save_config(&cfg, &game, &config).map_err(|e| format!("Could not write controls.ini: {e}"))
}

/// The measured Logitech G923 bindings as an editable (unsaved) config.
#[tauri::command]
fn g923_preset(state: State<AppState>) -> Result<ControlsConfig, String> {
    let (cfg, game) = cfg_and_game(&state)?;
    Ok(controls::g923_config(&controls::load_text(&cfg, &game)))
}

#[derive(Serialize)]
struct VideoDto {
    config: VideoConfig,
    modes: Vec<DisplayMode>,
    filters: Vec<String>,
}

#[tauri::command]
fn read_video(state: State<AppState>) -> Result<VideoDto, String> {
    let (cfg, game) = cfg_and_game(&state)?;
    Ok(VideoDto {
        config: video::read_config(&video::load_text(&cfg, &game)),
        modes: video::detect_modes(),
        filters: video::list_filters(&game),
    })
}

#[tauri::command]
fn save_video(state: State<AppState>, config: VideoConfig) -> Result<(), String> {
    let (cfg, game) = cfg_and_game(&state)?;
    video::save_config(&cfg, &game, &config).map_err(|e| format!("Could not write video.ini: {e}"))
}

/// Apply a quality preset ("low" | "medium" | "high" | "ultra") to a config (not saved).
#[tauri::command]
fn video_preset(mut config: VideoConfig, preset: String) -> Result<VideoConfig, String> {
    if video::apply_quality_preset(&mut config, &preset) { Ok(config) } else { Err(format!("unknown preset {preset}")) }
}

#[tauri::command]
fn read_audio(state: State<AppState>) -> Result<AudioConfig, String> {
    let (cfg, game) = cfg_and_game(&state)?;
    Ok(audio::read_config(&audio::load_text(&cfg, &game)))
}

#[tauri::command]
fn save_audio(state: State<AppState>, config: AudioConfig) -> Result<(), String> {
    let (cfg, game) = cfg_and_game(&state)?;
    audio::save_config(&cfg, &game, &config).map_err(|e| format!("Could not write audio.ini: {e}"))
}

/// Error text for blocking configuration problems, if any.
fn blocking_issues(resolved: &Resolved) -> Option<String> {
    resolved.has_errors().then(|| {
        resolved.issues.iter().filter(|i| i.level == paths::Level::Error).map(|i| i.message.clone()).collect::<Vec<_>>().join("\n")
    })
}

fn docs_and_game(state: &State<AppState>) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let settings = state.settings.lock().unwrap().clone();
    let r = paths::resolve(&settings);
    Ok((r.user_docs.ok_or("Proton prefix not found")?, r.game_dir.ok_or("No game folder")?))
}

// ---- replays

#[tauri::command]
fn list_replays(state: State<AppState>) -> Result<Vec<Replay>, String> {
    let (docs, game) = docs_and_game(&state)?;
    let content = game.join("content");
    Ok(replays::list(&docs, &content::car_ids(&content), &content::track_ids(&content)))
}

/// Play a replay: write race.ini's [REPLAY] section and start the game.
#[tauri::command]
fn play_replay(app: AppHandle, state: State<AppState>, name: String, temp: bool) -> Result<LaunchInfo, String> {
    let settings = state.settings.lock().unwrap().clone();
    let resolved = paths::resolve(&settings);
    if let Some(e) = blocking_issues(&resolved) {
        return Err(e);
    }
    let docs = resolved.user_docs.clone().ok_or("Proton prefix not found")?;
    // Temp recordings are copied next to saved ones so the game finds them as regular replays.
    let file = if temp { replays::keep(&docs, &name).map_err(|e| e.to_string())? } else { name };
    race::write_replay(&docs.join("cfg"), &file).map_err(|e| format!("Could not write race.ini: {e}"))?;
    start_game(app, &settings, &resolved)
}

#[tauri::command]
fn keep_replay(state: State<AppState>, name: String) -> Result<String, String> {
    let (docs, _) = docs_and_game(&state)?;
    replays::keep(&docs, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_replay(state: State<AppState>, name: String, temp: bool) -> Result<(), String> {
    let (docs, _) = docs_and_game(&state)?;
    replays::delete(&docs, &name, temp).map_err(|e| e.to_string())
}

// ---- setups

#[tauri::command]
fn list_setups(state: State<AppState>, car: Option<String>) -> Result<Vec<SetupFile>, String> {
    let (docs, game) = docs_and_game(&state)?;
    Ok(setups::list(&docs, &game, car.as_deref()))
}

#[tauri::command]
fn import_setup(state: State<AppState>, file: String, car: Option<String>, scope: String) -> Result<SetupFile, String> {
    let (docs, _) = docs_and_game(&state)?;
    setups::import(&docs, std::path::Path::new(&file), car.as_deref(), &scope).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_setup(state: State<AppState>, path: String, dest: String) -> Result<(), String> {
    let (docs, game) = docs_and_game(&state)?;
    setups::export(&docs, &game, std::path::Path::new(&path), std::path::Path::new(&dest)).map_err(|e| e.to_string())
}

#[tauri::command]
fn adopt_setup(state: State<AppState>, path: String) -> Result<SetupFile, String> {
    let (docs, game) = docs_and_game(&state)?;
    setups::adopt_builtin(&docs, &game, std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_setup(state: State<AppState>, path: String) -> Result<(), String> {
    let (docs, _) = docs_and_game(&state)?;
    setups::delete(&docs, std::path::Path::new(&path)).map_err(|e| e.to_string())
}

// ---- mod / skin install

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanDto {
    id: u64,
    #[serde(flatten)]
    plan: ModPlan,
}

#[tauri::command]
fn mod_inspect(state: State<AppState>, archive: String) -> Result<PlanDto, String> {
    let content = content_dir(&state)?;
    let plan = mods::inspect(std::path::Path::new(&archive), &content).map_err(|e| e.to_string())?;
    let id = state.next_plan.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    state.plans.lock().unwrap().insert(id, plan.clone());
    Ok(PlanDto { id, plan })
}

#[tauri::command]
fn mod_install(state: State<AppState>, id: u64, overwrite: bool, skin_car: Option<String>) -> Result<Vec<String>, String> {
    let content = content_dir(&state)?;
    let plan = state.plans.lock().unwrap().remove(&id).ok_or("This install was already finished or cancelled")?;
    let report = mods::install(&plan, &content, overwrite, skin_car.as_deref());
    mods::discard(&plan);
    report.map_err(|e| format!("Install failed: {e}"))
}

#[tauri::command]
fn mod_discard(state: State<AppState>, id: u64) {
    if let Some(plan) = state.plans.lock().unwrap().remove(&id) {
        mods::discard(&plan);
    }
}

fn tail(path: Option<std::path::PathBuf>, max_bytes: usize) -> String {
    let Some(bytes) = path.and_then(|p| fs::read(p).ok()) else { return String::new() };
    let start = bytes.len().saturating_sub(max_bytes);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

/// Last output of the Proton process (stdout/stderr).
#[tauri::command]
fn read_launch_log() -> String {
    tail(launch::launch_log_path(), 20_000)
}

/// The game's own `logs/log.txt` (useful to verify it parsed our race.ini).
#[tauri::command]
fn read_game_log(state: State<AppState>) -> String {
    let settings = state.settings.lock().unwrap().clone();
    tail(paths::resolve(&settings).user_docs.map(|d| d.join("logs/log.txt")), 20_000)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { settings: Mutex::new(Settings::load()), monitor: Mutex::new(None), plans: Mutex::new(Default::default()), next_plan: Default::default() })
        .setup(|app| {
            let state = app.state::<AppState>();
            let _ = snapshot(app.handle(), &state); // registers the asset scope
            if state.settings.lock().unwrap().windowed_launcher {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.set_fullscreen(false);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            toggle_fullscreen,
            save_settings,
            list_cars,
            list_skins,
            list_tracks,
            list_weather,
            assist_presets,
            launch_race,
            read_launch_log,
            read_game_log,
            list_input_devices,
            start_input_monitor,
            stop_input_monitor,
            read_controls,
            save_controls,
            g923_preset,
            read_video,
            save_video,
            video_preset,
            read_audio,
            save_audio,
            list_replays,
            play_replay,
            keep_replay,
            delete_replay,
            list_setups,
            import_setup,
            export_setup,
            adopt_setup,
            delete_setup,
            mod_inspect,
            mod_install,
            mod_discard
        ])
        .run(tauri::generate_context!())
        .expect("error while running the launcher");
}
