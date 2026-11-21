<script lang="ts">
  // Home: the current selection as a big hero, one clear Start button, and quick links.
  import { onMount } from "svelte";
  import { img, listInputDevices, type AppState, type Page, type Car, type SessionOptions, type Track, type Weather } from "./api";

  let { app, car, skinPreview, track, layout, options, weather, running, launchMsg, onstart, onnav, onextras, ondrive }: {
    app: AppState | null; car?: Car; skinPreview?: string; track?: Track; layout: string; options: SessionOptions; weather: Weather[];
    running: boolean; launchMsg: { kind: "ok" | "err"; text: string } | null;
    onstart: () => void; onnav: (p: Page) => void; onextras: (tab: "replays" | "setups" | "install") => void; ondrive: (tab: "car" | "track") => void;
  } = $props();

  let wheel = $state<string | null>(null);
  onMount(() => { listInputDevices().then((d) => (wheel = d.devices[0]?.name ?? null)).catch(() => {}); });

  const lay = $derived(track?.layouts.find((l) => l.config === layout) ?? track?.layouts[0]);
  const steamDown = $derived(!!app?.resolved.issues.some((i) => /steam/i.test(i.message) && i.level !== "info"));
  const proton = $derived(app?.resolved.protonDir?.split("/").pop() ?? "—");
  const label = $derived(options.sessions.length === 1 ? `Start ${options.sessions[0].kind}` : "Start weekend");
  const wname = $derived(weather.find((w) => w.id === options.weather.id)?.name ?? "");
  const clock = (h: number) => `${String(Math.floor(h)).padStart(2, "0")}:${String(Math.round((h % 1) * 60)).padStart(2, "0")}`;
  const facts = $derived([
    options.sessions.map((s) => s.kind[0].toUpperCase() + s.kind.slice(1)).join(" + "),
    wname, clock(options.weather.hour), `${options.weather.ambient}°C`,
    options.ai.count ? `${options.ai.count} opponents` : "Solo",
  ].filter(Boolean));
</script>

<div class="home">
  <div class="hero">{#key skinPreview ?? car?.thumbnail}<img src={img(skinPreview ?? car?.thumbnail)} alt="" />{/key}</div>

  <div class="content">
    <p class="eyebrow rise" style="--d:0.05s">Ready to drive</p>
    <h1 class="big rise" style="--d:0.12s">{car?.name ?? "Choose a car"}</h1>
    <p class="track rise" style="--d:0.2s">{lay?.name ?? "Choose a track"}{track?.country ? ` · ${track.country}` : ""}</p>
    <p class="facts rise" style="--d:0.28s">{#each facts as f, i}{#if i}<i>/</i>{/if}<span>{f}</span>{/each}</p>

    <div class="cta rise" style="--d:0.36s">
      <button class="btn primary lg" disabled={!car || !track || running} onclick={onstart}>{running ? "Running…" : label}</button>
      <button class="btn lg" onclick={() => ondrive("car")}>Car</button>
      <button class="btn lg" onclick={() => ondrive("track")}>Track</button>
      <button class="btn lg" onclick={() => onnav("session")}>Session</button>
    </div>
    {#if launchMsg}<p class="msg {launchMsg.kind} note">{launchMsg.text}</p>{/if}
    {#if app?.resolved.issues.some((i) => i.level === "error")}
      <p class="msg err note">{app.resolved.issues.find((i) => i.level === "error")?.message}
        <button class="link" onclick={() => onnav("settings")}>Open settings</button></p>
    {/if}
  </div>

  <footer class="rise" style="--d:0.5s">
    <div class="status">
      <span class="dot" class:bad={steamDown}></span><span>{steamDown ? "Steam not running" : "Steam ready"}</span>
      <span class="sep"></span>
      <span class="dot" class:off={!wheel}></span><span>{wheel ?? "No wheel detected"}</span>
      <span class="sep"></span>
      <span>{proton}</span>
    </div>
    <div class="quick">
      <button class="link" onclick={() => onextras("replays")}>Replays</button>
      <button class="link" onclick={() => onextras("setups")}>Setups</button>
      <button class="link" onclick={() => onextras("install")}>Install content</button>
    </div>
  </footer>
</div>

<style>
  .home { position: relative; height: 100%; overflow: hidden; display: flex; flex-direction: column; }
  /* The game's previews are only ~1022x574: show them near native size (max ~1.2x) and fade the edges into black. */
  .hero { position: absolute; top: 0; right: 0; bottom: 0; width: min(72%, 1240px); pointer-events: none;
    -webkit-mask-image: linear-gradient(to left, #000 55%, transparent 100%); mask-image: linear-gradient(to left, #000 55%, transparent 100%); }
  .hero img { position: absolute; right: 0; top: 50%; width: 100%; aspect-ratio: 16 / 9; height: auto; transform: translateY(-50%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 22%, #000 78%, transparent 100%);
    mask-image: linear-gradient(to bottom, transparent 0, #000 22%, #000 78%, transparent 100%);
    animation: reveal 1.1s var(--ease) both; }
  .content { position: relative; z-index: 1; flex: 1; display: flex; flex-direction: column; justify-content: center; padding: 0 64px; max-width: 760px; }
  .big { font-size: clamp(38px, 5.4vw, 72px); font-weight: 600; letter-spacing: -0.04em; line-height: 1.02; margin-top: 14px; }
  .track { font-size: 20px; color: var(--fg-2); margin-top: 14px; }
  .facts { margin-top: 22px; display: flex; flex-wrap: wrap; gap: 10px; font-size: 13px; color: var(--fg-3); }
  .facts i { font-style: normal; color: #333; }
  .cta { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 40px; }
  .note { margin-top: 18px; align-self: flex-start; }
  footer { position: relative; z-index: 1; display: flex; justify-content: space-between; align-items: center; padding: 20px 64px 26px; border-top: 1px solid var(--line); font-size: 12.5px; color: var(--fg-3); background: linear-gradient(to top, #000 40%, transparent); }
  .status { display: flex; align-items: center; gap: 10px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: #fff; }
  .dot.off { background: transparent; border: 1px solid var(--fg-3); }
  .dot.bad { background: transparent; border: 1px solid #fff; }
  .sep { width: 1px; height: 12px; background: var(--line-2); margin: 0 6px; }
  .quick { display: flex; gap: 22px; }
  .rise { animation: rise 0.8s var(--ease) both; animation-delay: var(--d, 0s); }
  @keyframes rise { from { opacity: 0; transform: translateY(14px); } to { opacity: 1; transform: none; } }
  @keyframes reveal { from { opacity: 0; } to { opacity: 1; } }
</style>
