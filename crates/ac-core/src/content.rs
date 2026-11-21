//! Scan `content/cars` and `content/tracks` for the UI metadata the game ships.
//! Everything is best-effort: a broken or missing JSON never hides the item,
//! we fall back to the folder name.

use crate::lenient_json;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn read_json(path: &Path) -> Option<Value> {
    lenient_json::parse(&fs::read(path).ok()?)
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn tags(v: &Value) -> Vec<String> {
    v.get("tags")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|t| t.trim_start_matches('#').to_string()).collect())
        .unwrap_or_default()
}

/// "ks_nordschleife" -> "Nordschleife", "bmw_m3_e30" -> "Bmw M3 E30".
fn prettify(id: &str) -> String {
    id.trim_start_matches("ks_")
        .split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sorted_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    v
}

fn existing(p: PathBuf) -> Option<String> {
    p.is_file().then(|| s(&p))
}

/// Folder names under `content/cars` / `content/tracks` (cheap; no metadata parsing).
pub fn car_ids(content_dir: &Path) -> Vec<String> {
    sorted_dirs(&content_dir.join("cars")).into_iter().map(|(id, _)| id).collect()
}
pub fn track_ids(content_dir: &Path) -> Vec<String> {
    sorted_dirs(&content_dir.join("tracks")).into_iter().map(|(id, _)| id).collect()
}

// ---------------------------------------------------------------- cars

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CarSpecs {
    pub bhp: Option<String>,
    pub torque: Option<String>,
    pub weight: Option<String>,
    pub topspeed: Option<String>,
    pub acceleration: Option<String>,
    pub pwratio: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Car {
    pub id: String,
    pub name: String,
    pub brand: Option<String>,
    pub class: Option<String>,
    pub tags: Vec<String>,
    pub specs: CarSpecs,
    pub description: Option<String>,
    /// Absolute path of `ui/badge.png`.
    pub badge: Option<String>,
    pub skin_count: usize,
    /// Preview of the first skin that has one, for the card thumbnail.
    pub thumbnail: Option<String>,
    /// False when the car had no/unreadable `ui_car.json` (name = folder name).
    pub has_metadata: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    /// Folder name; this is what `SKIN=` in race.ini expects.
    pub id: String,
    pub name: String,
    pub number: Option<String>,
    pub team: Option<String>,
    pub preview: Option<String>,
}

/// List every car in `content/cars`.
pub fn list_cars(content_dir: &Path) -> Vec<Car> {
    sorted_dirs(&content_dir.join("cars"))
        .into_iter()
        // A car folder without a `skins` dir cannot be driven (e.g. leftovers).
        .filter(|(_, p)| p.join("skins").is_dir())
        .map(|(id, path)| {
            let json = read_json(&path.join("ui/ui_car.json"));
            let skins = sorted_dirs(&path.join("skins"));
            let thumbnail = skins.iter().find_map(|(_, sp)| existing(sp.join("preview.jpg")));
            let specs = json.as_ref().and_then(|j| j.get("specs")).map(|sp| CarSpecs {
                bhp: str_field(sp, "bhp"),
                torque: str_field(sp, "torque"),
                weight: str_field(sp, "weight"),
                topspeed: str_field(sp, "topspeed"),
                acceleration: str_field(sp, "acceleration"),
                pwratio: str_field(sp, "pwratio"),
            });
            Car {
                name: json.as_ref().and_then(|j| str_field(j, "name")).unwrap_or_else(|| prettify(&id)),
                brand: json.as_ref().and_then(|j| str_field(j, "brand")),
                class: json.as_ref().and_then(|j| str_field(j, "class")),
                tags: json.as_ref().map(tags).unwrap_or_default(),
                specs: specs.unwrap_or_default(),
                description: json.as_ref().and_then(|j| str_field(j, "description")),
                badge: existing(path.join("ui/badge.png")),
                skin_count: skins.len(),
                thumbnail,
                has_metadata: json.is_some(),
                id,
            }
        })
        .collect()
}

/// List the skins of one car.
pub fn list_skins(content_dir: &Path, car_id: &str) -> Vec<Skin> {
    if car_id.contains(['/', '\\']) || car_id.contains("..") {
        return Vec::new(); // ids come from the UI; never allow path traversal
    }
    sorted_dirs(&content_dir.join("cars").join(car_id).join("skins"))
        .into_iter()
        .map(|(id, path)| {
            let json = read_json(&path.join("ui_skin.json"));
            Skin {
                name: json.as_ref().and_then(|j| str_field(j, "skinname")).unwrap_or_else(|| prettify(&id)),
                number: json.as_ref().and_then(|j| str_field(j, "number")),
                team: json.as_ref().and_then(|j| str_field(j, "team")),
                preview: existing(path.join("preview.jpg")),
                id,
            }
        })
        .collect()
}

// ------------------------------------------------------------- weather

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    /// Folder name; value for `[WEATHER] NAME=`.
    pub id: String,
    pub name: String,
    /// `TEMPERATURE_COEFF` (-1..1): how much the weather heats/cools the asphalt.
    pub temperature_coeff: f32,
}

