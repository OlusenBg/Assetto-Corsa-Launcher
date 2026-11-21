//! Driving assists and difficulty: `assists.ini`.
//!
//! Values observed in the shipped presets (`cfg/templates/{gamer,racer,pro}.ini`):
//! ABS / TRACTION_CONTROL 0=off 1=factory 2=full, STABILITY_CONTROL 0-100 %,
//! DAMAGE / VISUALDAMAGE 0-100 %, FUEL_RATE / TYRE_WEAR are multipliers (0=off, 1=normal),
//! the rest are 0/1 switches.

use crate::ini::Ini;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Assists {
    pub ideal_line: bool,
    pub auto_blip: bool,
    pub stability_control: u32,
    pub auto_brake: bool,
    pub auto_shifter: bool,
    pub abs: u8,
    pub traction_control: u8,
    pub auto_clutch: bool,
    pub visual_damage: u32,
    pub damage: u32,
    pub fuel_rate: u32,
    pub tyre_wear: u32,
    pub tyre_blankets: bool,
    pub slipstream: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssistPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub assists: Assists,
}

/// The three stock presets (values copied from the shipped templates).
pub fn presets() -> Vec<AssistPreset> {
    vec![
        AssistPreset { id: "gamer", name: "Gamer", assists: Assists {
            ideal_line: true, auto_blip: true, stability_control: 100, auto_brake: false, auto_shifter: true,
            abs: 2, traction_control: 2, auto_clutch: true, visual_damage: 0, damage: 0, fuel_rate: 0,
            tyre_wear: 1, tyre_blankets: true, slipstream: 1 } },
        AssistPreset { id: "racer", name: "Racer", assists: Assists {
            ideal_line: false, auto_blip: true, stability_control: 50, auto_brake: false, auto_shifter: false,
            abs: 2, traction_control: 1, auto_clutch: true, visual_damage: 20, damage: 20, fuel_rate: 0,
            tyre_wear: 1, tyre_blankets: true, slipstream: 1 } },
        AssistPreset { id: "pro", name: "Pro", assists: Assists {
            ideal_line: false, auto_blip: false, stability_control: 0, auto_brake: false, auto_shifter: false,
            abs: 1, traction_control: 1, auto_clutch: false, visual_damage: 100, damage: 100, fuel_rate: 1,
            tyre_wear: 1, tyre_blankets: false, slipstream: 1 } },
    ]
}

fn b(v: bool) -> u8 {
    v as u8
}

/// Apply assists to `assists.ini` text, preserving unknown keys.
pub fn apply(base: &str, a: &Assists) -> String {
    let mut ini = Ini::parse(base);
    let s = "ASSISTS";
    ini.set(s, "IDEAL_LINE", b(a.ideal_line));
    ini.set(s, "AUTO_BLIP", b(a.auto_blip));
    ini.set(s, "STABILITY_CONTROL", a.stability_control.min(100));
    ini.set(s, "AUTO_BRAKE", b(a.auto_brake));
    ini.set(s, "AUTO_SHIFTER", b(a.auto_shifter));
    ini.set(s, "ABS", a.abs.min(2));
    ini.set(s, "TRACTION_CONTROL", a.traction_control.min(2));
    ini.set(s, "AUTO_CLUTCH", b(a.auto_clutch));
    ini.set(s, "VISUALDAMAGE", a.visual_damage.min(100));
    ini.set(s, "DAMAGE", a.damage.min(100));
    ini.set(s, "FUEL_RATE", a.fuel_rate.min(10));
    ini.set(s, "TYRE_WEAR", a.tyre_wear.min(10));
    ini.set(s, "TYRE_BLANKETS", b(a.tyre_blankets));
    ini.set(s, "SLIPSTREAM", a.slipstream.min(10));
    ini.to_string_crlf()
}

/// Write `<cfg_dir>/assists.ini` (seeded from the user's file, else the shipped default).
pub fn write(cfg_dir: &Path, game_dir: &Path, a: &Assists) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("assists.ini");
    let read = |p: &Path| fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    let base = read(&target).or_else(|| read(&game_dir.join("cfg/assists.ini"))).unwrap_or_default();
    fs::write(target, apply(&base, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_roundtrip() {
        let pro = presets().into_iter().find(|p| p.id == "pro").unwrap().assists;
        let ini = Ini::parse(&apply("[ASSISTS]\r\nABS=2\r\nCUSTOM=7\r\n", &pro));
        assert_eq!(ini.get("ASSISTS", "ABS"), Some("1"));
        assert_eq!(ini.get("ASSISTS", "DAMAGE"), Some("100"));
        assert_eq!(ini.get("ASSISTS", "CUSTOM"), Some("7"));
    }
}
