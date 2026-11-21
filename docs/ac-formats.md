# Assetto Corsa file formats (as observed)

Observed on AC 1.16.4 (x64) under Proton 11.0. Nothing here is copied from
Kunos binaries; it is derived from plain-text files shipped in the install
and from the game's own log.

## Locations

| What | Path |
|---|---|
| Game install | `<steamlib>/steamapps/common/assettocorsa` |
| Prefix | `<steamlib>/steamapps/compatdata/244210/pfx` |
| User docs | `<prefix>/drive_c/users/steamuser/Documents/Assetto Corsa` |
| User cfg (read/written by game) | `<user docs>/cfg/*.ini` (absent until first written) |
| Default cfg shipped | `<install>/cfg/*.ini` (read-only defaults) |
| Game log | `<user docs>/logs/log.txt` |
| Race result | `<user docs>/out/race_out.json` |
| Proton used | `compatdata/244210/config_info` (line 1 hints at the Proton dir) |

The log prints `-----------PRINTING RACE INI-----------` followed by the
parsed race.ini. On our first run it printed empty, i.e. the game looked for
`<user docs>/cfg/race.ini`, found nothing, and fell back to built-in defaults
(lotus_elise_sc @ magione). To be verified in Phase 2: writing race.ini there
changes what the log prints.

## INI dialect

- Windows-style: `[SECTION]`, `KEY=VALUE`, CRLF line endings in shipped files.
- Comments start with `;` and may trail a value (`ABS=2 ; note`). Some trail
  with no space (`GAS=0x26 ; Up`). Values must be trimmed after stripping comments.
- Keys are upper-case in practice; one shipped file has `[LIGHTING] ` with a
  trailing space and `[GEARUP] ` with trailing spaces: parser must trim headers.
- Duplicate-free per section. Section order seems not to matter.
- Writer rule: preserve unknown keys/sections and comments when editing an
  existing file (round-trip), only create from scratch if missing.

## race.ini (what the game consumes at launch)

```
[RACE]        TRACK, CONFIG_TRACK (layout folder, empty = default), MODEL, MODEL_CONFIG,
              CARS (total count incl. AI), AI_LEVEL, FIXED_SETUP, PENALTIES
[GHOST_CAR]   RECORDING, PLAYING, SECONDS_ADVANTAGE, LOAD, FILE
[REPLAY]      FILENAME, ACTIVE
[LIGHTING]    SUN_ANGLE (-80..80ish; -48 ~ 08:00? mapping TBD), TIME_MULT, CLOUD_SPEED
[GROOVE]      VIRTUAL_LAPS, MAX_LAPS, STARTING_LAPS
[DYNAMIC_TRACK] SESSION_START, SESSION_TRANSFER, RANDOMNESS, LAP_GAIN
[REMOTE]      ACTIVE=0 (online; out of scope)
[LAP_INVALIDATOR] ALLOWED_TYRES_OUT
[TEMPERATURE] AMBIENT, ROAD
[WEATHER]     NAME = folder name in content/weather (e.g. 4_mid_clear)
[CAR_0]       MODEL (- = same as RACE), MODEL_CONFIG, SKIN, DRIVER_NAME, NATIONALITY, AI_LEVEL
[CAR_1..n]    same shape; AI opponents
[SESSION_0..] (not in the shipped default; the game's own launcher writes
              NAME, TYPE (1=practice, 2=qualify, 3=race), DURATION_MINUTES, LAPS,
              SPAWN_SET) -> TO CONFIRM in Phase 3 by observing out/race_out.json
              and log, since the shipped race.ini has no session block.
```

`out/race_out.json` after a run shows `sessions[].type = 1` for the practice
session, matching TYPE=1 = practice.

## Content layout

Cars: `content/cars/<id>/`
- `ui/ui_car.json` (JSON-ish: **contains raw newlines inside strings and
  possibly tabs**, so a strict JSON parser fails; need lenient parsing).
  Fields: name, brand, description, tags[], class, specs{bhp,torque,weight,
  topspeed,acceleration,pwratio}, torqueCurve[[rpm,nm]], powerCurve.
  2 of 180 cars lack it (`ks_ferrari_488_challenge_evo`, `ks_ferrari_488_gt3_2020`): fall back to folder name.
