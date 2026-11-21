//! `audio.ini`: volume levels. Like every cfg file, only the user copy is read
//! by the game, so we seed it from the shipped default.

use crate::ini::Ini;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Volume levels, 0.0-1.0 (the file stores plain floats).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub master: f32,
    pub engine: f32,
    pub wind: f32,
    pub tyres: f32,
    pub opponents: f32,
    pub music: f32,
    pub ui_sounds: f32,
    pub surfaces: f32,
    pub dirt_bottom: f32,
    /// `[SKIDS] ENTRY_POINT`: tyre-squeal threshold (percent of the slip curve).
    pub skid_entry_point: u32,
}

const KEYS: &[(&str, &str)] = &[
    ("MASTER", "master"), ("ENGINE", "engine"), ("WIND", "wind"), ("TYRES", "tyres"), ("OPPONENTS", "opponents"),
    ("MUSIC", "music"), ("UISOUNDS", "ui_sounds"), ("SURFACES", "surfaces"), ("DIRT_BOTTOM", "dirt_bottom"),
];

pub fn read_config(text: &str) -> AudioConfig {
    let ini = Ini::parse(text);
    let f = |k: &str, d: f32| ini.get("LEVELS", k).and_then(|v| v.trim().parse().ok()).unwrap_or(d);
    AudioConfig {
        master: f("MASTER", 1.0),
        engine: f("ENGINE", 1.0),
        wind: f("WIND", 0.4),
        tyres: f("TYRES", 0.5),
        opponents: f("OPPONENTS", 0.8),
        music: f("MUSIC", 0.5),
        ui_sounds: f("UISOUNDS", 0.7),
        surfaces: f("SURFACES", 0.9),
        dirt_bottom: f("DIRT_BOTTOM", 1.0),
        skid_entry_point: ini.get("SKIDS", "ENTRY_POINT").and_then(|v| v.trim().parse().ok()).unwrap_or(100),
    }
}

pub fn apply_config(base: &str, c: &AudioConfig) -> String {
    let mut ini = Ini::parse(base);
    ini.set("HEADER", "VERSION", ini.get("HEADER", "VERSION").unwrap_or("1").to_string());
    let vals = [c.master, c.engine, c.wind, c.tyres, c.opponents, c.music, c.ui_sounds, c.surfaces, c.dirt_bottom];
    for ((key, _), v) in KEYS.iter().zip(vals) {
        ini.set("LEVELS", key, (v.clamp(0.0, 1.0) * 100.0).round() / 100.0);
    }
    ini.set("SKIDS", "ENTRY_POINT", c.skid_entry_point.min(100));
    ini.to_string_crlf()
}

pub fn load_text(cfg_dir: &Path, game_dir: &Path) -> String {
    let read = |p: &Path| fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    read(&cfg_dir.join("audio.ini")).or_else(|| read(&game_dir.join("cfg/audio.ini"))).unwrap_or_default()
}

pub fn save_config(cfg_dir: &Path, game_dir: &Path, c: &AudioConfig) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("audio.ini");
    let base = load_text(cfg_dir, game_dir);
    if target.exists() {
        let backup = cfg_dir.join("audio.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&target, backup)?;
        }
    }
    fs::write(target, apply_config(&base, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut c = read_config("[LEVELS]\r\nMASTER=1\r\nENGINE=1.0\r\n");
        c.engine = 0.65;
        c.music = 0.0;
        let out = apply_config("[HEADER]\r\nVERSION=1\r\n[LEVELS]\r\nMASTER=1\r\n", &c);
        let back = read_config(&out);
        assert_eq!(back.engine, 0.65);
        assert_eq!(back.music, 0.0);
    }
}
