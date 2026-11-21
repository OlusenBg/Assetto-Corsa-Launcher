// Typed wrappers around the Tauri commands exposed by src-tauri/src/lib.rs.
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export interface Settings {
  gameDir: string | null;
  compatDataDir: string | null;
  protonDir: string | null;
  steamRoot: string | null;
  wrapper: string;
  extraEnv: string;
  windowedLauncher: boolean;
  useGamescope: boolean;
  gamescopeArgs: string;
  stayOpenOnLaunch: boolean;
}
export interface Issue { level: "info" | "warning" | "error"; message: string }
export interface ProtonInstall { name: string; dir: string }
export interface Resolved {
  steamRoot: string | null;
  gameDir: string | null;
  compatDataDir: string | null;
  protonDir: string | null;
  protonCandidates: ProtonInstall[];
  userDocs: string | null;
  issues: Issue[];
}
export interface AppState { settings: Settings; resolved: Resolved }

export interface CarSpecs {
  bhp?: string; torque?: string; weight?: string;
  topspeed?: string; acceleration?: string; pwratio?: string;
}
export interface Car {
  id: string; name: string; brand?: string; class?: string; tags: string[];
  specs: CarSpecs; description?: string; badge?: string; skinCount: number;
  thumbnail?: string; hasMetadata: boolean;
}
export interface Skin { id: string; name: string; number?: string; team?: string; preview?: string }
export interface TrackLayout {
  config: string; name: string; description?: string; lengthM?: number;
  pitboxes?: number; run?: string; preview?: string; outline?: string;
}
export interface Track {
  id: string; name: string; country?: string; city?: string; tags: string[]; layouts: TrackLayout[];
}
export interface Weather { id: string; name: string; temperatureCoeff: number }
export type SessionKind = "practice" | "qualify" | "race";
export interface SessionSpec { kind: SessionKind; minutes: number; laps: number }
export interface WeatherSpec { id: string; hour: number; ambient: number; timeMult: number; temperatureCoeff: number }
export interface AiSpec { count: number; strength: number; aggression: number; cars: string[] }
export interface RaceRequest {
  car: string; skin: string; track: string; trackConfig: string; driverName: string;
  sessions: SessionSpec[]; weather: WeatherSpec; ai: AiSpec; penalties: boolean; startPosition: number;
}
export interface Assists {
  idealLine: boolean; autoBlip: boolean; stabilityControl: number; autoBrake: boolean; autoShifter: boolean;
  abs: number; tractionControl: number; autoClutch: boolean; visualDamage: number; damage: number;
  fuelRate: number; tyreWear: number; tyreBlankets: boolean; slipstream: number;
}
export interface AssistPreset { id: string; name: string; assists: Assists }
/** Everything on the Session page; combined with car/track picks into a RaceRequest. */
export interface SessionOptions {
  sessions: SessionSpec[]; weather: Omit<WeatherSpec, "temperatureCoeff">; ai: AiSpec;
  penalties: boolean; startPosition: number; assists: Assists; assistPreset: string;
}
export interface AxisInfo { index: number; code: number; name: string; min: number; max: number; value: number }
export interface ButtonInfo { index: number; code: number }
export interface InputDevice {
  path: string; name: string; vendor: number; product: number; pguid: string; axes: AxisInfo[]; buttons: ButtonInfo[]; hats: number;
}
export interface InputSnapshot { path: string; axes: Record<string, number>; buttons: number[] }
export interface DeviceRef { name: string; pguid: string }
export interface AxisBinding { joy: number; axle: number; invert: boolean; gamma: number | null }
export interface ButtonBinding { action: string; label: string; joy: number; button: number }
export interface WheelTuning {
  lock: number; steerGamma: number; steerFilter: number; speedSensitivity: number; ffGain: number; filterFf: number;
  minFf: number; centerBoostGain: number; centerBoostRange: number; curbs: number; road: number; slips: number;
  abs: number; understeer: number;
}
export interface ControlsConfig {
  inputMethod: "WHEEL" | "X360" | "KEYBOARD"; devices: DeviceRef[];
  steer: AxisBinding; throttle: AxisBinding; brakes: AxisBinding; clutch: AxisBinding;
  buttons: ButtonBinding[]; tuning: WheelTuning;
}
export interface LaunchInfo { command: string; logPath?: string }

