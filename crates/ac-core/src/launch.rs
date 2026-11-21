//! Start `acs.exe` through Proton, the way Steam would, but skipping the
//! stock launcher (`AssettoCorsa.exe`) that crashes on its CEF3 browser.

use crate::paths::{Resolved, Settings, AC_APP_ID};
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Where the combined stdout/stderr of the last launch goes.
pub fn launch_log_path() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|d| d.join("ac-linux-launcher").join("last-launch.log"))
}

/// Parse `KEY=VALUE` lines (blank lines and `#` comments ignored).
fn parse_env(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// gamescope arguments: output = monitor size/refresh, inner = the game's render size.
/// `extra` (user text) replaces the automatic sizing when non-empty.
pub fn gamescope_words(inner: (u32, u32), output: Option<(u32, u32, u32)>, extra: &str) -> Vec<String> {
    let mut w = vec!["gamescope".to_string()];
    if extra.trim().is_empty() {
        let (ow, oh, hz) = output.unwrap_or((inner.0, inner.1, 60));
        for a in ["-W", &ow.to_string(), "-H", &oh.to_string(), "-w", &inner.0.to_string(), "-h", &inner.1.to_string(), "-r", &hz.to_string(), "-f"] {
            w.push(a.to_string());
        }
    } else {
        w.extend(extra.split_whitespace().map(String::from));
    }
    w.push("--".into());
    w
}

/// Build the command without running it (also used by tests and the UI's
/// "show command" feature).
pub fn build_command(settings: &Settings, r: &Resolved) -> io::Result<Command> {
    let missing = |what: &str| io::Error::new(io::ErrorKind::NotFound, format!("{what} not configured"));
    let game = r.game_dir.as_ref().ok_or_else(|| missing("game dir"))?;
    let compat = r.compat_data_dir.as_ref().ok_or_else(|| missing("Proton prefix"))?;
    let proton = r.proton_dir.as_ref().ok_or_else(|| missing("Proton"))?;
    let steam = r.steam_root.as_ref().ok_or_else(|| missing("Steam root"))?;

    // [gamescope ... --] [wrapper tokens, e.g. "gamemoderun"] proton run acs.exe
    let mut words: Vec<String> = Vec::new();
    if settings.use_gamescope {
        // The game renders at video.ini's size; gamescope scales that to the monitor.
        let video = r.cfg_dir().map(|c| crate::video::read_config(&crate::video::load_text(&c, game)));
        let inner = video.map(|v| (v.width, v.height)).unwrap_or((1920, 1080));
        let output = crate::video::detect_primary_mode().map(|m| (m.width, m.height, m.refresh));
        words.extend(gamescope_words(inner, output, &settings.gamescope_args));
    }
    words.extend(settings.wrapper.split_whitespace().map(String::from));
    words.push(proton.join("proton").to_string_lossy().into_owned());
    words.push("run".into());
    words.push(game.join("acs.exe").to_string_lossy().into_owned());

    let mut cmd = Command::new(&words[0]);
    cmd.args(&words[1..])
        .current_dir(game)
        .env("STEAM_COMPAT_DATA_PATH", compat)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam)
        .env("SteamAppId", AC_APP_ID)
        .env("SteamGameId", AC_APP_ID);
    for (k, v) in parse_env(&settings.extra_env) {
        cmd.env(k, v);
    }
    Ok(cmd)
}

/// Human-readable form of the command for display/debugging.
pub fn describe(cmd: &Command) -> String {
    let envs: Vec<String> = cmd
        .get_envs()
        .filter_map(|(k, v)| v.map(|v| format!("{}={:?}", k.to_string_lossy(), v.to_string_lossy())))
        .collect();
    let args: Vec<String> = cmd.get_args().map(|a| format!("{:?}", a.to_string_lossy())).collect();
    format!("{} {:?} {}", envs.join(" "), cmd.get_program().to_string_lossy(), args.join(" "))
}

/// Spawn the game. Output is redirected to [`launch_log_path`].
pub fn spawn(settings: &Settings, r: &Resolved) -> io::Result<Child> {
    let mut cmd = build_command(settings, r)?;
    if let Some(log) = launch_log_path() {
        if let Some(dir) = log.parent() {
            fs::create_dir_all(dir)?;
        }
        let out = File::create(&log)?;
        cmd.stdout(Stdio::from(out.try_clone()?)).stderr(Stdio::from(out));
    }
    cmd.stdin(Stdio::null());
    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_parsing() {
        let e = parse_env("# c\nDXVK_HUD=fps\n\nbad line\nA = b=c");
        assert_eq!(e, vec![("DXVK_HUD".into(), "fps".into()), ("A".into(), "b=c".into())]);
    }

    #[test]
    fn gamescope_args() {
        let w = gamescope_words((1600, 900), Some((1920, 1080, 75)), "");
        assert_eq!(w.join(" "), "gamescope -W 1920 -H 1080 -w 1600 -h 900 -r 75 -f --");
        assert_eq!(gamescope_words((1, 1), None, "-f -W 10").join(" "), "gamescope -f -W 10 --");
    }

    #[test]
    fn command_shape() {
        let r = Resolved {
            game_dir: Some("/g/ac".into()),
            compat_data_dir: Some("/g/compat".into()),
            proton_dir: Some("/g/Proton 11.0".into()),
            steam_root: Some("/s".into()),
            ..Default::default()
        };
        let s = Settings { wrapper: "gamemoderun".into(), ..Default::default() };
        let cmd = build_command(&s, &r).unwrap();
        assert_eq!(cmd.get_program(), "gamemoderun");
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["/g/Proton 11.0/proton", "run", "/g/ac/acs.exe"]);
    }
}
