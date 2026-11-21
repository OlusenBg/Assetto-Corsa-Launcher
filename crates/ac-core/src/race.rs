//! Build `race.ini`, the file `acs.exe` reads at startup to know what to run.
//!
//! Strategy: start from a built-in template that mirrors the `race.ini`
//! shipped in `<install>/cfg/`, then overwrite only the keys we control.
//! (If a user `race.ini` already exists we start from it instead, so settings
//! we do not expose yet survive.)

use crate::content;
use crate::ini::Ini;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;

/// Template mirroring the shipped default, minus the stock-launcher specifics.
const TEMPLATE: &str = "\
[RACE]
TRACK=magione
CONFIG_TRACK=
MODEL=
MODEL_CONFIG=
CARS=1
AI_LEVEL=98
FIXED_SETUP=0
PENALTIES=0

[GHOST_CAR]
RECORDING=1
PLAYING=1
SECONDS_ADVANTAGE=0
LOAD=1
FILE=

[REPLAY]
FILENAME=
ACTIVE=0

[LIGHTING]
SUN_ANGLE=-48
TIME_MULT=1
CLOUD_SPEED=0.2

[GROOVE]
VIRTUAL_LAPS=10
MAX_LAPS=30
STARTING_LAPS=0

[DYNAMIC_TRACK]
SESSION_START=100
SESSION_TRANSFER=50
RANDOMNESS=0
LAP_GAIN=1

[REMOTE]
ACTIVE=0
SERVER_IP=
SERVER_PORT=
NAME=
TEAM=
GUID=
REQUESTED_CAR=
PASSWORD=

[LAP_INVALIDATOR]
ALLOWED_TYRES_OUT=-1

[TEMPERATURE]
AMBIENT=26
ROAD=32

[WEATHER]
NAME=3_clear

[CAR_0]
MODEL=
MODEL_CONFIG=
SKIN=
DRIVER_NAME=Driver
NATIONALITY=
AI_LEVEL=96
";

/// Session type values: 1 = practice is confirmed by `out/race_out.json`;
/// 2 = qualify and 3 = race follow community documentation (verify in-game).
pub const SESSION_PRACTICE: u8 = 1;
pub const SESSION_QUALIFY: u8 = 2;
pub const SESSION_RACE: u8 = 3;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    Practice,
    Qualify,
    Race,
}

/// One session in the weekend. Sessions run in the order given.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSpec {
    pub kind: SessionKind,
    /// Time limit for practice/qualify; 0 = unlimited (practice only).
    #[serde(default)]
    pub minutes: u32,
    /// Lap count for races.
    #[serde(default)]
    pub laps: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherSpec {
    /// Folder name in `content/weather`.
    pub id: String,
    /// Time of day in hours, 8.0..=18.0 (the game's range without extensions).
    pub hour: f32,
    /// Ambient temperature in °C.
    pub ambient: i32,
    /// Clock speed multiplier (1 = real time).
    #[serde(default = "one")]
    pub time_mult: u32,
    /// Asphalt temperature offset from `TEMPERATURE_COEFF` (-1..1) of the weather.
    #[serde(default)]
    pub temperature_coeff: f32,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSpec {
    /// Number of opponents (clamped to the layout's pit boxes - 1).
    pub count: u32,
    /// Skill 0-100 (`AI_LEVEL`).
    pub strength: u32,
    /// Aggression 0-100 (`AI_AGGRESSION`).
    pub aggression: u32,
    /// Car ids for the opponents, used round-robin. Empty = same car as the player.
    #[serde(default)]
    pub cars: Vec<String>,
}

/// Everything the UI can ask for.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceRequest {
    pub car: String,
    pub skin: String,
    pub track: String,
    /// `CONFIG_TRACK` value; empty = default layout.
    #[serde(default)]
    pub track_config: String,
    #[serde(default)]
    pub driver_name: String,
    pub sessions: Vec<SessionSpec>,
    pub weather: WeatherSpec,
    pub ai: AiSpec,
    #[serde(default)]
    pub penalties: bool,
    /// Grid slot in the race (1 = pole).
    #[serde(default = "one")]
    pub start_position: u32,
}

