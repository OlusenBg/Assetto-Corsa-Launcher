//! Wheel bindings: generate `controls.ini` for a steering wheel.
//!
//! Axis/button numbers are the indices the game uses under Wine. Wine numbers
//! axes in the order the device reports them (not by fixed DirectInput
//! offsets): the G923 has X, Y, Z, Rz so they are 0, 1, 2, 3. Buttons are
//! numbered in the order of the evdev button codes the device reports.
//! Observed values for the Logitech G923 (PC/PS mode, USB 046d:c266) were
//! measured by moving each control and recording `/dev/input/event*`.

use crate::ini::Ini;
use std::fs;
use std::io;
use std::path::Path;

/// Everything needed to describe one wheel in `controls.ini`.
#[derive(Debug, Clone)]
pub struct WheelProfile {
    /// Name as DirectInput reports it (also what the game prints in its log).
    pub device_name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub steer_axis: u8,
    pub throttle_axis: u8,
    pub brake_axis: u8,
    pub clutch_axis: u8,
    /// Pedals report 255 at rest and 0 pressed; AC needs MIN/MAX swapped.
    pub invert_pedals: bool,
    pub gear_up_button: i32,
    pub gear_down_button: i32,
    /// Steering lock in degrees (G923: 900).
    pub lock_degrees: u32,
    pub ff_gain: f32,
}

impl WheelProfile {
    /// Logitech G923 (PlayStation/PC version), measured on real hardware.
    pub fn g923() -> Self {
        WheelProfile {
            device_name: "Logitech G923 Racing Wheel for PlayStation 4 and PC".into(),
            vendor_id: 0x046d,
            product_id: 0xc266,
            steer_axis: 0,    // ABS_X
            throttle_axis: 2, // ABS_Z
            brake_axis: 3,    // ABS_RZ (4th reported axis; 5 did not work in testing)
            clutch_axis: 1,   // ABS_Y
            invert_pedals: true,
            gear_up_button: 4,   // right paddle
            gear_down_button: 5, // left paddle
            lock_degrees: 900,
            ff_gain: 1.0,
        }
    }

    /// Windows product GUID string: `<PID><VID>-0000-0000-0000-504944564944`.
    pub fn pguid(&self) -> String {
        format!("{:04X}{:04X}-0000-0000-0000-504944564944", self.product_id, self.vendor_id)
    }
}

/// Apply a wheel profile to a `controls.ini` text (existing user file or the shipped default).
pub fn apply_wheel(base: &str, p: &WheelProfile) -> String {
    let mut ini = Ini::parse(base);
    ini.set("HEADER", "INPUT_METHOD", "WHEEL");
    ini.set("CONTROLLERS", "CON0", &p.device_name);
    ini.set("CONTROLLERS", "PGUID0", p.pguid());

    ini.set("STEER", "JOY", 0);
    ini.set("STEER", "AXLE", p.steer_axis);
    ini.set("STEER", "SCALE", 1);
    ini.set("STEER", "LOCK", p.lock_degrees);
    ini.set("STEER", "FF_GAIN", p.ff_gain);

    let (min, max) = if p.invert_pedals { (1, -1) } else { (-1, 1) };
    for (section, axis) in [("THROTTLE", p.throttle_axis), ("BRAKES", p.brake_axis), ("CLUTCH", p.clutch_axis)] {
        ini.set(section, "JOY", 0);
        ini.set(section, "AXLE", axis);
        ini.set(section, "MIN", min);
        ini.set(section, "MAX", max);
    }
    ini.set("GEARUP", "JOY", 0);
    ini.set("GEARUP", "BUTTON", p.gear_up_button);
    ini.set("GEARDN", "JOY", 0);
    ini.set("GEARDN", "BUTTON", p.gear_down_button);
    ini.to_string_crlf()
}

/// Write `<cfg_dir>/controls.ini`, based on the user's current file or, failing
/// that, the shipped default. A one-time backup is kept as `controls.ini.launcher-backup`.
pub fn write_wheel(cfg_dir: &Path, game_dir: &Path, p: &WheelProfile) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("controls.ini");
    let base = fs::read(&target)
        .or_else(|_| fs::read(game_dir.join("cfg/controls.ini")))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    if target.exists() {
        let backup = cfg_dir.join("controls.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&target, backup)?;
        }
    }
    fs::write(target, apply_wheel(&base, p))
}

