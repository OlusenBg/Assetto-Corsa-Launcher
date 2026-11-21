//! Install cars, tracks and skins from an archive (zip / rar / 7z / tar...).
//!
//! Two steps so the user can confirm: [`inspect`] unpacks to a temp folder and
//! works out what is inside, [`install`] copies it into `content/`.
//! Detection is by folder structure (observed in the shipped content):
//! * car   = folder with `ui/ui_car.json` (or `data.acd` + `skins/`)
//! * track = folder with `ui/ui_track.json` or `ui/<layout>/ui_track.json`
//! * skin  = folder with `ui_skin.json`, or a `preview.jpg` inside a `skins/` folder;
//!   its car is the folder two levels up (`<car>/skins/<skin>`) when present.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModKind {
    Car,
    Track,
    Skin,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModItem {
    pub kind: ModKind,
    /// Folder name that becomes the content id.
    pub id: String,
    /// For skins: the car it belongs to, when the archive says so.
    pub car: Option<String>,
    /// True when the target folder already exists.
    pub exists: bool,
    /// Skin whose car is unknown: the UI must supply one.
    pub needs_car: bool,
    #[serde(skip)]
    pub source: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPlan {
    pub archive: String,
    pub items: Vec<ModItem>,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub temp: PathBuf,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('.') && !id.contains(['/', '\\', '\0']) && id != ".."
}

fn is_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn subdirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| is_dir(p)).collect();
    v.sort();
    v
}

fn is_car(d: &Path) -> bool {
    d.join("ui/ui_car.json").is_file() || (d.join("data.acd").is_file() && d.join("skins").is_dir())
}

fn is_track(d: &Path) -> bool {
    d.join("ui/ui_track.json").is_file() || subdirs(&d.join("ui")).iter().any(|l| l.join("ui_track.json").is_file())
}

fn dir_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn is_skin(d: &Path) -> bool {
    d.join("ui_skin.json").is_file()
        || (d.parent().is_some_and(|p| dir_name(p).eq_ignore_ascii_case("skins")) && d.join("preview.jpg").is_file())
}

/// Walk an unpacked archive and collect installable items.
fn scan(dir: &Path, depth: u32, content: &Path, out: &mut Vec<ModItem>) {
    if depth > 8 {
        return;
    }
    let id = dir_name(dir);
    if depth > 0 && valid_id(&id) {
        if is_car(dir) {
            out.push(ModItem { kind: ModKind::Car, exists: content.join("cars").join(&id).exists(), car: None, needs_car: false, source: dir.into(), id });
            return;
        }
        if is_track(dir) {
            out.push(ModItem { kind: ModKind::Track, exists: content.join("tracks").join(&id).exists(), car: None, needs_car: false, source: dir.into(), id });
            return;
        }
        if is_skin(dir) {
            let parent = dir.parent();
            let car = parent
                .filter(|p| dir_name(p).eq_ignore_ascii_case("skins"))
                .and_then(|p| p.parent())
                .map(dir_name)
                .filter(|c| valid_id(c));
            let exists = car.as_ref().is_some_and(|c| content.join("cars").join(c).join("skins").join(&id).exists());
            out.push(ModItem { kind: ModKind::Skin, needs_car: car.is_none(), car, exists, source: dir.into(), id });
            return;
        }
    }
    for sub in subdirs(dir) {
        scan(&sub, depth + 1, content, out);
    }
}

