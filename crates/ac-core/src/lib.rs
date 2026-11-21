//! `ac-core`: everything the launcher does that is not UI.
//!
//! * [`paths`]   – settings + auto-detection of game, prefix and Proton
//! * [`content`] – installed cars/tracks/skins with previews
//! * [`ini`]     – lossless INI editing
//! * [`race`]    – writing `race.ini`
//! * [`launch`]  – starting `acs.exe` via Proton

pub mod assists;
pub mod audio;
pub mod content;
pub mod controls;
pub mod ini;
pub mod input;
pub mod launch;
pub mod lenient_json;
pub mod mods;
pub mod paths;
pub mod race;
pub mod replays;
pub mod setups;
pub mod video;