// ---------------------------------------------------------------------------
// Full, editable model used by the Controls page.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// An entry of `[CONTROLLERS]`; `JOY=n` elsewhere refers to the n-th entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRef {
    pub name: String,
    pub pguid: String,
}

/// Which physical axis drives a control.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AxisBinding {
    /// Index into the device list; -1 = unbound.
    pub joy: i32,
    pub axle: i32,
    /// Pedal reads high at rest (MIN=1 MAX=-1 in the file).
    pub invert: bool,
    /// Response curve (brake only; 1 = linear).
    pub gamma: Option<f32>,
}

/// Which physical button triggers an action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ButtonBinding {
    /// Section name in controls.ini, e.g. `GEARUP`.
    pub action: String,
    pub label: String,
    pub joy: i32,
    pub button: i32,
}

/// Steering and force-feedback tuning. Field names mirror the file keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WheelTuning {
    /// Wheel rotation range in degrees (`[STEER] LOCK`).
    pub lock: u32,
    pub steer_gamma: f32,
    pub steer_filter: f32,
    pub speed_sensitivity: f32,
    pub ff_gain: f32,
    pub filter_ff: f32,
    pub min_ff: f32,
    pub center_boost_gain: f32,
    pub center_boost_range: f32,
    pub curbs: f32,
    pub road: f32,
    pub slips: f32,
    pub abs: f32,
    pub understeer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControlsConfig {
    /// `WHEEL`, `X360` (gamepad) or `KEYBOARD`.
    pub input_method: String,
    pub devices: Vec<DeviceRef>,
    pub steer: AxisBinding,
    pub throttle: AxisBinding,
    pub brakes: AxisBinding,
    pub clutch: AxisBinding,
    pub buttons: Vec<ButtonBinding>,
    pub tuning: WheelTuning,
}

/// Button actions exposed in the UI: (section in controls.ini, label).
pub const BUTTON_ACTIONS: &[(&str, &str)] = &[
    ("GEARUP", "Shift up"),
    ("GEARDN", "Shift down"),
    ("HANDBRAKE", "Handbrake"),
    ("STARTER", "Engine start"),
    ("ACTION_HEADLIGHTS", "Headlights"),
    ("ACTION_HEADLIGHTS_FLASH", "Flash lights"),
    ("ACTION_HORN", "Horn"),
    ("ACTION_CHANGE_CAMERA", "Change camera"),
    ("GLANCELEFT", "Look left"),
    ("GLANCERIGHT", "Look right"),
    ("GLANCEBACK", "Look back"),
    ("KERS", "KERS"),
    ("DRS", "DRS"),
    ("BALANCEUP", "Brake bias +"),
    ("BALANCEDN", "Brake bias -"),
    ("TCUP", "Traction control +"),
    ("TCDN", "Traction control -"),
    ("ABSUP", "ABS +"),
    ("ABSDN", "ABS -"),
    ("TURBOUP", "Turbo +"),
    ("TURBODN", "Turbo -"),
    ("ENGINE_BRAKE_UP", "Engine brake +"),
    ("ENGINE_BRAKE_DN", "Engine brake -"),
];