/// A resolved opponent.
#[derive(Debug, Clone)]
pub struct AiDriver {
    pub car: String,
    pub skin: String,
    pub name: String,
}

/// Reject ids that could escape the content dir or break the INI line format.
fn check_id(label: &str, v: &str, allow_empty: bool) -> io::Result<()> {
    let bad = (v.is_empty() && !allow_empty)
        || v.contains(['/', '\\', '\n', '\r', ';', '='])
        || v.contains("..");
    if bad {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {label}: {v:?}")));
    }
    Ok(())
}

/// Sun angle for a time of day: 13:00 = 0°, 16° per hour (so 08:00 = -80°, 18:00 = +80°).
pub fn sun_angle(hour: f32) -> i32 {
    ((hour.clamp(8.0, 18.0) - 13.0) * 16.0).round() as i32
}

/// Approximate asphalt temperature from ambient and the weather's coefficient.
/// (The game's launcher uses a more detailed curve; this is a simple stand-in.)
pub fn road_temperature(ambient: i32, coeff: f32) -> i32 {
    ambient + (6.0 + 6.0 * coeff.clamp(-1.0, 1.0)).round() as i32
}

/// Pick opponents: cars round-robin, skins rotating through each car's skins,
/// names from the game's `system/data/drivers.txt`.
pub fn ai_roster(game_dir: &Path, req: &RaceRequest) -> Vec<AiDriver> {
    let content = game_dir.join("content");
    let pit_limit = content::track_layout(&content, &req.track, &req.track_config)
        .and_then(|l| l.pitboxes)
        .map(|p| p.saturating_sub(1))
        .unwrap_or(20);
    let count = req.ai.count.min(pit_limit).min(40) as usize;
    let cars: Vec<&str> = if req.ai.cars.is_empty() {
        vec![req.car.as_str()]
    } else {
        req.ai.cars.iter().map(String::as_str).collect()
    };
    let names: Vec<String> = fs::read_to_string(game_dir.join("system/data/drivers.txt"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Skins per car; when AI uses the player's car, the player's own skin goes last.
    let mut skins: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for c in &cars {
        skins.entry(c).or_insert_with(|| {
            let mut v: Vec<String> = content::list_skins(&content, c).into_iter().map(|s| s.id).collect();
            if *c == req.car {
                if let Some(i) = v.iter().position(|s| *s == req.skin) {
                    let own = v.remove(i);
                    v.push(own);
                }
            }
            v
        });
    }
    (0..count)
        .map(|i| {
            let car = cars[i % cars.len()];
            let list = &skins[car];
            let nth = i / cars.len();
            AiDriver {
                car: car.to_string(),
                skin: list.get(nth % list.len().max(1)).cloned().unwrap_or_default(),
                name: names.get(i % names.len().max(1)).cloned().unwrap_or_else(|| format!("Driver {}", i + 1)),
            }
        })
        .collect()
}

/// Produce race.ini text. `existing` is the current user race.ini, if any.
pub fn build(existing: Option<&str>, req: &RaceRequest, roster: &[AiDriver]) -> io::Result<String> {
    check_id("car", &req.car, false)?;
    check_id("skin", &req.skin, false)?;
    check_id("track", &req.track, false)?;
    check_id("track layout", &req.track_config, true)?;
    check_id("driver name", &req.driver_name, true)?;
    check_id("weather", &req.weather.id, false)?;
    for d in roster {
        check_id("opponent car", &d.car, false)?;
        check_id("opponent skin", &d.skin, true)?;
        check_id("opponent name", &d.name, true)?;
    }
    if req.sessions.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "at least one session is required"));
    }

    let mut ini = Ini::parse(existing.unwrap_or(TEMPLATE));

    // Drop opponents and sessions from a previous race.ini.
    for name in ini.sections().into_iter().map(String::from).collect::<Vec<_>>() {
        let up = name.to_ascii_uppercase();
        if (up.starts_with("CAR_") && up != "CAR_0") || up.starts_with("SESSION_") {
            ini.remove_section(&name);
        }
    }

    // A previous replay launch must not leak into a normal session.
    ini.set("REPLAY", "ACTIVE", 0);
    ini.set("REPLAY", "FILENAME", "");

    ini.set("RACE", "TRACK", &req.track);
    ini.set("RACE", "CONFIG_TRACK", &req.track_config);
    ini.set("RACE", "MODEL", &req.car);
    ini.set("RACE", "MODEL_CONFIG", "");
    ini.set("RACE", "CARS", 1 + roster.len());
    ini.set("RACE", "AI_LEVEL", req.ai.strength.min(100));
    ini.set("RACE", "PENALTIES", req.penalties as u8);

    let w = &req.weather;
    ini.set("LIGHTING", "SUN_ANGLE", sun_angle(w.hour));
    ini.set("LIGHTING", "TIME_MULT", w.time_mult.clamp(0, 60));
    ini.set("WEATHER", "NAME", &w.id);
    ini.set("TEMPERATURE", "AMBIENT", w.ambient.clamp(0, 36));
    ini.set("TEMPERATURE", "ROAD", road_temperature(w.ambient.clamp(0, 36), w.temperature_coeff));

    ini.set("CAR_0", "MODEL", &req.car);
    ini.set("CAR_0", "MODEL_CONFIG", "");
    ini.set("CAR_0", "SKIN", &req.skin);
    ini.set("CAR_0", "DRIVER_NAME", if req.driver_name.is_empty() { "Driver" } else { &req.driver_name });
    for (i, d) in roster.iter().enumerate() {
        let sec = format!("CAR_{}", i + 1);
        ini.set(&sec, "MODEL", &d.car);
        ini.set(&sec, "MODEL_CONFIG", "");
        ini.set(&sec, "SKIN", &d.skin);
        ini.set(&sec, "DRIVER_NAME", &d.name);
        ini.set(&sec, "NATIONALITY", "");
        ini.set(&sec, "AI_LEVEL", req.ai.strength.min(100));
        ini.set(&sec, "AI_AGGRESSION", req.ai.aggression.min(100));
    }

    for (i, s) in req.sessions.iter().enumerate() {
        let sec = format!("SESSION_{i}");
        match s.kind {
            SessionKind::Practice => {
                ini.set(&sec, "NAME", "Practice");
                ini.set(&sec, "TYPE", SESSION_PRACTICE);
                ini.set(&sec, "DURATION_MINUTES", s.minutes);
                ini.set(&sec, "SPAWN_SET", "PIT");
            }
            SessionKind::Qualify => {
                ini.set(&sec, "NAME", "Qualify");
                ini.set(&sec, "TYPE", SESSION_QUALIFY);
                ini.set(&sec, "DURATION_MINUTES", s.minutes.max(1));
                ini.set(&sec, "SPAWN_SET", "PIT");
            }
            SessionKind::Race => {
                ini.set(&sec, "NAME", "Race");
                ini.set(&sec, "TYPE", SESSION_RACE);
                ini.set(&sec, "LAPS", s.laps.max(1));
                ini.set(&sec, "SPAWN_SET", "START");
                // Grid slot is clamped to the field size (player + opponents).
                ini.set(&sec, "STARTING_POSITION", req.start_position.clamp(1, 1 + roster.len() as u32));
            }
        }
    }
    Ok(ini.to_string_crlf())
}

