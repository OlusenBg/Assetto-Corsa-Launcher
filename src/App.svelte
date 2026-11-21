<script lang="ts">
  // Root component: owns app state, loads content, and wires the pages together.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { fade } from "svelte/transition";
  import {
    getState, toggleFullscreen, playReplay, listCars, listSkins, listTracks, listWeather, assistPresets, launchRace,
    type AppState, type AssistPreset, type Car, type Page, type SettingsTab, type Skin, type Track, type Weather,
  } from "./lib/api";
  import Nav from "./lib/Nav.svelte";
  import Splash from "./lib/Splash.svelte";
  import Home from "./lib/Home.svelte";
  import SettingsHub from "./lib/SettingsHub.svelte";
  import CarGrid from "./lib/CarGrid.svelte";
  import TrackGrid from "./lib/TrackGrid.svelte";
  import Summary from "./lib/Summary.svelte";
  import LogDrawer from "./lib/LogDrawer.svelte";
  import SessionPage from "./lib/SessionPage.svelte";
  import ExtrasPage from "./lib/ExtrasPage.svelte";
  import { loadOptions, saveOptions } from "./lib/options";

  let booting = $state(true); // splash screen until the first load finishes
  let page = $state<Page>("home");
  let settingsTab = $state<SettingsTab>("general");
  let extrasTab = $state<"replays" | "setups" | "install">("replays");
  let tab = $state<"car" | "track">("car");
  let app = $state<AppState | null>(null);
  let cars = $state<Car[]>([]);
  let tracks = $state<Track[]>([]);
  let skins = $state<Skin[]>([]);
  let loadError = $state("");
  let weather = $state<Weather[]>([]);
  let presets = $state<AssistPreset[]>([]);
  let options = $state(loadOptions());

  // Selection (remembered between runs)
  const saved = (() => { try { return JSON.parse(localStorage.getItem("sel") ?? "{}"); } catch { return {}; } })();
  let carId = $state<string>(saved.carId ?? "");
  let skinId = $state<string>(saved.skinId ?? "");
  let trackId = $state<string>(saved.trackId ?? "");
  let layout = $state<string>(saved.layout ?? "");

  let running = $state(false);
  let launchMsg = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  let logOpen = $state(false);

  const car = $derived(cars.find((c) => c.id === carId));
  const track = $derived(tracks.find((t) => t.id === trackId));
  const skin = $derived(skins.find((s) => s.id === skinId));

  async function load() {
    loadError = "";
    try {
      app = await getState();
      if (app.resolved.issues.some((i) => i.level === "error" && !app!.resolved.gameDir)) {
        cars = []; tracks = [];
        page = "settings";
        settingsTab = "general";
        return;
      }
      [cars, tracks, weather, presets] = await Promise.all([listCars(), listTracks(), listWeather(), assistPresets()]);
      if (weather.length && !weather.some((w) => w.id === options.weather.id)) options.weather.id = weather[0].id;
      // Default / repair the selection.
      if (!cars.some((c) => c.id === carId)) carId = cars.find((c) => c.id.includes("mx5"))?.id ?? cars[0]?.id ?? "";
      if (!tracks.some((t) => t.id === trackId)) { trackId = tracks[0]?.id ?? ""; layout = ""; }
    } catch (e) {
      loadError = String(e);
    }
  }

  // Reload skins whenever the car changes.
  $effect(() => {
    const id = carId;
    if (!id) { skins = []; return; }
    listSkins(id).then((s) => {
      if (id !== carId) return; // stale response
      skins = s;
      if (!s.some((x) => x.id === skinId)) skinId = s[0]?.id ?? "";
    });
  });

  // Keep the layout valid for the track.
  $effect(() => {
    if (track && !track.layouts.some((l) => l.config === layout)) layout = track.layouts[0].config;
  });

  // Persist selection.
  $effect(() => {
    try { localStorage.setItem("sel", JSON.stringify({ carId, skinId, trackId, layout })); } catch {}
  });

  // Persist session options.
  $effect(() => { saveOptions($state.snapshot(options)); });

  const layoutInfo = $derived(track?.layouts.find((l) => l.config === layout));
  /** Opponents the selected layout can hold (pit boxes - 1). */
  const maxAi = $derived(Math.max(0, Math.min((layoutInfo?.pitboxes ?? 20) - 1, 40)));
  const startLabel = $derived(
    options.sessions.length === 1
      ? `Start ${options.sessions[0].kind}`
      : `Start weekend (${options.sessions.map((s) => s.kind).join(" → ")})`,
  );

  async function start() {
    if (!car || !track || !skinId) return;
    launchMsg = null;
    try {
      const o = $state.snapshot(options);
      const coeff = weather.find((w) => w.id === o.weather.id)?.temperatureCoeff ?? 0;
      await launchRace({
        car: car.id, skin: skinId, track: track.id, trackConfig: layout, driverName: "",
        sessions: o.sessions,
        weather: { ...o.weather, temperatureCoeff: coeff },
        ai: { ...o.ai, count: Math.min(o.ai.count, maxAi) },
        penalties: o.penalties, startPosition: o.startPosition,
      }, o.assists);
      running = true;
      launchMsg = { kind: "ok", text: "Game starting through Proton…" };
    } catch (e) {
      launchMsg = { kind: "err", text: String(e) };
      logOpen = true;
    }
  }

  async function playReplayFile(name: string, temp: boolean) {
    launchMsg = null;
    try {
      await playReplay(name, temp);
      running = true;
      launchMsg = { kind: "ok", text: "Starting replay…" };
    } catch (e) {
      launchMsg = { kind: "err", text: String(e) };
      logOpen = true;
    }
  }

  /** Show the splash for a minimum time so the animation can be seen, then reveal the app. */
  async function boot() {
    const started = Date.now();
    await load();
    await new Promise((r) => setTimeout(r, Math.max(0, 2200 - (Date.now() - started))));
    booting = false;
  }

  onMount(() => {
    boot();
    const f11 = (e: KeyboardEvent) => { if (e.key === "F11") { e.preventDefault(); toggleFullscreen(); } };
    window.addEventListener("keydown", f11);
    const un = listen<number | null>("game-exited", (e) => {
      running = false;
      const code = e.payload;
      launchMsg = code === 0 || code === null
        ? { kind: "ok", text: "Game closed." }
        : { kind: "err", text: `Game exited with code ${code}. Open the log for details.` };
      if (code !== 0 && code !== null) logOpen = true;
    });
    return () => { un.then((f) => f()); window.removeEventListener("keydown", f11); };
  });