- `ui/badge.png` brand badge.
- `skins/<skin_id>/preview.jpg` + optional `ui_skin.json` {skinname, drivername,
  country, team, number, priority}. `skin_id` is what goes in `[CAR_0] SKIN=`.
- `data.acd` is an archive (packed physics); not read by us.
- Some cars have multiple configs (`MODEL_CONFIG`); to inspect in Phase 2.

Tracks: `content/tracks/<id>/`
- Single-layout: `ui/ui_track.json`, `ui/preview.png`, `ui/outline.png`.
- Multi-layout (e.g. ks_nordschleife): `ui/<layout>/ui_track.json|preview.png|outline.png`,
  `models_<layout>.ini`. The layout folder name goes in `CONFIG_TRACK=`.
- ui_track.json: name, description, tags, country, city, length (m, string),
  width, pitboxes, run. Contains non-UTF8 bytes (degree sign in geotags): decode as Latin-1/lossy.
- Of 21 track folders, count of layouts to be enumerated in Phase 2.

Weather: `content/weather/<id>/weather.ini`, `[LAUNCHER] NAME=` is the display name.
Shipped: 1_heavy_fog, 2_light_fog, 3_clear, 4_mid_clear, 5_light_clouds,
6_mid_clouds, 7_heavy_clouds.

## Other cfg files (phases 3-5)

- `assists.ini [ASSISTS]`: IDEAL_LINE, AUTO_BLIP, STABILITY_CONTROL (0-100), AUTO_BRAKE,
  AUTO_SHIFTER, ABS (0-2), TRACTION_CONTROL (0-2), AUTO_CLUTCH, VISUALDAMAGE, DAMAGE (0-100),
  FUEL_RATE, TYRE_WEAR, TYRE_BLANKETS, SLIPSTREAM.
- `controls.ini`: `[HEADER] INPUT_METHOD=KEYBOARD|WHEEL|X360`; `[CONTROLLERS]
  CON0=<device name>`, `PGUID0=<guid>`; `[STEER] JOY AXLE SCALE LOCK FF_GAIN FILTER_FF
  STEER_GAMMA STEER_FILTER SPEED_SENSITIVITY DEBOUNCING_MS`; `[THROTTLE]/[BRAKES]/[CLUTCH]
  JOY AXLE MIN MAX (GAMMA)`; per-action `[GEARUP] JOY BUTTON KEY XBOXBUTTON`; `[FF_TWEAKS]`,
  `[FF_ENHANCEMENT]`. `JOY=-1` = unbound; `JOY` is the index into the CONn list.
  Keys are Windows VK hex codes (`0x26`).
  Pedal inversion = swap MIN/MAX (preset: throttle MIN=1 MAX=-1).
  `PGUID` format: `<PID><VID>-0000-0000-0000-504944564944` (little-endian IDs: G25 = C299046D).
  Expected for G923 (USB 046d:c266): `C266046D-0000-0000-0000-504944564944`; to confirm.
- `cfg/controllers/presets/*.ini`: complete controls.ini templates per wheel (Fanatec, G25, ...).
  No G923 preset ships; we can generate one.
- `video.ini`, `audio.ini`, `gameplay.ini`, `launcher.ini`: simple INI as above.
  `launcher.ini` holds the stock launcher's UI state only; the game doesn't need it.
- `cfg/templates/*.ini`: assist presets (gamer, pro, racer, custom, tracks).

## Launching

`acs.exe` reads cfg on start and writes `out/race_out.json` on exit. It needs
Steam running (steam_api64.dll). Preferred launch: Proton `run` verb with
`STEAM_COMPAT_DATA_PATH` and `STEAM_COMPAT_CLIENT_INSTALL_PATH` set, cwd = install dir.

## Findings from testing (Phases 2-3)

- **Only the user cfg is read.** The game reads `<docs>/cfg/*.ini` and does NOT fall back to
  `<install>/cfg/`. A user `video.ini` containing only `[POST_PROCESS]` made the game start
  windowed ("windowed mode requested"). The launcher therefore seeds every file it manages
  from the shipped default and edits it in place, with one-time `.launcher-backup` copies.