export const getState = () => invoke<AppState>("get_state");
export const saveSettings = (settings: Settings) => invoke<AppState>("save_settings", { settings });
export const listCars = () => invoke<Car[]>("list_cars");
export const listSkins = (car: string) => invoke<Skin[]>("list_skins", { car });
export const listTracks = () => invoke<Track[]>("list_tracks");
export const listWeather = () => invoke<Weather[]>("list_weather");
export const assistPresets = () => invoke<AssistPreset[]>("assist_presets");
export const launchRace = (request: RaceRequest, assists: Assists) => invoke<LaunchInfo>("launch_race", { request, assists });
export const readLaunchLog = () => invoke<string>("read_launch_log");
export const readGameLog = () => invoke<string>("read_game_log");

/** Turn an absolute file path into a URL the webview may load (asset protocol). */
export const img = (path?: string | null) => (path ? convertFileSrc(path) : "");

export const listInputDevices = () => invoke<{ devices: InputDevice[]; unreadable: number }>("list_input_devices");
export const startInputMonitor = () => invoke<void>("start_input_monitor");
export const stopInputMonitor = () => invoke<void>("stop_input_monitor");
export const readControls = () => invoke<ControlsConfig>("read_controls");
export const saveControls = (config: ControlsConfig) => invoke<void>("save_controls", { config });
export const g923Preset = () => invoke<ControlsConfig>("g923_preset");

export interface VideoConfig {
  width: number; height: number; refresh: number; fullscreen: boolean; vsync: boolean; fpsCap: number;
  anisotropic: number; aaSamples: number; aaQuality: number; shadowMapSize: number; worldDetail: number;
  motionBlur: number; fxaa: boolean; smoke: number; smokeInMirror: boolean;
  cubemapSize: number; cubemapFacesPerFrame: number; cubemapFarplane: number; mirrorSize: number; mirrorHq: boolean;
  hideArms: boolean; hideSteer: boolean; saturation: number;
  ppEnabled: boolean; heatShimmer: boolean; raysOfGod: boolean; ppQuality: number; glare: number; dof: number; ppFilter: string;
}
export interface DisplayMode { width: number; height: number; refresh: number[] }
export interface AudioConfig {
  master: number; engine: number; wind: number; tyres: number; opponents: number; music: number;
  uiSounds: number; surfaces: number; dirtBottom: number; skidEntryPoint: number;
}
export const readVideo = () => invoke<{ config: VideoConfig; modes: DisplayMode[]; filters: string[] }>("read_video");
export const saveVideo = (config: VideoConfig) => invoke<void>("save_video", { config });
export const videoPreset = (config: VideoConfig, preset: string) => invoke<VideoConfig>("video_preset", { config, preset });
export const readAudio = () => invoke<AudioConfig>("read_audio");
export const saveAudio = (config: AudioConfig) => invoke<void>("save_audio", { config });
export const toggleFullscreen = () => invoke<void>("toggle_fullscreen");

export interface Replay { name: string; temp: boolean; size: number; modified: number; car?: string; track?: string; layout?: string }
export interface SetupFile { car: string; scope: string; name: string; path: string; size: number; modified: number; builtin: boolean }
export interface ModItem { kind: "car" | "track" | "skin"; id: string; car?: string; exists: boolean; needsCar: boolean }
export interface ModPlan { id: number; archive: string; items: ModItem[]; warnings: string[] }
export const listReplays = () => invoke<Replay[]>("list_replays");
export const playReplay = (name: string, temp: boolean) => invoke<LaunchInfo>("play_replay", { name, temp });
export const keepReplay = (name: string) => invoke<string>("keep_replay", { name });
export const deleteReplay = (name: string, temp: boolean) => invoke<void>("delete_replay", { name, temp });
export const listSetups = (car: string | null) => invoke<SetupFile[]>("list_setups", { car });
export const importSetup = (file: string, car: string | null, scope: string) => invoke<SetupFile>("import_setup", { file, car, scope });
export const exportSetup = (path: string, dest: string) => invoke<void>("export_setup", { path, dest });
export const adoptSetup = (path: string) => invoke<SetupFile>("adopt_setup", { path });
export const deleteSetup = (path: string) => invoke<void>("delete_setup", { path });
export const modInspect = (archive: string) => invoke<ModPlan>("mod_inspect", { archive });
export const modInstall = (id: number, overwrite: boolean, skinCar: string | null) => invoke<string[]>("mod_install", { id, overwrite, skinCar });
export const modDiscard = (id: number) => invoke<void>("mod_discard", { id });

export type Page = "home" | "drive" | "session" | "extras" | "settings";
export type SettingsTab = "general" | "controls" | "video" | "audio";
