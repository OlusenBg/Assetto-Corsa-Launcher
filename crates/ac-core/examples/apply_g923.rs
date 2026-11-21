//! Write G923 bindings and a fullscreen video mode into the game's cfg. Close the game first.
//!   cargo run -p ac-core --example apply_g923
use ac_core::{controls, paths, video};

fn main() {
    let r = paths::resolve(&paths::Settings::load());
    let (cfg, game) = (r.cfg_dir().expect("no prefix"), r.game_dir.expect("no game dir"));
    controls::write_wheel(&cfg, &game, &controls::WheelProfile::g923()).unwrap();
    println!("wrote controls.ini");
    let mode = video::detect_primary_mode().unwrap_or(video::VideoMode { width: 1920, height: 1080, refresh: 60, fullscreen: true });
    // Windowed for now: fullscreen under XWayland was stretched and unusable.
    let mode = video::VideoMode { width: 1600, height: 900, fullscreen: false, ..mode };
    video::write_mode(&cfg, &game, &mode).unwrap();
    println!("wrote video.ini: {mode:?}");
}
