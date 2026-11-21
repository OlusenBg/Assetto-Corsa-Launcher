//! Replays: the game records the last session to `replay/temp/AC_<ddmmyy>-<hhmmss>_<O>_<car>_<track>[_<layout>]`
//! (no extension). Replays saved from the game's replay screen are `replay/<name>.acreplay`.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Replay {
    /// File name (as in the replay folder).
    pub name: String,
    /// True for the automatic recording in `replay/temp`.
    pub temp: bool,
    pub size: u64,
    /// Unix seconds.
    pub modified: u64,
    pub car: Option<String>,
    pub track: Option<String>,
    pub layout: Option<String>,
}

/// Names must be a single path component.
fn safe_name(name: &str) -> io::Result<()> {
    if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." || name.starts_with('.') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid replay name {name:?}")));
    }
    Ok(())
}

/// Pull car / track / layout out of an auto-generated name by matching installed ids.
pub fn parse_name(name: &str, cars: &[String], tracks: &[String]) -> (Option<String>, Option<String>, Option<String>) {
    let stem = name.strip_suffix(".acreplay").unwrap_or(name);
    let mut it = stem.splitn(4, '_');
    let (Some("AC"), Some(_stamp), Some(_kind), Some(rest)) = (it.next(), it.next(), it.next(), it.next()) else {
        return (None, None, None);
    };
    let pick = |ids: &[String], text: &str| -> Option<String> {
        ids.iter()
            .filter(|id| text == id.as_str() || text.starts_with(&format!("{id}_")))
            .max_by_key(|id| id.len())
            .cloned()
    };
    let Some(car) = pick(cars, rest) else { return (None, None, None) };
    let after = rest[car.len()..].trim_start_matches('_');
    let track = pick(tracks, after);
    let layout = track.as_ref().and_then(|t| {
        let l = after[t.len()..].trim_start_matches('_');
        (!l.is_empty()).then(|| l.to_string())
    });
    (Some(car), track, layout)
}

fn scan(dir: &Path, temp: bool, cars: &[String], tracks: &[String], out: &mut Vec<Replay>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let (car, track, layout) = parse_name(&name, cars, tracks);
        out.push(Replay {
            temp,
            size: meta.len(),
            modified: meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
            name,
            car,
            track,
            layout,
        });
    }
}

/// All replays, newest first.
pub fn list(user_docs: &Path, cars: &[String], tracks: &[String]) -> Vec<Replay> {
    let mut out = Vec::new();
    scan(&user_docs.join("replay"), false, cars, tracks, &mut out);
    scan(&user_docs.join("replay/temp"), true, cars, tracks, &mut out);
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn path_of(user_docs: &Path, name: &str, temp: bool) -> io::Result<PathBuf> {
    safe_name(name)?;
    Ok(user_docs.join("replay").join(if temp { "temp" } else { "" }).join(name))
}

/// Keep a temp recording: copy it to `replay/<name>.acreplay`. Returns the new file name.
pub fn keep(user_docs: &Path, name: &str) -> io::Result<String> {
    let src = path_of(user_docs, name, true)?;
    let new_name = if name.ends_with(".acreplay") { name.to_string() } else { format!("{name}.acreplay") };
    let dest = user_docs.join("replay").join(&new_name);
    if !dest.exists() {
        fs::copy(src, dest)?;
    }
    Ok(new_name)
}

pub fn delete(user_docs: &Path, name: &str, temp: bool) -> io::Result<()> {
    fs::remove_file(path_of(user_docs, name, temp)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_auto_names() {
        let cars = ids(&["ks_ferrari_sf15t", "ks_ferrari", "lotus_elise_sc"]);
        let tracks = ids(&["ks_silverstone", "magione", "ks_nordschleife"]);
        let r = parse_name("AC_211126-113243_O_ks_ferrari_sf15t_ks_silverstone_gp", &cars, &tracks);
        assert_eq!(r, (Some("ks_ferrari_sf15t".into()), Some("ks_silverstone".into()), Some("gp".into())));
        let r = parse_name("AC_201126-232405_O_lotus_elise_sc_magione_", &cars, &tracks);
        assert_eq!(r, (Some("lotus_elise_sc".into()), Some("magione".into()), None));
        assert_eq!(parse_name("my_replay.acreplay", &cars, &tracks), (None, None, None));
    }

    #[test]
    fn rejects_path_tricks() {
        assert!(path_of(Path::new("/x"), "../a", true).is_err());
        assert!(path_of(Path::new("/x"), "a/b", false).is_err());
    }
}
