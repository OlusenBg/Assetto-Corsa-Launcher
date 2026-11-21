//! Headless check: detect paths, print the race.ini and Proton command that
//! would be used. Pass `--launch` to really write race.ini and start the game.
//!   cargo run -p ac-core --example dry_run [-- --launch]
use ac_core::race::*;
use ac_core::{launch, paths};

fn main() {
    let settings = paths::Settings::load();
    let r = paths::resolve(&settings);
    println!("issues: {:?}\n", r.issues);
    let req = RaceRequest {
        car: "ks_mazda_mx5_cup".into(),
        skin: "01_cup_07".into(),
        track: "magione".into(),
        track_config: String::new(),
        driver_name: String::new(),
        sessions: vec![
            SessionSpec { kind: SessionKind::Practice, minutes: 0, laps: 0 },
            SessionSpec { kind: SessionKind::Race, minutes: 0, laps: 3 },
        ],
        weather: WeatherSpec { id: "3_clear".into(), hour: 10.0, ambient: 22, time_mult: 1, temperature_coeff: 1.0 },
        ai: AiSpec { count: 3, strength: 90, aggression: 30, cars: vec![] },
        penalties: true,
        start_position: 4,
    };
    let game = r.game_dir.clone().expect("no game dir");
    let roster = ai_roster(&game, &req);
    println!("roster: {roster:#?}");
    println!("--- race.ini ---\n{}", build(None, &req, &roster).unwrap());
    match launch::build_command(&settings, &r) {
        Ok(c) => println!("--- command ---\n{}", launch::describe(&c)),
        Err(e) => println!("cannot build command: {e}"),
    }
    if std::env::args().any(|a| a == "--launch") {
        write(&r.cfg_dir().unwrap(), &game, &req).unwrap();
        let mut child = launch::spawn(&settings, &r).unwrap();
        println!("started pid {}; log: {:?}", child.id(), launch::launch_log_path());
        println!("exit: {:?}", child.wait());
    }
}