</script>

{#if booting}
  <Splash />
{:else}
  <div class="shell" in:fade={{ duration: 500 }}>
    <Nav {page} onnav={(p) => (page = p)} />

    <main>
      {#key page}
        <div class="view" in:fade={{ duration: 220 }}>
          {#if page === "home"}
            <Home
              {app} {car} skinPreview={skin?.preview} {track} {layout} {options} {weather} {running} {launchMsg}
              onstart={start} onnav={(p) => (page = p)}
              onextras={(t) => { extrasTab = t; page = "extras"; }}
              ondrive={(t) => { tab = t; page = "drive"; }}
            />
          {:else if page === "drive"}
            <div class="drive">
              <section class="browser">
                <header>
                  <div class="seg" data-tab={tab}>
                    <span class="thumb"></span>
                    <button class:on={tab === "car"} onclick={() => (tab = "car")}>Cars <small>{cars.length}</small></button>
                    <button class:on={tab === "track"} onclick={() => (tab = "track")}>Tracks <small>{tracks.length}</small></button>
                  </div>
                </header>
                {#if loadError}
                  <p class="msg err">{loadError}</p>
                {:else if tab === "car"}
                  <CarGrid {cars} selected={carId} onselect={(id) => { carId = id; }} />
                {:else}
                  <TrackGrid {tracks} selected={trackId} onselect={(id) => { trackId = id; layout = ""; }} />
                {/if}
              </section>
              <Summary
                {car} {skins} {skinId} {track} {layout} {running} {launchMsg} label={startLabel}
                onskin={(id) => (skinId = id)}
                onlayout={(c) => (layout = c)}
                onpick={(t) => (tab = t)}
                onstart={start}
                onlog={() => (logOpen = !logOpen)}
              />
            </div>
          {:else if page === "session"}
            <SessionPage bind:options {weather} {presets} {cars} {maxAi} />
          {:else if page === "extras"}
            <ExtrasPage {cars} {tracks} {carId} {running} {launchMsg} initialTab={extrasTab} onchanged={load} onplay={playReplayFile} />
          {:else}
            <SettingsHub bind:tab={settingsTab} {app} onsaved={(s) => { app = s; load(); }} />
          {/if}
        </div>
      {/key}
    </main>

    <LogDrawer open={logOpen} onclose={() => (logOpen = false)} />
  </div>
{/if}

<style>
  .shell { display: flex; flex-direction: column; height: 100%; }
  main { flex: 1; min-height: 0; position: relative; }
  .view { height: 100%; }

  .drive { height: 100%; display: flex; }
  .browser { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; padding: 28px 8px 0 40px; }
  header { margin-bottom: 20px; padding-right: 24px; }

  /* Segmented control with a sliding thumb */
  .seg { position: relative; display: inline-grid; grid-template-columns: 1fr 1fr; border: 1px solid var(--line-2); border-radius: 999px; padding: 3px; width: 260px; }
  .seg button { position: relative; z-index: 1; padding: 8px 0; border-radius: 999px; color: var(--fg-3); font-weight: 500; transition: color 0.25s; }
  .seg button small { margin-left: 4px; color: inherit; opacity: 0.7; }
  .seg button.on { color: #000; }
  .thumb { position: absolute; top: 3px; bottom: 3px; left: 3px; width: calc(50% - 3px); border-radius: 999px; background: #fff; transition: transform 0.35s var(--ease); }
  .seg[data-tab="track"] .thumb { transform: translateX(100%); }
</style>
