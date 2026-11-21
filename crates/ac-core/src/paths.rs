//! Launcher settings and auto-detection of the AC install, Proton prefix and
//! Proton version. Nothing here is hardcoded to one machine: every value can
//! be overridden in [`Settings`], and detection walks Steam's own library files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const AC_APP_ID: &str = "244210";

/// User-editable settings, persisted as JSON in the XDG config dir.
/// Any `None` / empty field means "auto-detect".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// `.../steamapps/common/assettocorsa`
    pub game_dir: Option<PathBuf>,
    /// `.../steamapps/compatdata/244210` (the dir that contains `pfx/`)
    pub compat_data_dir: Option<PathBuf>,
    /// A Proton dir (contains the `proton` script), e.g. `.../common/Proton 11.0`
    pub proton_dir: Option<PathBuf>,
    /// Steam root, e.g. `~/.local/share/Steam`
    pub steam_root: Option<PathBuf>,
    /// Optional command prefix, e.g. `gamemoderun` or `mangohud`.
    pub wrapper: String,
    /// Extra environment, one `KEY=VALUE` per line.
    pub extra_env: String,
    /// Launcher starts in a window instead of fullscreen.
    pub windowed_launcher: bool,
    /// Run the game inside gamescope (proper fullscreen on Wayland compositors).
    pub use_gamescope: bool,
    /// Extra gamescope arguments; empty = automatic sizing.
    pub gamescope_args: String,
    /// Keep the launcher visible while the game runs (default: minimize it, restore on exit).
    pub stay_open_on_launch: bool,
}

impl Settings {
    pub fn file() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ac-linux-launcher").join("settings.json"))
    }

    /// Load settings; a missing or corrupt file yields defaults.
    pub fn load() -> Self {
        Self::file()
            .and_then(|p| fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::file().ok_or_else(|| std::io::Error::other("no config dir"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)
    }
}

/// An installed Proton build.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonInstall {
    pub name: String,
    pub dir: PathBuf,
}

/// Severity of a [`Issue`].
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// Something the UI should tell the user about (missing path, Steam closed...).
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub level: Level,
    pub message: String,
}

/// The fully resolved environment.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    pub steam_root: Option<PathBuf>,
    pub game_dir: Option<PathBuf>,
    pub compat_data_dir: Option<PathBuf>,
    pub proton_dir: Option<PathBuf>,
    pub proton_candidates: Vec<ProtonInstall>,
    /// `<compat>/pfx/drive_c/users/steamuser/Documents/Assetto Corsa`
    pub user_docs: Option<PathBuf>,
    pub issues: Vec<Issue>,
}

impl Resolved {
    /// `<user_docs>/cfg`, where the game reads user config.
    pub fn cfg_dir(&self) -> Option<PathBuf> {
        self.user_docs.as_ref().map(|d| d.join("cfg"))
    }
    pub fn content_dir(&self) -> Option<PathBuf> {
        self.game_dir.as_ref().map(|d| d.join("content"))
    }
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Error)
    }
}

fn issue(level: Level, msg: impl Into<String>) -> Issue {
    Issue { level, message: msg.into() }
}

/// Steam root candidates (native, legacy symlink, Flatpak).
fn steam_root_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local/share/Steam"));
        v.push(h.join(".steam/steam"));
        v.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    v.into_iter().filter(|p| p.join("steamapps").is_dir()).collect()
}

/// Extract every `"path"  "<dir>"` value from a `libraryfolders.vdf`.
/// A full VDF parser is overkill: the format is line-oriented key/value pairs.
fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split('"').collect();
            // ["\t", "path", "\t\t", "/some/dir", ""]
            (parts.len() >= 4 && parts[1] == "path").then(|| PathBuf::from(parts[3].replace("\\\\", "\\")))
        })
        .collect()
}

/// All Steam library roots (dirs that contain `steamapps/`).
fn libraries(steam_root: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam_root.to_path_buf()];
    if let Ok(text) = fs::read_to_string(steam_root.join("steamapps/libraryfolders.vdf")) {
        for p in library_paths(&text) {
            if !libs.contains(&p) {
                libs.push(p);
            }
        }
    }
    libs
}

/// Read `"installdir"` from an appmanifest; defaults to `assettocorsa`.
fn install_dir_from_manifest(acf: &str) -> String {
    acf.lines()
        .find_map(|l| {
            let p: Vec<&str> = l.split('"').collect();
            (p.len() >= 4 && p[1] == "installdir").then(|| p[3].to_string())
        })
        .unwrap_or_else(|| "assettocorsa".into())
}