/// Unpack with bsdtar (libarchive: zip, rar, 7z, tar.*) or, failing that, 7z.
fn extract(archive: &Path, dest: &Path) -> io::Result<()> {
    let tried = Command::new("bsdtar").arg("-xf").arg(archive).arg("-C").arg(dest).output();
    if tried.as_ref().is_ok_and(|o| o.status.success()) {
        return Ok(());
    }
    let seven = Command::new("7z").arg("x").arg("-y").arg(format!("-o{}", dest.display())).arg(archive).output();
    match seven {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(io::Error::other(format!("could not unpack the archive: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("unknown error")))),
        Err(_) => Err(io::Error::other("no archive tool found: install bsdtar (libarchive) or 7z")),
    }
}

/// Unpack `archive` into a fresh temp folder and report what it contains.
pub fn inspect(archive: &Path, content_dir: &Path) -> io::Result<ModPlan> {
    if !archive.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "archive not found"));
    }
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("ac-linux-launcher");
    let temp = base.join(format!("extract-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)));
    fs::create_dir_all(&temp)?;
    if let Err(e) = extract(archive, &temp) {
        let _ = fs::remove_dir_all(&temp);
        return Err(e);
    }
    let mut items = Vec::new();
    scan(&temp, 0, content_dir, &mut items);
    let mut warnings = Vec::new();
    if items.is_empty() {
        warnings.push("No car, track or skin found. Only content folders (with ui/ui_car.json, ui/ui_track.json or a skin) can be installed here.".into());
    }
    for it in &items {
        if let (ModKind::Skin, Some(car)) = (it.kind, &it.car) {
            let in_plan = items.iter().any(|o| o.kind == ModKind::Car && &o.id == car);
            if !in_plan && !content_dir.join("cars").join(car).is_dir() {
                warnings.push(format!("Skin \"{}\" is for car \"{car}\", which is not installed.", it.id));
            }
        }
    }
    Ok(ModPlan { archive: archive.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), items, warnings, temp })
}

/// Recursively copy `src` into `dst`, overwriting files and skipping symlinks.
fn copy_merge(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let ty = e.file_type()?;
        let to = dst.join(e.file_name());
        if ty.is_symlink() {
            continue;
        } else if ty.is_dir() {
            copy_merge(&e.path(), &to)?;
        } else {
            fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

/// Copy the items into `content/`. `overwrite` allows replacing existing folders
/// (cars/tracks are merged file-by-file so other installed skins are kept).
/// `skin_car` supplies the car for skins that did not say. Returns one line per item.
pub fn install(plan: &ModPlan, content_dir: &Path, overwrite: bool, skin_car: Option<&str>) -> io::Result<Vec<String>> {
    let mut report = Vec::new();
    // Cars and tracks first: a skin copied earlier would create its car's folder and make the car look installed.
    let mut ordered: Vec<&ModItem> = plan.items.iter().collect();
    ordered.sort_by_key(|i| i.kind == ModKind::Skin);
    for it in ordered {
        let dest = match it.kind {
            ModKind::Car => content_dir.join("cars").join(&it.id),
            ModKind::Track => content_dir.join("tracks").join(&it.id),
            ModKind::Skin => {
                let car = it.car.as_deref().or(skin_car).filter(|c| valid_id(c));
                let Some(car) = car else {
                    report.push(format!("Skipped skin {}: choose which car it is for", it.id));
                    continue;
                };
                if !content_dir.join("cars").join(car).is_dir() && !plan.items.iter().any(|o| o.kind == ModKind::Car && o.id == car) {
                    report.push(format!("Skipped skin {}: car {car} is not installed", it.id));
                    continue;
                }
                content_dir.join("cars").join(car).join("skins").join(&it.id)
            }
        };
        if dest.exists() && !overwrite {
            report.push(format!("Skipped {} (already installed; tick overwrite to replace)", it.id));
            continue;
        }
        copy_merge(&it.source, &dest)?;
        report.push(format!("Installed {:?} {}", it.kind, it.id).replace("Car", "car").replace("Track", "track").replace("Skin", "skin"));
    }
    Ok(report)
}

/// Remove the temp folder of a plan.
pub fn discard(plan: &ModPlan) {
    if plan.temp.starts_with(dirs::cache_dir().unwrap_or_else(std::env::temp_dir)) || plan.temp.starts_with(std::env::temp_dir()) {
        let _ = fs::remove_dir_all(&plan.temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ac-mods-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }
    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "x").unwrap();
    }

    #[test]
    fn detects_car_track_and_loose_skin() {
        let root = tmp("scan");
        touch(&root.join("pack/content/cars/my_car/ui/ui_car.json"));
        touch(&root.join("pack/content/cars/my_car/skins/red/preview.jpg")); // inside the car: not separate
        touch(&root.join("pack/content/tracks/my_track/ui/gp/ui_track.json"));
        touch(&root.join("pack/content/cars/other_car/skins/blue/preview.jpg"));
        touch(&root.join("loose_skin/ui_skin.json"));
        let content = tmp("content");
        let mut items = Vec::new();
        scan(&root, 0, &content, &mut items);
        let desc: Vec<_> = items.iter().map(|i| (i.kind, i.id.as_str(), i.car.as_deref(), i.needs_car)).collect();
        assert!(desc.contains(&(ModKind::Car, "my_car", None, false)));
        assert!(desc.contains(&(ModKind::Track, "my_track", None, false)));
        assert!(desc.contains(&(ModKind::Skin, "blue", Some("other_car"), false)));
        assert!(desc.contains(&(ModKind::Skin, "loose_skin", None, true)));
        assert!(!desc.iter().any(|d| d.1 == "red")); // belongs to my_car
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&content).unwrap();
    }

    #[test]
    fn install_respects_overwrite_and_missing_car() {
        let root = tmp("inst-src");
        touch(&root.join("my_car/ui/ui_car.json"));
        touch(&root.join("my_car/skins/red/preview.jpg"));
        touch(&root.join("loose/ui_skin.json"));
        let content = tmp("inst-dst");
        let mut items = Vec::new();
        scan(&root, 0, &content, &mut items);
        let plan = ModPlan { archive: "t.zip".into(), items, warnings: vec![], temp: PathBuf::from("/nonexistent") };
        let r = install(&plan, &content, false, Some("my_car")).unwrap();
        assert!(content.join("cars/my_car/ui/ui_car.json").is_file());
        assert!(content.join("cars/my_car/skins/loose/ui_skin.json").is_file());
        assert!(r.iter().all(|l| !l.starts_with("Skipped")));
        // second time: everything exists
        let r2 = install(&plan, &content, false, Some("my_car")).unwrap();
        assert!(r2.iter().all(|l| l.starts_with("Skipped")));
        // loose skin without a car is skipped
        let r3 = install(&plan, &content, true, None).unwrap();
        assert!(r3.iter().any(|l| l.contains("choose which car")));
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&content).unwrap();
    }

    #[test]
    fn unpacks_a_real_zip_if_tools_exist() {
        if Command::new("bsdtar").arg("--version").output().is_err() {
            return;
        }
        let src = tmp("zip-src");
        touch(&src.join("zz_car/ui/ui_car.json"));
        let zip = src.join("pack.zip");
        let ok = Command::new("bsdtar").args(["-a", "-cf"]).arg(&zip).arg("-C").arg(&src).arg("zz_car").status().unwrap().success();
        assert!(ok);
        let content = tmp("zip-content");
        let plan = inspect(&zip, &content).unwrap();
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].id, "zz_car");
        install(&plan, &content, false, None).unwrap();
        assert!(content.join("cars/zz_car/ui/ui_car.json").is_file());
        discard(&plan);
        fs::remove_dir_all(&src).unwrap();
        fs::remove_dir_all(&content).unwrap();
    }
}
