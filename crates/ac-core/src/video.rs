//! `video.ini`: the game reads ONLY the user copy (`<docs>/cfg/video.ini`);
//! without a `[VIDEO]` block it falls back to a small window. We seed it from
//! the shipped default and set resolution/fullscreen explicitly.

use crate::ini::Ini;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VideoMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub fullscreen: bool,
}

/// Apply a display mode to `video.ini` text.
pub fn apply_mode(base: &str, m: &VideoMode) -> String {
    let mut ini = Ini::parse(base);
    ini.set("VIDEO", "WIDTH", m.width);
    ini.set("VIDEO", "HEIGHT", m.height);
    ini.set("VIDEO", "REFRESH", m.refresh);
    ini.set("VIDEO", "FULLSCREEN", m.fullscreen as u8);
    ini.to_string_crlf()
}

/// Write `<cfg_dir>/video.ini`. If a user file with a `[VIDEO]` block exists it
/// is edited in place; otherwise the shipped default is the base.
pub fn write_mode(cfg_dir: &Path, game_dir: &Path, m: &VideoMode) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("video.ini");
    let read = |p: &Path| fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    let user = read(&target).filter(|t| Ini::parse(t).sections().iter().any(|s| s.eq_ignore_ascii_case("VIDEO")));
    let base = user.or_else(|| read(&game_dir.join("cfg/video.ini"))).unwrap_or_default();
    if target.exists() {
        let backup = cfg_dir.join("video.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&target, backup)?;
        }
    }
    fs::write(target, apply_mode(&base, m))
}