/// Proton builds in a Steam library's `common/` and in `compatibilitytools.d`.
fn find_protons(steam_root: &Path, libs: &[PathBuf]) -> Vec<ProtonInstall> {
    let mut roots: Vec<PathBuf> = libs.iter().map(|l| l.join("steamapps/common")).collect();
    roots.push(steam_root.join("compatibilitytools.d"));
    roots.push(PathBuf::from("/usr/share/steam/compatibilitytools.d"));
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let is_proton = name.starts_with("Proton") || name.contains("Proton");
            if is_proton && e.path().join("proton").is_file() {
                out.push(ProtonInstall { name, dir: e.path() });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Which Proton did Steam last use for this prefix? `config_info` line 1 is a
/// path inside that Proton install (`.../Proton 11.0/files/share/fonts/`).
fn proton_from_config_info(compat: &Path, candidates: &[ProtonInstall]) -> Option<PathBuf> {
    let text = fs::read_to_string(compat.join("config_info")).ok()?;
    let first = text.lines().nth(1)?; // line 0 is the version, line 1 the font path
    candidates
        .iter()
        .find(|c| first.starts_with(&*c.dir.to_string_lossy()))
        .map(|c| c.dir.clone())
}

/// Combine user settings with auto-detection and collect diagnostics.
pub fn resolve(settings: &Settings) -> Resolved {
    let mut r = Resolved::default();

    // --- Steam root
    r.steam_root = settings
        .steam_root
        .clone()
        .or_else(|| steam_root_candidates().into_iter().next());

    let libs = r.steam_root.as_deref().map(libraries).unwrap_or_default();

    // --- Game dir + compat data dir
    r.game_dir = settings.game_dir.clone();
    r.compat_data_dir = settings.compat_data_dir.clone();
    if r.game_dir.is_none() || r.compat_data_dir.is_none() {
        for lib in &libs {
            let acf = lib.join(format!("steamapps/appmanifest_{AC_APP_ID}.acf"));
            let Ok(text) = fs::read_to_string(&acf) else { continue };
            let game = lib.join("steamapps/common").join(install_dir_from_manifest(&text));
            if r.game_dir.is_none() {
                r.game_dir = Some(game);
            }
            if r.compat_data_dir.is_none() {
                r.compat_data_dir = Some(lib.join(format!("steamapps/compatdata/{AC_APP_ID}")));
            }
            break;
        }
    }

    // --- Proton
    r.proton_candidates = r
        .steam_root
        .as_deref()
        .map(|s| find_protons(s, &libs))
        .unwrap_or_default();
    r.proton_dir = settings.proton_dir.clone().or_else(|| {
        r.compat_data_dir
            .as_deref()
            .and_then(|c| proton_from_config_info(c, &r.proton_candidates))
            .or_else(|| {
                // Prefer the newest official "Proton x.y" over GE builds.
                r.proton_candidates
                    .iter()
                    .filter(|p| p.name.starts_with("Proton"))
                    .next_back()
                    .map(|p| p.dir.clone())
            })
    });

    r.user_docs = r.compat_data_dir.as_ref().map(|c| {
        c.join("pfx/drive_c/users/steamuser/Documents/Assetto Corsa")
    });

    // --- Diagnostics
    match &r.game_dir {
        None => r.issues.push(issue(Level::Error, "Assetto Corsa install not found. Set the game folder in Settings.")),
        Some(g) if !g.join("acs.exe").is_file() => r.issues.push(issue(
            Level::Error,
            format!("acs.exe not found in {}", g.display()),
        )),
        _ => {}
    }
    match &r.compat_data_dir {
        None => r.issues.push(issue(Level::Error, "Proton prefix (compatdata/244210) not found. Run the game once from Steam, or set it in Settings.")),
        Some(c) if !c.join("pfx").is_dir() => r.issues.push(issue(
            Level::Error,
            format!("No Proton prefix at {}/pfx. Launch the game once from Steam to create it.", c.display()),
        )),
        _ => {}
    }
    match &r.proton_dir {
        None => r.issues.push(issue(Level::Error, "No Proton install found. Set one in Settings.")),
        Some(p) if !p.join("proton").is_file() => r.issues.push(issue(
            Level::Error,
            format!("{} has no `proton` script.", p.display()),
        )),
        _ => {}
    }
    if r.steam_root.is_none() {
        r.issues.push(issue(Level::Error, "Steam installation not found. Set the Steam root in Settings."));
    } else if !steam_is_running() {
        r.issues.push(issue(Level::Warning, "Steam does not appear to be running. acs.exe needs Steam to start."));
    }
    r
}

/// True if a process named `steam` exists (scans /proc; Linux only).
pub fn steam_is_running() -> bool {
    let Ok(rd) = fs::read_dir("/proc") else { return false };
    rd.flatten().any(|e| {
        e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_digit())
            && fs::read_to_string(e.path().join("comm"))
                .map(|c| c.trim() == "steam")
                .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_vdf() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games/SteamLibrary\"\n\t}\n}";
        assert_eq!(
            library_paths(vdf),
            vec![PathBuf::from("/home/u/.local/share/Steam"), PathBuf::from("/mnt/games/SteamLibrary")]
        );
    }

    #[test]
    fn manifest_installdir() {
        assert_eq!(install_dir_from_manifest("\t\"installdir\"\t\t\"assettocorsa\""), "assettocorsa");
        assert_eq!(install_dir_from_manifest("garbage"), "assettocorsa");
    }
}