fn num<T: std::str::FromStr>(ini: &Ini, section: &str, key: &str, default: T) -> T {
    ini.get(section, key).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn read_axis(ini: &Ini, section: &str) -> AxisBinding {
    AxisBinding {
        joy: num(ini, section, "JOY", -1),
        axle: num(ini, section, "AXLE", -1),
        invert: num::<i32>(ini, section, "MIN", -1) > num::<i32>(ini, section, "MAX", 1),
        gamma: ini.get(section, "GAMMA").and_then(|v| v.parse().ok()),
    }
}

/// Parse a `controls.ini` text into the editable model (missing keys get defaults).
pub fn read_config(text: &str) -> ControlsConfig {
    let ini = Ini::parse(text);
    let mut devices = Vec::new();
    for i in 0..8 {
        match ini.get("CONTROLLERS", &format!("CON{i}")) {
            Some(name) => devices.push(DeviceRef { name: name.to_string(), pguid: ini.get("CONTROLLERS", &format!("PGUID{i}")).unwrap_or("").to_string() }),
            None => break,
        }
    }
    ControlsConfig {
        input_method: ini.get("HEADER", "INPUT_METHOD").unwrap_or("KEYBOARD").to_ascii_uppercase(),
        devices,
        steer: read_axis(&ini, "STEER"),
        throttle: read_axis(&ini, "THROTTLE"),
        brakes: read_axis(&ini, "BRAKES"),
        clutch: read_axis(&ini, "CLUTCH"),
        buttons: BUTTON_ACTIONS
            .iter()
            .map(|(section, label)| ButtonBinding {
                action: section.to_string(),
                label: label.to_string(),
                joy: num(&ini, section, "JOY", -1),
                button: num(&ini, section, "BUTTON", -1),
            })
            .collect(),
        tuning: WheelTuning {
            lock: num(&ini, "STEER", "LOCK", 900),
            steer_gamma: num(&ini, "STEER", "STEER_GAMMA", 1.0),
            steer_filter: num(&ini, "STEER", "STEER_FILTER", 0.0),
            speed_sensitivity: num(&ini, "STEER", "SPEED_SENSITIVITY", 0.0),
            ff_gain: num(&ini, "STEER", "FF_GAIN", 1.0),
            filter_ff: num(&ini, "STEER", "FILTER_FF", 0.0),
            min_ff: num(&ini, "FF_TWEAKS", "MIN_FF", 0.05),
            center_boost_gain: num(&ini, "FF_TWEAKS", "CENTER_BOOST_GAIN", 0.0),
            center_boost_range: num(&ini, "FF_TWEAKS", "CENTER_BOOST_RANGE", 0.1),
            curbs: num(&ini, "FF_ENHANCEMENT", "CURBS", 0.4),
            road: num(&ini, "FF_ENHANCEMENT", "ROAD", 0.5),
            slips: num(&ini, "FF_ENHANCEMENT", "SLIPS", 0.0),
            abs: num(&ini, "FF_ENHANCEMENT", "ABS", 0.21),
            understeer: num(&ini, "FF_ENHANCEMENT_2", "UNDERSTEER", 0.0),
        },
    }
}

fn set_axis(ini: &mut Ini, section: &str, a: &AxisBinding) {
    ini.set(section, "JOY", a.joy);
    ini.set(section, "AXLE", a.axle);
    if section != "STEER" {
        let (min, max) = if a.invert { (1, -1) } else { (-1, 1) };
        ini.set(section, "MIN", min);
        ini.set(section, "MAX", max);
    }
    if let Some(g) = a.gamma {
        ini.set(section, "GAMMA", g);
    }
}

/// Apply the model onto a controls.ini text, keeping every key we do not manage.
pub fn apply_config(base: &str, c: &ControlsConfig) -> String {
    let mut ini = Ini::parse(base);
    let method = match c.input_method.as_str() {
        "WHEEL" | "X360" | "KEYBOARD" => c.input_method.as_str(),
        _ => "KEYBOARD",
    };
    ini.set("HEADER", "INPUT_METHOD", method);

    // Rebuild the device list from scratch so stale entries cannot linger.
    for i in 0..8 {
        ini.remove_key("CONTROLLERS", &format!("CON{i}"));
        ini.remove_key("CONTROLLERS", &format!("PGUID{i}"));
    }
    for (i, d) in c.devices.iter().enumerate() {
        ini.set("CONTROLLERS", &format!("CON{i}"), &d.name);
        ini.set("CONTROLLERS", &format!("PGUID{i}"), &d.pguid);
    }

    set_axis(&mut ini, "STEER", &c.steer);
    set_axis(&mut ini, "THROTTLE", &c.throttle);
    set_axis(&mut ini, "BRAKES", &c.brakes);
    set_axis(&mut ini, "CLUTCH", &c.clutch);

    for b in &c.buttons {
        if BUTTON_ACTIONS.iter().any(|(s, _)| *s == b.action) {
            ini.set(&b.action, "JOY", b.joy);
            ini.set(&b.action, "BUTTON", b.button);
        }
    }

    let t = &c.tuning;
    ini.set("STEER", "SCALE", 1);
    ini.set("STEER", "LOCK", t.lock.clamp(90, 1440));
    ini.set("STEER", "STEER_GAMMA", t.steer_gamma);
    ini.set("STEER", "STEER_FILTER", t.steer_filter);
    ini.set("STEER", "SPEED_SENSITIVITY", t.speed_sensitivity);
    ini.set("STEER", "FF_GAIN", t.ff_gain);
    ini.set("STEER", "FILTER_FF", t.filter_ff);
    ini.set("FF_TWEAKS", "MIN_FF", t.min_ff);
    ini.set("FF_TWEAKS", "CENTER_BOOST_GAIN", t.center_boost_gain);
    ini.set("FF_TWEAKS", "CENTER_BOOST_RANGE", t.center_boost_range);
    ini.set("FF_ENHANCEMENT", "CURBS", t.curbs);
    ini.set("FF_ENHANCEMENT", "ROAD", t.road);
    ini.set("FF_ENHANCEMENT", "SLIPS", t.slips);
    ini.set("FF_ENHANCEMENT", "ABS", t.abs);
    ini.set("FF_ENHANCEMENT_2", "UNDERSTEER", t.understeer);
    ini.to_string_crlf()
}

/// Current user `controls.ini`, or the shipped default if the user has none yet.
pub fn load_text(cfg_dir: &Path, game_dir: &Path) -> String {
    fs::read(cfg_dir.join("controls.ini"))
        .or_else(|_| fs::read(game_dir.join("cfg/controls.ini")))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

/// Save a config to `<cfg_dir>/controls.ini` (with the one-time backup).
pub fn save_config(cfg_dir: &Path, game_dir: &Path, c: &ControlsConfig) -> io::Result<()> {
    fs::create_dir_all(cfg_dir)?;
    let target = cfg_dir.join("controls.ini");
    let base = load_text(cfg_dir, game_dir);
    if target.exists() {
        let backup = cfg_dir.join("controls.ini.launcher-backup");
        if !backup.exists() {
            fs::copy(&target, backup)?;
        }
    }
    fs::write(target, apply_config(&base, c))
}

/// The G923 preset as an editable config (pure data; same values as [`WheelProfile::g923`]).
pub fn g923_config(base: &str) -> ControlsConfig {
    let p = WheelProfile::g923();
    let mut c = read_config(&apply_wheel(base, &p));
    c.tuning.lock = p.lock_degrees;
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g923_guid() {
        assert_eq!(WheelProfile::g923().pguid(), "C266046D-0000-0000-0000-504944564944");
    }

    #[test]
    fn applies_bindings_and_keeps_other_keys() {
        let base = "[HEADER]\r\nINPUT_METHOD=KEYBOARD\r\n[BRAKES]\r\nJOY=-1\r\nAXLE=-1\r\nGAMMA=2.4\r\n[GEARUP] \r\nJOY=-1 \r\nBUTTON=-1\r\nKEY=0x20 ; SPACE\r\n";
        let out = apply_wheel(base, &WheelProfile::g923());
        let ini = Ini::parse(&out);
        assert_eq!(ini.get("HEADER", "INPUT_METHOD"), Some("WHEEL"));
        assert_eq!(ini.get("THROTTLE", "AXLE"), Some("2"));
        assert_eq!(ini.get("BRAKES", "AXLE"), Some("3"));
        assert_eq!(ini.get("BRAKES", "GAMMA"), Some("2.4"));
        assert_eq!(ini.get("GEARUP", "BUTTON"), Some("4"));
        assert_eq!(ini.get("GEARUP", "KEY"), Some("0x20"));
    }

    #[test]
    fn config_roundtrip() {
        let base = "[HEADER]\r\nINPUT_METHOD=KEYBOARD\r\n[GEARUP]\r\nJOY=-1\r\nBUTTON=-1\r\nKEY=0x20 ; SPACE\r\n";
        let cfg = g923_config(base);
        assert_eq!(cfg.input_method, "WHEEL");
        assert_eq!(cfg.devices[0].pguid, "C266046D-0000-0000-0000-504944564944");
        assert_eq!(cfg.throttle.axle, 2);
        assert!(cfg.throttle.invert);
        let gear = cfg.buttons.iter().find(|b| b.action == "GEARUP").unwrap();
        assert_eq!(gear.button, 4);
        // writing the model back and re-reading gives the same model
        let again = read_config(&apply_config(base, &cfg));
        assert_eq!(again, cfg);
        assert!(apply_config(base, &cfg).contains("KEY=0x20 ; SPACE"));
    }

    #[test]
    fn stale_devices_are_removed() {
        let mut cfg = g923_config("");
        let first = apply_config("", &cfg);
        cfg.devices.clear();
        let out = read_config(&apply_config(&first, &cfg));
        assert!(out.devices.is_empty());
    }
}