/// Best-effort primary display mode via `xrandr` (works on X11 and XWayland).
pub fn detect_primary_mode() -> Option<VideoMode> {
    let out = std::process::Command::new("xrandr").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines().peekable();
    while let Some(l) = lines.next() {
        if l.contains(" connected") {
            let mode_line = lines.next()?; // first mode line, e.g. "   1920x1080     74.91*+"
            let mut parts = mode_line.split_whitespace();
            let (w, h) = parts.next()?.split_once('x')?;
            let hz: f32 = parts.find(|p| p.contains('*'))?.trim_matches(|c| c == '*' || c == '+').parse().ok()?;
            return Some(VideoMode { width: w.parse().ok()?, height: h.parse().ok()?, refresh: hz.round() as u32, fullscreen: true });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Full, editable model for the Video page.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Every `video.ini` setting the launcher exposes. Field names are the UI's.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    /// Frame cap in FPS; 0 = unlimited (the file stores milliseconds per frame).
    pub fps_cap: u32,
    pub anisotropic: u32,
    /// MSAA samples: 0, 2, 4 or 8.
    pub aa_samples: u32,
    pub aa_quality: u32,
    pub shadow_map_size: u32,
    /// 0-5
    pub world_detail: u32,
    pub motion_blur: u32,
    pub fxaa: bool,
    pub smoke: u32,
    pub smoke_in_mirror: bool,
    pub cubemap_size: u32,
    pub cubemap_faces_per_frame: u32,
    pub cubemap_farplane: u32,
    pub mirror_size: u32,
    pub mirror_hq: bool,
    pub hide_arms: bool,
    pub hide_steer: bool,
    pub saturation: u32,
    pub pp_enabled: bool,
    pub heat_shimmer: bool,
    pub rays_of_god: bool,
    /// 0-5 (effect buffer scale)
    pub pp_quality: u32,
    pub glare: u32,
    pub dof: u32,
    /// Name of a file in `system/cfg/ppfilters` (without `.ini`).
    pub pp_filter: String,
}

fn n<T: std::str::FromStr>(ini: &Ini, s: &str, k: &str, d: T) -> T {
    ini.get(s, k).and_then(|v| v.trim().parse().ok()).unwrap_or(d)
}
fn flag(ini: &Ini, s: &str, k: &str, d: bool) -> bool {
    ini.get(s, k).and_then(|v| v.trim().parse::<i32>().ok()).map(|v| v != 0).unwrap_or(d)
}

/// Parse `video.ini` text. Missing keys get the shipped defaults.
pub fn read_config(text: &str) -> VideoConfig {
    let ini = Ini::parse(text);
    let cap_ms: u32 = n(&ini, "VIDEO", "FPS_CAP_MS", 0);
    VideoConfig {
        width: n(&ini, "VIDEO", "WIDTH", 1600),
        height: n(&ini, "VIDEO", "HEIGHT", 900),
        refresh: n(&ini, "VIDEO", "REFRESH", 60),
        fullscreen: flag(&ini, "VIDEO", "FULLSCREEN", false),
        vsync: flag(&ini, "VIDEO", "VSYNC", false),
        fps_cap: if cap_ms == 0 { 0 } else { (1000.0 / cap_ms as f32).round() as u32 },
        anisotropic: n(&ini, "VIDEO", "ANISOTROPIC", 8),
        aa_samples: n(&ini, "VIDEO", "AASAMPLES", 2),
        aa_quality: n(&ini, "VIDEO", "AAQUALITY", 0),
        shadow_map_size: n(&ini, "VIDEO", "SHADOW_MAP_SIZE", 2048),
        world_detail: n(&ini, "ASSETTOCORSA", "WORLD_DETAIL", 5),
        motion_blur: n(&ini, "EFFECTS", "MOTION_BLUR", 0),
        fxaa: flag(&ini, "POST_PROCESS", "FXAA", true),
        smoke: n(&ini, "EFFECTS", "SMOKE", 3),
        smoke_in_mirror: flag(&ini, "EFFECTS", "RENDER_SMOKE_IN_MIRROR", true),
        cubemap_size: n(&ini, "CUBEMAP", "SIZE", 512),
        cubemap_faces_per_frame: n(&ini, "CUBEMAP", "FACES_PER_FRAME", 0),
        cubemap_farplane: n(&ini, "CUBEMAP", "FARPLANE", 500),
        mirror_size: n(&ini, "MIRROR", "SIZE", 512),
        mirror_hq: flag(&ini, "MIRROR", "HQ", false),
        hide_arms: flag(&ini, "ASSETTOCORSA", "HIDE_ARMS", false),
        hide_steer: flag(&ini, "ASSETTOCORSA", "HIDE_STEER", false),
        saturation: n(&ini, "SATURATION", "LEVEL", 100),
        pp_enabled: flag(&ini, "POST_PROCESS", "ENABLED", true),
        heat_shimmer: flag(&ini, "POST_PROCESS", "HEAT_SHIMMER", true),
        rays_of_god: flag(&ini, "POST_PROCESS", "RAYS_OF_GOD", true),
        pp_quality: n(&ini, "POST_PROCESS", "QUALITY", 3),
        glare: n(&ini, "POST_PROCESS", "GLARE", 3),
        dof: n(&ini, "POST_PROCESS", "DOF", 3),
        pp_filter: ini.get("POST_PROCESS", "FILTER").unwrap_or("default").to_string(),
    }
}

/// Apply the model onto video.ini text, keeping keys we do not manage.
pub fn apply_config(base: &str, c: &VideoConfig) -> String {
    let mut ini = Ini::parse(base);
    let b = |v: bool| v as u8;
    ini.set("VIDEO", "WIDTH", c.width);
    ini.set("VIDEO", "HEIGHT", c.height);
    ini.set("VIDEO", "REFRESH", c.refresh);
    ini.set("VIDEO", "FULLSCREEN", b(c.fullscreen));
    ini.set("VIDEO", "VSYNC", b(c.vsync));
    ini.set("VIDEO", "FPS_CAP_MS", if c.fps_cap == 0 { 0 } else { (1000.0 / c.fps_cap as f32).round() as u32 });
    ini.set("VIDEO", "ANISOTROPIC", c.anisotropic.min(16));
    ini.set("VIDEO", "AASAMPLES", c.aa_samples);
    ini.set("VIDEO", "AAQUALITY", c.aa_quality);
    ini.set("VIDEO", "SHADOW_MAP_SIZE", c.shadow_map_size);
    ini.set("ASSETTOCORSA", "WORLD_DETAIL", c.world_detail.min(5));
    ini.set("ASSETTOCORSA", "HIDE_ARMS", b(c.hide_arms));
    ini.set("ASSETTOCORSA", "HIDE_STEER", b(c.hide_steer));
    ini.set("EFFECTS", "MOTION_BLUR", c.motion_blur);
    ini.set("EFFECTS", "FXAA", b(c.fxaa));
    ini.set("EFFECTS", "SMOKE", c.smoke);
    ini.set("EFFECTS", "RENDER_SMOKE_IN_MIRROR", b(c.smoke_in_mirror));
    ini.set("CUBEMAP", "SIZE", c.cubemap_size);
    ini.set("CUBEMAP", "FACES_PER_FRAME", c.cubemap_faces_per_frame);
    ini.set("CUBEMAP", "FARPLANE", c.cubemap_farplane);
    ini.set("MIRROR", "SIZE", c.mirror_size);
    ini.set("MIRROR", "HQ", b(c.mirror_hq));
    ini.set("SATURATION", "LEVEL", c.saturation.min(200));
    ini.set("POST_PROCESS", "ENABLED", b(c.pp_enabled));
    ini.set("POST_PROCESS", "HEAT_SHIMMER", b(c.heat_shimmer));
    ini.set("POST_PROCESS", "RAYS_OF_GOD", b(c.rays_of_god));
    ini.set("POST_PROCESS", "FXAA", b(c.fxaa));
    ini.set("POST_PROCESS", "QUALITY", c.pp_quality.min(5));
    ini.set("POST_PROCESS", "GLARE", c.glare.min(5));
    ini.set("POST_PROCESS", "DOF", c.dof.min(5));
    let filter_ok = !c.pp_filter.is_empty() && !c.pp_filter.contains(['/', '\\', ';', '=', '\n', '\r']) && !c.pp_filter.contains("..");
    ini.set("POST_PROCESS", "FILTER", if filter_ok { c.pp_filter.as_str() } else { "default" });
    ini.to_string_crlf()
}

/// Current user `video.ini`; if it has no `[VIDEO]` block (the game's own stub) use the shipped default.
pub fn load_text(cfg_dir: &Path, game_dir: &Path) -> String {
    let read = |p: &Path| fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    read(&cfg_dir.join("video.ini"))
        .filter(|t| Ini::parse(t).sections().iter().any(|s| s.eq_ignore_ascii_case("VIDEO")))
        .or_else(|| read(&game_dir.join("cfg/video.ini")))
        .unwrap_or_default()
}

pub fn save_config(cfg_dir: &Path, game_dir: &Path, c: &VideoConfig) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("video.ini");
    let base = load_text(cfg_dir, game_dir);
    if target.exists() {
        let backup = cfg_dir.join("video.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&target, backup)?;
        }
    }
    fs::write(target, apply_config(&base, c))
}

/// Quality presets: only the graphics-quality fields; resolution, fullscreen,
/// vsync and FPS cap are left alone. These are the launcher's own sensible
/// groupings, not Kunos' presets.
pub fn apply_quality_preset(c: &mut VideoConfig, preset: &str) -> bool {
    // (aniso, aa, shadow, world, cube, faces, far, mirror, hq, pp q, glare, dof, smoke)
    let v = match preset {
        "low" => (2, 0, 1024, 1, 256, 1, 250, 256, false, 1, 1, 0, 1),
        "medium" => (4, 2, 2048, 3, 512, 1, 400, 512, false, 2, 2, 1, 2),
        "high" => (8, 4, 2048, 4, 512, 0, 500, 512, true, 3, 3, 3, 3),
        "ultra" => (16, 8, 4096, 5, 1024, 0, 800, 1024, true, 4, 4, 4, 3),
        _ => return false,
    };
    c.anisotropic = v.0;
    c.aa_samples = v.1;
    c.shadow_map_size = v.2;
    c.world_detail = v.3;
    c.cubemap_size = v.4;
    c.cubemap_faces_per_frame = v.5;
    c.cubemap_farplane = v.6;
    c.mirror_size = v.7;
    c.mirror_hq = v.8;
    c.pp_quality = v.9;
    c.glare = v.10;
    c.dof = v.11;
    c.smoke = v.12;
    true
}

/// Post-processing filters available in `system/cfg/ppfilters`.
pub fn list_filters(game_dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(game_dir.join("system/cfg/ppfilters"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).filter(|_| e.path().extension().is_some_and(|x| x == "ini")))
        .collect();
    v.sort();
    v
}

/// A resolution with the refresh rates the display offers for it.
#[derive(Debug, Clone, Serialize)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: Vec<u32>,
}