/// race.ini text that plays back a saved replay (`file` = name inside the replay folder).
pub fn build_replay(existing: Option<&str>, file: &str) -> io::Result<String> {
    check_id("replay file", file, false)?;
    let mut ini = Ini::parse(existing.unwrap_or(TEMPLATE));
    ini.set("REPLAY", "FILENAME", file);
    ini.set("REPLAY", "ACTIVE", 1);
    Ok(ini.to_string_crlf())
}

/// Write race.ini for replay playback (keeps the rest of the current file).
pub fn write_replay(cfg_dir: &Path, file: &str) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let path = cfg_dir.join("race.ini");
    let existing = fs::read(&path).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    fs::write(path, build_replay(existing.as_deref(), file)?)
}

/// Write `<cfg_dir>/race.ini`, creating the dir if needed. Keeps a one-time
/// backup of a pre-existing file as `race.ini.launcher-backup`.
pub fn write(cfg_dir: &Path, game_dir: &Path, req: &RaceRequest) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let path = cfg_dir.join("race.ini");
    let existing = fs::read(&path).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    if existing.is_some() {
        let backup = cfg_dir.join("race.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&path, backup)?;
        }
    }
    let roster = ai_roster(game_dir, req);
    fs::write(path, build(existing.as_deref(), req, &roster)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> RaceRequest {
        RaceRequest {
            car: "ks_mazda_mx5_cup".into(),
            skin: "01_cup_07".into(),
            track: "ks_nordschleife".into(),
            track_config: "touristenfahrten".into(),
            driver_name: String::new(),
            sessions: vec![
                SessionSpec { kind: SessionKind::Practice, minutes: 0, laps: 0 },
                SessionSpec { kind: SessionKind::Race, minutes: 0, laps: 5 },
            ],
            weather: WeatherSpec { id: "3_clear".into(), hour: 10.0, ambient: 22, time_mult: 1, temperature_coeff: 1.0 },
            ai: AiSpec { count: 2, strength: 90, aggression: 40, cars: vec![] },
            penalties: true,
            start_position: 2,
        }
    }

    fn roster() -> Vec<AiDriver> {
        vec![
            AiDriver { car: "ks_mazda_mx5_cup".into(), skin: "02_cup_23".into(), name: "Rossi".into() },
            AiDriver { car: "ks_mazda_mx5_cup".into(), skin: "03_cup_24".into(), name: "Bianchi".into() },
        ]
    }

    #[test]
    fn builds_full_weekend() {
        let out = build(None, &req(), &roster()).unwrap();
        let ini = Ini::parse(&out);
        assert_eq!(ini.get("RACE", "CARS"), Some("3"));
        assert_eq!(ini.get("RACE", "PENALTIES"), Some("1"));
        assert_eq!(ini.get("CAR_2", "DRIVER_NAME"), Some("Bianchi"));
        assert_eq!(ini.get("CAR_1", "AI_AGGRESSION"), Some("40"));
        assert_eq!(ini.get("SESSION_0", "TYPE"), Some("1"));
        assert_eq!(ini.get("SESSION_1", "TYPE"), Some("3"));
        assert_eq!(ini.get("SESSION_1", "LAPS"), Some("5"));
        assert_eq!(ini.get("SESSION_1", "STARTING_POSITION"), Some("2"));
        assert_eq!(ini.get("LIGHTING", "SUN_ANGLE"), Some("-48"));
        assert_eq!(ini.get("TEMPERATURE", "ROAD"), Some("34"));
        assert!(out.contains("\r\n"));
    }

    #[test]
    fn sun_angle_mapping() {
        assert_eq!(sun_angle(13.0), 0);
        assert_eq!(sun_angle(8.0), -80);
        assert_eq!(sun_angle(18.0), 80);
        assert_eq!(sun_angle(30.0), 80);
    }

    #[test]
    fn replaces_stale_sessions_and_ai() {
        let first = build(None, &req(), &roster()).unwrap();
        let mut r = req();
        r.sessions.truncate(1);
        let out = build(Some(&first), &r, &[]).unwrap();
        let ini = Ini::parse(&out);
        assert!(ini.get("CAR_1", "MODEL").is_none());
        assert!(ini.get("SESSION_1", "TYPE").is_none());
        assert_eq!(ini.get("RACE", "CARS"), Some("1"));
    }

    #[test]
    fn rejects_injection_and_empty_sessions() {
        let mut r = req();
        r.car = "x\r\n[REMOTE]".into();
        assert!(build(None, &r, &[]).is_err());
        let mut r = req();
        r.sessions.clear();
        assert!(build(None, &r, &[]).is_err());
    }

    #[test]
    fn replay_flag_is_reset_for_normal_sessions() {
        let replay = build_replay(None, "x.acreplay").unwrap();
        assert_eq!(Ini::parse(&replay).get("REPLAY", "ACTIVE"), Some("1"));
        let out = build(Some(&replay), &req(), &[]).unwrap();
        assert_eq!(Ini::parse(&out).get("REPLAY", "ACTIVE"), Some("0"));
        assert_eq!(Ini::parse(&out).get("REPLAY", "FILENAME"), Some(""));
        assert!(build_replay(None, "../evil").is_err());
    }
}
