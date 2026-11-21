# Assetto Corsa Launcher

A clean, black-and-white, Linux-native launcher for **Assetto Corsa** (Steam app 244210) running under Proton.

The stock `AssettoCorsa.exe` launcher crashes under Proton/Wine because it cannot load its embedded CEF3 browser. The game itself (`acs.exe`) runs fine, so this launcher does what the stock one did: it writes the game's `.ini` config files and then starts `acs.exe` through Proton.

> Unofficial and not affiliated with Kunos Simulazioni or 505 Games. This project contains no game assets or code from the game; it only reads the plain-text files your own installation ships with.

## Features

- **Home**: your current car, track and session at a glance, one big Start button.
- **Drive**: browse every installed car (with liveries) and track (with layouts), with previews.
- **Session**: practice / qualify / race weekends, weather and time of day, AI opponents (count, skill, aggression), penalties, grid position and assists (Gamer / Racer / Pro presets or individual).
- **Controls**: detects wheels and joysticks, binds axes and buttons by moving them, wheel rotation, pedal inversion, force-feedback tuning. Writes `controls.ini`.
- **Video / Audio**: resolution and refresh rate from your real display modes, quality presets, effects, volume levels.
- **Extras**: replays, car setups (import / export), and installing cars, tracks and skins from `.zip` / `.rar` / `.7z`.
- Auto-detects the game, Proton prefix and Proton version from Steam's own files. Every path can be overridden in Settings.
- Optional **gamescope** launch for proper fullscreen on Wayland.

## Requirements

- Linux with Steam, Assetto Corsa installed, and a Proton prefix (launch the game once from Steam).
- Build: Rust, Node.js, and the Tauri 2 system libraries (`webkit2gtk-4.1`, `gtk3`).
- Optional: `gamescope` (fullscreen on Wayland), `bsdtar` or `7z` (installing mods).
- Reading input devices needs access to `/dev/input/event*` (usually membership of the `input` group).

On Arch: `sudo pacman -S webkit2gtk-4.1 gtk3 rust nodejs npm libarchive`

## Build and run

```sh
npm install
npm run tauri dev                       # development
npm run tauri build -- --no-bundle      # release binary: target/release/ac-linux-launcher
```

Steam must be running when you start the game.

### Starting it from Steam

Put this in the game's **Properties → Launch Options** so Steam's Play button opens the launcher instead of the broken stock one:

```
/path/to/ac-linux-launcher %command%
```

If Steam's runtime container prevents the window from opening, add the binary as a **non-Steam game** instead.

## How it works

| Piece | What it does |
|---|---|
| `crates/ac-core` | All the logic, no UI: path detection, content scanning, lossless INI editing, `race.ini` / `controls.ini` / `video.ini` / `audio.ini` / `assists.ini` writers, device input (evdev), replays, setups, mod install, Proton launching. Unit-tested. |
| `src-tauri` | Thin Tauri 2 command layer over `ac-core`. |
| `src` | Svelte 5 + TypeScript interface. |
| `docs/ac-formats.md` | The file formats and behaviours observed while building this. |

Notable findings (details in `docs/ac-formats.md`): the game reads **only** the config in `<prefix>/.../Documents/Assetto Corsa/cfg/` (it does not fall back to the install's `cfg/`), and under Wine axes and buttons are numbered in the order the device reports them.

Settings live in `~/.config/ac-linux-launcher/settings.json`. The launcher keeps a one-time backup of each game config file it first changes (`*.launcher-backup`).

### Headless checks

```sh
cargo test --workspace
AC_GAME_DIR=/path/to/assettocorsa cargo test -p ac-core -- --nocapture
cargo run -p ac-core --example dry_run            # prints detection, race.ini, launch command
```

## Known limitations

- D-pad (hat) buttons are not bindable yet: the encoding AC expects is unconfirmed.
- Replay playback and the race `STARTING_POSITION` / qualify session type values follow community documentation and are not yet verified in-game.
- Native fullscreen under XWayland can come up stretched; use the gamescope option.
- Out of scope for now: online lobby, Custom Shaders Patch.

## License

[MIT](LICENSE)
