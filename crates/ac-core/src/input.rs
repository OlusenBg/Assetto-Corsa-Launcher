//! Input device discovery and live monitoring via evdev (`/dev/input/event*`).
//!
//! The game itself sees devices through Wine's DirectInput. To keep the
//! launcher's numbering consistent with that, indices follow what we measured
//! on real hardware (see docs/ac-formats.md): **axes** are numbered in the
//! order the device reports them (ABS codes below 0x10, hats excluded), and
//! **buttons** in the order of their evdev key codes (>= 0x100).
//!
//! **Hats** (D-pads, ABS_HAT0X/Y = 0x10/0x11, next hat 0x12/0x13 ...) are exposed
//! as virtual buttons numbered from [`HAT_BASE`]: `128 + 4*hat + direction`
//! with direction 0 = up, 1 = right, 2 = down, 3 = left (DirectInput POV
//! encoding as used by AC; to be confirmed against a binding made in-game).

use evdev::{AbsoluteAxisCode, Device, EventType};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// First virtual button index used for hat directions.
pub const HAT_BASE: u32 = 128;

/// Hat (D-pad) binding is OFF: binding hats as buttons 128+ made the game see
/// "down" as permanently pressed, so that encoding is wrong. Re-enable only
/// once the real encoding is confirmed from a binding made in the game's own menu.
pub const HAT_BINDING_ENABLED: bool = false;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisInfo {
    /// Index the game uses (`AXLE=`).
    pub index: u32,
    /// Raw evdev code (0 = X, 1 = Y, 2 = Z, 3 = Rx, 4 = Ry, 5 = Rz ...).
    pub code: u16,
    pub name: String,
    pub min: i32,
    pub max: i32,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonInfo {
    /// Index the game uses (`BUTTON=`).
    pub index: u32,
    pub code: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    /// `/dev/input/eventN`
    pub path: String,
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    /// Windows-style product GUID used in `controls.ini` (`PGUID0=`).
    pub pguid: String,
    pub axes: Vec<AxisInfo>,
    pub buttons: Vec<ButtonInfo>,
    /// Number of hat switches (D-pads); each adds 4 virtual buttons from [`HAT_BASE`].
    pub hats: u32,
}

/// `<PID><VID>-0000-0000-0000-504944564944`, e.g. G923 = `C266046D-...`.
pub fn pguid(vendor: u16, product: u16) -> String {
    format!("{product:04X}{vendor:04X}-0000-0000-0000-504944564944")
}

fn axis_name(code: u16) -> String {
    match code {
        0 => "X".into(),
        1 => "Y".into(),
        2 => "Z".into(),
        3 => "Rx".into(),
        4 => "Ry".into(),
        5 => "Rz".into(),
        6 => "Throttle".into(),
        7 => "Rudder".into(),
        8 => "Wheel".into(),
        9 => "Gas".into(),
        10 => "Brake".into(),
        c => format!("Axis {c}"),
    }
}

fn describe(path: PathBuf, dev: &Device) -> Option<InputDevice> {
    let axes_set = dev.supported_absolute_axes()?;
    let keys = dev.supported_keys();
    // Joystick-like = has a primary axis AND buttons from the joystick/gamepad range.
    let has_axis = axes_set.contains(AbsoluteAxisCode::ABS_X);
    let button_codes: Vec<u16> = keys
        .map(|k| k.iter().map(|c| c.0).filter(|c| (0x100..0x300).contains(c)).collect())
        .unwrap_or_default();
    // 0x120.. = joystick buttons, 0x130.. = gamepad; exclude mice (BTN_LEFT 0x110) and touchpads.
    let joystick_like = button_codes.iter().any(|c| (0x120..0x150).contains(c));
    if !has_axis || !joystick_like || axes_set.contains(AbsoluteAxisCode::ABS_MT_SLOT) {
        return None;
    }
    let mut axes: Vec<AxisInfo> = dev
        .get_absinfo()
        .ok()?
        .filter(|(c, _)| c.0 < 0x10)
        .map(|(c, i)| AxisInfo { index: 0, code: c.0, name: axis_name(c.0), min: i.minimum(), max: i.maximum(), value: i.value() })
        .collect();
    axes.sort_by_key(|a| a.code);
    for (i, a) in axes.iter_mut().enumerate() {
        a.index = i as u32;
    }
    let mut codes = button_codes;
    codes.sort_unstable();
    let buttons = codes.into_iter().enumerate().map(|(i, code)| ButtonInfo { index: i as u32, code }).collect();
    let id = dev.input_id();
    Some(InputDevice {
        path: path.to_string_lossy().into_owned(),
        name: dev.name().unwrap_or("Unknown device").to_string(),
        vendor: id.vendor(),
        product: id.product(),
        pguid: pguid(id.vendor(), id.product()),
        axes,
        buttons,
        hats: (0..4u16).filter(|h| axes_set.contains(AbsoluteAxisCode(0x10 + 2 * h)) || axes_set.contains(AbsoluteAxisCode(0x11 + 2 * h))).count() as u32,
    })
}

/// All joystick-like devices we can open. Devices we lack permission for are skipped;
/// the second value counts them so the UI can explain an empty list.
pub fn list_devices() -> (Vec<InputDevice>, usize) {
    let mut out = Vec::new();
    for (path, dev) in evdev::enumerate() {
        if let Some(d) = describe(path, &dev) {
            out.push(d);
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    let unreadable = std::fs::read_dir("/dev/input")
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("event") && std::fs::File::open(e.path()).is_err())
                .count()
        })
        .unwrap_or(0);
    (out, unreadable)
}