- **Wheel axes/buttons under Wine** are numbered in the order the device reports them
  (G923: X=0, Y=1, Z=2, Rz=3), not by fixed DirectInput offsets. Measured on a G923
  (046d:c266): steering X, throttle Z, brake Rz, clutch Y, pedals read 255 at rest so
  MIN=1 MAX=-1; paddles are buttons 4 (right, up) and 5 (left, down).
- **Fullscreen** under XWayland came up stretched and not interactive; windowed is the default.
- **Time of day:** `[LIGHTING] SUN_ANGLE = (hour - 13) * 16` (13:00 = 0, 08:00 = -80, 18:00 = +80).
  The shipped default -48 = 10:00.
- **Sessions:** `[SESSION_n]` with `NAME`, `TYPE` (1 practice, 2 qualify, 3 race),
  `DURATION_MINUTES`, `LAPS`, `SPAWN_SET` (PIT/START), race `STARTING_POSITION`.
  Types 2/3 and STARTING_POSITION come from community docs: verify in-game.
- **Opponents:** extra `[CAR_n]` blocks (MODEL, SKIN, DRIVER_NAME, AI_LEVEL, AI_AGGRESSION)
  and `[RACE] CARS` = 1 + opponents. Names come from `system/data/drivers.txt`.
- **Assists:** `assists.ini [ASSISTS]` (see `crates/ac-core/src/assists.rs`).

## video.ini / audio.ini (Phase 5)

- `video.ini [VIDEO]`: WIDTH, HEIGHT, REFRESH, FULLSCREEN, VSYNC, FPS_CAP_MS (ms per frame, 0 = off),
  ANISOTROPIC, AASAMPLES (0/2/4/8), AAQUALITY, SHADOW_MAP_SIZE. Other sections: `[ASSETTOCORSA]`
  WORLD_DETAIL 0-5, HIDE_ARMS, HIDE_STEER; `[EFFECTS]` MOTION_BLUR, FXAA, SMOKE, RENDER_SMOKE_IN_MIRROR;
  `[CUBEMAP]` SIZE, FACES_PER_FRAME, FARPLANE; `[MIRROR]` SIZE, HQ; `[SATURATION]` LEVEL;
  `[POST_PROCESS]` ENABLED, HEAT_SHIMMER, RAYS_OF_GOD, FXAA, QUALITY, GLARE, DOF, FILTER
  (a file in `system/cfg/ppfilters`). Quality presets in the launcher are our own groupings.
- `audio.ini [LEVELS]`: MASTER, ENGINE, WIND, TYRES, OPPONENTS, MUSIC, UISOUNDS, SURFACES, DIRT_BOTTOM
  (floats, 0-1); `[SKIDS] ENTRY_POINT`.
- D-pads (hats) are bound as buttons >= 128: `128 + 4*hat + (up, right, down, left)`. Assumed from
  DirectInput POV conventions; confirm against a binding made in the game's own menu.

## Replays, setups, content install (Phase 6)

- **Replays:** the game records the last session to `<docs>/replay/temp/AC_<ddmmyy>-<hhmmss>_<O>_<car>_<track>[_<layout>]`
  (no extension, ~7 MB). Saved replays are `<docs>/replay/<name>.acreplay`. Playback is requested through
  `race.ini [REPLAY] ACTIVE=1 FILENAME=<file in replay/>` (UNVERIFIED: written by the launcher, to be confirmed
  in-game). A normal session must reset `ACTIVE=0`, which the launcher does.
- **Setups:** `<docs>/setups/<car>/<track folder or "generic">/<name>.ini`; `last.ini` / `last.sp` are the game's
  autosave. Files are INI sections `[NAME] VALUE=n` and end with `[CAR] MODEL=<car id>`. The game ships a few in
  `<install>/system/setups/<car>/<track>.ini` (without a `[CAR]` section).
- **Content layout used for detection:** car = `ui/ui_car.json` (or `data.acd` + `skins/`); track = `ui/ui_track.json`
  or `ui/<layout>/ui_track.json`; skin = `ui_skin.json`, or `preview.jpg` inside a `skins/` folder
  (`<car>/skins/<skin>`). Archives are unpacked with `bsdtar` (libarchive) or `7z`.