/// List `content/weather/*` presets in folder order (1_heavy_fog ... 7_heavy_clouds).
pub fn list_weather(content_dir: &Path) -> Vec<Weather> {
    sorted_dirs(&content_dir.join("weather"))
        .into_iter()
        .filter_map(|(id, path)| {
            let ini = crate::ini::Ini::parse(&String::from_utf8_lossy(&fs::read(path.join("weather.ini")).ok()?));
            Some(Weather {
                name: ini.get("LAUNCHER", "NAME").map(String::from).unwrap_or_else(|| prettify(&id)),
                temperature_coeff: ini.get("LAUNCHER", "TEMPERATURE_COEFF").and_then(|v| v.parse().ok()).unwrap_or(0.0),
                id,
            })
        })
        .collect()
}

// -------------------------------------------------------------- tracks

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackLayout {
    /// Value for `CONFIG_TRACK=` ("" = the default layout).
    pub config: String,
    pub name: String,
    pub description: Option<String>,
    pub length_m: Option<u32>,
    pub pitboxes: Option<u32>,
    pub run: Option<String>,
    pub preview: Option<String>,
    pub outline: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub tags: Vec<String>,
    pub layouts: Vec<TrackLayout>,
}

fn num(v: &Value, key: &str) -> Option<u32> {
    match v.get(key)? {
        Value::String(t) => t.trim().parse().ok(),
        Value::Number(n) => n.as_u64().map(|n| n as u32),
        _ => None,
    }
}

fn layout_from(dir: &Path, config: &str, fallback_name: &str) -> Option<(TrackLayout, Value)> {
    let json = read_json(&dir.join("ui_track.json"));
    // A layout exists if it has a ui json or at least a preview image.
    if json.is_none() && !dir.join("preview.png").is_file() {
        return None;
    }
    let j = json.unwrap_or(Value::Null);
    Some((
        TrackLayout {
            config: config.to_string(),
            name: str_field(&j, "name").unwrap_or_else(|| fallback_name.to_string()),
            description: str_field(&j, "description"),
            length_m: num(&j, "length"),
            pitboxes: num(&j, "pitboxes"),
            run: str_field(&j, "run"),
            preview: existing(dir.join("preview.png")),
            outline: existing(dir.join("outline.png")),
        },
        j,
    ))
}

/// Look up a single layout without scanning every track (`config` "" = default layout).
pub fn track_layout(content_dir: &Path, track_id: &str, config: &str) -> Option<TrackLayout> {
    if [track_id, config].iter().any(|v| v.contains(['/', '\\']) || v.contains("..")) {
        return None;
    }
    let ui = content_dir.join("tracks").join(track_id).join("ui");
    let dir = if config.is_empty() { ui } else { ui.join(config) };
    layout_from(&dir, config, track_id).map(|(l, _)| l)
}

/// List every track with all its layouts.
///
/// Single-layout tracks keep their UI files in `ui/`; multi-layout tracks put
/// one subfolder per layout in `ui/<layout>/` and the folder name is the
/// `CONFIG_TRACK` value.
pub fn list_tracks(content_dir: &Path) -> Vec<Track> {
    sorted_dirs(&content_dir.join("tracks"))
        .into_iter()
        .filter_map(|(id, path)| {
            let ui = path.join("ui");
            let mut layouts = Vec::new();
            let mut meta = Value::Null;
            if let Some((l, j)) = layout_from(&ui, "", &prettify(&id)) {
                layouts.push(l);
                meta = j;
            }
            for (cfg, p) in sorted_dirs(&ui) {
                if let Some((l, j)) = layout_from(&p, &cfg, &prettify(&cfg)) {
                    if meta.is_null() {
                        meta = j;
                    }
                    layouts.push(l);
                }
            }
            if layouts.is_empty() {
                return None;
            }
            let name = if layouts.len() == 1 { layouts[0].name.clone() } else { prettify(&id) };
            Some(Track {
                name,
                country: str_field(&meta, "country"),
                city: str_field(&meta, "city"),
                tags: tags(&meta),
                layouts,
                id,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettify_ids() {
        assert_eq!(prettify("ks_nordschleife"), "Nordschleife");
        assert_eq!(prettify("bmw_m3_e30"), "Bmw M3 E30");
    }

    /// Runs against the real install when `AC_GAME_DIR` is set:
    /// `AC_GAME_DIR=/path/to/assettocorsa cargo test -p ac-core -- --nocapture`
    #[test]
    fn scans_real_install_if_available() {
        let Ok(game) = std::env::var("AC_GAME_DIR") else { return };
        let content = PathBuf::from(game).join("content");
        let cars = list_cars(&content);
        let tracks = list_tracks(&content);
        println!("{} cars, {} tracks", cars.len(), tracks.len());
        let no_meta: Vec<_> = cars.iter().filter(|c| !c.has_metadata).map(|c| &c.id).collect();
        println!("cars without usable ui_car.json: {no_meta:?}");
        for t in &tracks {
            println!("{} -> {:?}", t.id, t.layouts.iter().map(|l| (&l.config, &l.name)).collect::<Vec<_>>());
        }
        assert!(!cars.is_empty() && !tracks.is_empty());
    }
}
