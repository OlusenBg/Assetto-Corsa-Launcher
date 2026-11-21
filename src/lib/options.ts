// Defaults and persistence for the Session page options.
import type { Assists, SessionOptions } from "./api";

export const defaultAssists: Assists = {
  idealLine: true, autoBlip: true, stabilityControl: 100, autoBrake: false, autoShifter: true,
  abs: 2, tractionControl: 2, autoClutch: true, visualDamage: 0, damage: 0,
  fuelRate: 0, tyreWear: 1, tyreBlankets: true, slipstream: 1,
};

export const defaultOptions: SessionOptions = {
  sessions: [{ kind: "practice", minutes: 0, laps: 0 }],
  weather: { id: "3_clear", hour: 12, ambient: 22, timeMult: 1 },
  ai: { count: 0, strength: 90, aggression: 30, cars: [] },
  penalties: false,
  startPosition: 1,
  assists: defaultAssists,
  assistPreset: "gamer",
};

const KEY = "sessionOptions.v1";

/** Load saved options, filling any missing field from the defaults. */
export function loadOptions(): SessionOptions {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return {
      ...defaultOptions, ...saved,
      weather: { ...defaultOptions.weather, ...saved.weather },
      ai: { ...defaultOptions.ai, ...saved.ai },
      assists: { ...defaultAssists, ...saved.assists },
    };
  } catch { return structuredClone(defaultOptions); }
}

export function saveOptions(o: SessionOptions) {
  try { localStorage.setItem(KEY, JSON.stringify(o)); } catch { /* storage unavailable: fine */ }
}