/// Live state of one device, sent to the UI ~30 times a second while it changes.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub path: String,
    /// axis index -> raw value
    pub axes: BTreeMap<u32, i32>,
    /// pressed button indices
    pub buttons: Vec<u32>,
}

/// Stops the monitor threads when dropped or when [`Monitor::stop`] is called.
pub struct Monitor {
    stop: Arc<AtomicBool>,
}

impl Monitor {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start one reader thread per device. `on_snapshot` is called from those threads.
pub fn monitor<F>(devices: &[InputDevice], on_snapshot: F) -> Monitor
where
    F: Fn(Snapshot) + Send + Sync + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let cb = Arc::new(on_snapshot);
    for d in devices {
        let (stop, cb, info) = (stop.clone(), cb.clone(), d.clone());
        std::thread::spawn(move || {
            let Ok(mut dev) = Device::open(&info.path) else { return };
            if dev.set_nonblocking(true).is_err() {
                return;
            }
            let axis_index: BTreeMap<u16, u32> = info.axes.iter().map(|a| (a.code, a.index)).collect();
            let button_index: BTreeMap<u16, u32> = info.buttons.iter().map(|b| (b.code, b.index)).collect();
            let mut snap = Snapshot { path: info.path.clone(), ..Default::default() };
            for a in &info.axes {
                snap.axes.insert(a.index, a.value);
            }
            let mut hats = [(0i32, 0i32); 4];
            let mut last_sent = Instant::now() - Duration::from_secs(1);
            let mut dirty = true;
            while !stop.load(Ordering::Relaxed) {
                if let Ok(events) = dev.fetch_events() {
                    for ev in events {
                        match ev.event_type() {
                            EventType::ABSOLUTE if HAT_BINDING_ENABLED && (0x10..0x18).contains(&ev.code()) => {
                                let hat = ((ev.code() - 0x10) / 2) as usize;
                                let is_y = ev.code() % 2 == 1;
                                if is_y { hats[hat].1 = ev.value() } else { hats[hat].0 = ev.value() }
                                let base = HAT_BASE + 4 * hat as u32;
                                snap.buttons.retain(|b| !(base..base + 4).contains(b));
                                let (x, y) = hats[hat];
                                if y < 0 { snap.buttons.push(base) }
                                if x > 0 { snap.buttons.push(base + 1) }
                                if y > 0 { snap.buttons.push(base + 2) }
                                if x < 0 { snap.buttons.push(base + 3) }
                                dirty = true;
                            }
                            EventType::ABSOLUTE => {
                                if let Some(&i) = axis_index.get(&ev.code()) {
                                    snap.axes.insert(i, ev.value());
                                    dirty = true;
                                }
                            }
                            EventType::KEY => {
                                if let Some(&i) = button_index.get(&ev.code()) {
                                    snap.buttons.retain(|b| *b != i);
                                    if ev.value() != 0 {
                                        snap.buttons.push(i);
                                    }
                                    dirty = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if dirty && last_sent.elapsed() >= Duration::from_millis(33) {
                    cb(snap.clone());
                    last_sent = Instant::now();
                    dirty = false;
                }
                std::thread::sleep(Duration::from_millis(4));
            }
        });
    }
    Monitor { stop }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_format() {
        assert_eq!(pguid(0x046d, 0xc266), "C266046D-0000-0000-0000-504944564944");
        assert_eq!(pguid(0x046d, 0xc299), "C299046D-0000-0000-0000-504944564944");
    }

    /// Runs on machines with a joystick; prints what it finds.
    #[test]
    fn lists_devices() {
        let (devs, unreadable) = list_devices();
        for d in &devs {
            println!("{} {} hats={} axes={:?} buttons={}", d.path, d.name, d.hats, d.axes.iter().map(|a| (a.index, a.name.clone())).collect::<Vec<_>>(), d.buttons.len());
        }
        println!("{} devices, {} unreadable event nodes", devs.len(), unreadable);
    }
}