/// All modes of the first connected output via `xrandr`, largest first.
/// Falls back to a common list when xrandr is unavailable.
pub fn detect_modes() -> Vec<DisplayMode> {
    let fallback = || {
        [(1280, 720), (1600, 900), (1920, 1080), (2560, 1440), (3840, 2160)]
            .into_iter()
            .map(|(width, height)| DisplayMode { width, height, refresh: vec![60] })
            .collect()
    };
    let Ok(out) = std::process::Command::new("xrandr").output() else { return fallback() };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut modes: Vec<DisplayMode> = Vec::new();
    let mut in_output = false;
    for line in text.lines() {
        if line.contains(" connected") {
            if in_output {
                break; // only the first connected output
            }
            in_output = true;
            continue;
        }
        if !in_output || !line.starts_with("   ") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some((w, h)) = parts.next().and_then(|r| r.split_once('x')) else { continue };
        let (Ok(width), Ok(height)) = (w.parse::<u32>(), h.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<u32>()) else { continue };
        let rates: Vec<u32> = parts
            .filter_map(|p| p.trim_matches(|c| c == '*' || c == '+').parse::<f32>().ok())
            .map(|f| f.round() as u32)
            .collect();
        let mut rates = rates;
        rates.sort_unstable_by(|a, b| b.cmp(a));
        rates.dedup();
        if !rates.is_empty() {
            modes.push(DisplayMode { width, height, refresh: rates });
        }
    }
    if modes.is_empty() {
        return fallback();
    }
    modes.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));
    modes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_video_block() {
        let out = apply_mode("[POST_PROCESS]\r\nFILTER=default\r\n", &VideoMode { width: 1920, height: 1080, refresh: 75, fullscreen: true });
        let ini = Ini::parse(&out);
        assert_eq!(ini.get("VIDEO", "WIDTH"), Some("1920"));
        assert_eq!(ini.get("VIDEO", "FULLSCREEN"), Some("1"));
        assert_eq!(ini.get("POST_PROCESS", "FILTER"), Some("default"));
    }

    #[test]
    fn config_roundtrip_and_fps_cap() {
        let mut c = read_config("[POST_PROCESS]\r\nFILTER=default\r\n");
        c.fps_cap = 120;
        c.fullscreen = true;
        let out = apply_config("[VIDEO]\r\nCUSTOM=1\r\n", &c);
        assert!(out.contains("FPS_CAP_MS=8"));
        assert!(out.contains("CUSTOM=1"));
        let back = read_config(&out);
        assert_eq!(back.fps_cap, 125); // 8 ms rounds to 125 fps
        assert!(back.fullscreen);
    }

    #[test]
    fn presets_leave_display_alone() {
        let mut c = read_config("");
        let before = (c.width, c.height, c.fullscreen, c.vsync);
        assert!(apply_quality_preset(&mut c, "ultra"));
        assert_eq!((c.width, c.height, c.fullscreen, c.vsync), before);
        assert_eq!(c.shadow_map_size, 4096);
        assert!(!apply_quality_preset(&mut c, "nope"));
    }
}
