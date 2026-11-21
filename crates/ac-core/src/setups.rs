//! Car setups. User setups: `<docs>/setups/<car>/<track or "generic">/<name>.ini`
//! (`last.ini`/`last.sp` are the game's autosave). The game also ships a few in
//! `<install>/system/setups/<car>/<track>.ini`. A setup file is INI sections of
//! `[NAME] VALUE=n` and ends with `[CAR] MODEL=<car id>`.

use crate::ini::Ini;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupFile {
    pub car: String,
    /// Track folder name, or "generic".
    pub scope: String,
    /// File name without `.ini`.
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: u64,
    /// Shipped with the game (read-only).
    pub builtin: bool,
}

fn modified(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn clean(s: &str) -> String {
    let c: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.') { c } else { '_' })
        .collect();
    c.trim().trim_start_matches('.').to_string()
}

fn dirs_in(p: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<_> = fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    v.sort();
    v
}

/// Setups for one car (or all when `car` is None): user files first, then shipped ones.
pub fn list(user_docs: &Path, game_dir: &Path, car: Option<&str>) -> Vec<SetupFile> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<SetupFile>, car: &str, scope: &str, path: PathBuf, builtin: bool| {
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("ini")) {
            return;
        }
        let Ok(meta) = fs::metadata(&path) else { return };
        out.push(SetupFile {
            car: car.into(),
            scope: scope.into(),
            name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            size: meta.len(),
            modified: modified(&meta),
            builtin,
        });
    };
    for (car_id, car_dir) in dirs_in(&user_docs.join("setups")) {
        if car.is_some_and(|c| c != car_id) {
            continue;
        }
        for (scope, scope_dir) in dirs_in(&car_dir) {
            for e in fs::read_dir(scope_dir).into_iter().flatten().flatten() {
                push(&mut out, &car_id, &scope, e.path(), false);
            }
        }
    }
    for (car_id, car_dir) in dirs_in(&game_dir.join("system/setups")) {
        if car.is_some_and(|c| c != car_id) {
            continue;
        }
        for e in fs::read_dir(car_dir).into_iter().flatten().flatten() {
            let scope = e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            push(&mut out, &car_id, &scope, e.path(), true);
        }
    }
    out.sort_by(|a, b| a.builtin.cmp(&b.builtin).then(b.modified.cmp(&a.modified)));
    out
}

/// True if `path` is inside `root` (after resolving symlinks and `..`).
fn inside(root: &Path, path: &Path) -> bool {
    match (root.canonicalize(), path.canonicalize()) {
        (Ok(r), Ok(p)) => p.starts_with(r),
        _ => false,
    }
}

/// Add a setup file. The car comes from the file's `[CAR] MODEL=` or `car_hint`.
pub fn import(user_docs: &Path, src: &Path, car_hint: Option<&str>, scope: &str) -> io::Result<SetupFile> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
    let text = String::from_utf8_lossy(&fs::read(src)?).into_owned();
    let ini = Ini::parse(&text);
    if ini.sections().is_empty() || ini.get("FUEL", "VALUE").is_none() {
        return Err(bad("this does not look like an Assetto Corsa setup file"));
    }
    let car = ini.get("CAR", "MODEL").map(str::to_string).or_else(|| car_hint.map(str::to_string)).ok_or_else(|| bad("cannot tell which car this setup is for"))?;
    let (car, scope) = (clean(&car), clean(if scope.is_empty() { "generic" } else { scope }));
    if car.is_empty() || scope.is_empty() {
        return Err(bad("invalid car or track name"));
    }
    let dir = user_docs.join("setups").join(&car).join(&scope);
    fs::create_dir_all(&dir)?;
    let stem = clean(&src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "setup".into()));
    let stem = if stem.is_empty() { "setup".into() } else { stem };
    // never overwrite: name.ini, name (2).ini, ...
    let mut dest = dir.join(format!("{stem}.ini"));
    let mut n = 2;
    while dest.exists() {
        dest = dir.join(format!("{stem} ({n}).ini"));
        n += 1;
    }
    fs::write(&dest, text)?;
    let meta = fs::metadata(&dest)?;
    Ok(SetupFile {
        car,
        scope,
        name: dest.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        path: dest.to_string_lossy().into_owned(),
        size: meta.len(),
        modified: modified(&meta),
        builtin: false,
    })
}

/// Copy a setup (user or shipped) to a chosen file.
pub fn export(user_docs: &Path, game_dir: &Path, path: &Path, dest: &Path) -> io::Result<()> {
    if !inside(&user_docs.join("setups"), path) && !inside(&game_dir.join("system/setups"), path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "not a setup file"));
    }
    fs::copy(path, dest).map(|_| ())
}

/// Copy a shipped setup into the user's setups so the game can use/edit it.
pub fn adopt_builtin(user_docs: &Path, game_dir: &Path, path: &Path) -> io::Result<SetupFile> {
    if !inside(&game_dir.join("system/setups"), path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "not a shipped setup"));
    }
    let car = path.parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let scope = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let text = fs::read_to_string(path)?;
    // Shipped files have no [CAR] section; we know the car from the folder.
    let tmp = user_docs.join("setups").join(".adopt.tmp");
    fs::create_dir_all(tmp.parent().unwrap())?;
    fs::write(&tmp, text)?;
    let res = import(user_docs, &tmp, Some(&car), &scope);
    let _ = fs::remove_file(&tmp);
    res
}

/// Delete a user setup (never shipped ones).
pub fn delete(user_docs: &Path, path: &Path) -> io::Result<()> {
    if !inside(&user_docs.join("setups"), path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "only your own setups can be deleted"));
    }
    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ac-setups-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn import_list_delete_roundtrip() {
        let docs = tmpdir("a");
        let src = docs.join("my setup.ini");
        fs::write(&src, "[FUEL]\r\nVALUE=30\r\n[CAR]\r\nMODEL=lotus_elise_sc\r\n").unwrap();
        let a = import(&docs, &src, None, "magione").unwrap();
        let b = import(&docs, &src, None, "magione").unwrap(); // does not overwrite
        assert_ne!(a.path, b.path);
        assert_eq!(a.car, "lotus_elise_sc");
        let all = list(&docs, Path::new("/nonexistent"), Some("lotus_elise_sc"));
        assert_eq!(all.len(), 2);
        delete(&docs, Path::new(&a.path)).unwrap();
        assert!(delete(&docs, &src).is_err()); // outside setups/
        fs::remove_dir_all(&docs).unwrap();
    }

    #[test]
    fn rejects_non_setup_and_unknown_car() {
        let docs = tmpdir("b");
        let junk = docs.join("x.ini");
        fs::write(&junk, "[SOMETHING]\r\nA=1\r\n").unwrap();
        assert!(import(&docs, &junk, Some("c"), "generic").is_err());
        let nocar = docs.join("y.ini");
        fs::write(&nocar, "[FUEL]\r\nVALUE=1\r\n").unwrap();
        assert!(import(&docs, &nocar, None, "generic").is_err());
        assert!(import(&docs, &nocar, Some("c"), "generic").is_ok());
        fs::remove_dir_all(&docs).unwrap();
    }
}
