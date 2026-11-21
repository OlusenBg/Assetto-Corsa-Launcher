<script lang="ts">
  // Video: display mode, quality presets and graphics options (writes video.ini).
  import { onMount } from "svelte";
  import { readVideo, saveVideo, videoPreset, type DisplayMode, type VideoConfig } from "./api";

  let config = $state<VideoConfig | null>(null);
  let modes = $state<DisplayMode[]>([]);
  let filters = $state<string[]>([]);
  let dirty = $state(false);
  let status = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  const touch = () => (dirty = true);

  async function load() {
    try {
      const v = await readVideo();
      config = v.config; modes = v.modes; filters = v.filters; dirty = false;
    } catch (e) { status = { kind: "err", text: String(e) }; }
  }

  // Resolution list: detected display modes plus the current (windowed) size if it is not among them.
  const resolutions = $derived.by(() => {
    const list = modes.map((m) => `${m.width}x${m.height}`);
    if (config && !list.includes(`${config.width}x${config.height}`)) list.push(`${config.width}x${config.height}`);
    return list;
  });
  const rates = $derived(
    modes.find((m) => config && m.width === config.width && m.height === config.height)?.refresh ?? [config?.refresh ?? 60],
  );

  function setResolution(v: string) {
    if (!config) return;
    const [w, h] = v.split("x").map(Number);
    config.width = w; config.height = h;
    const r = modes.find((m) => m.width === w && m.height === h)?.refresh;
    if (r && !r.includes(config.refresh)) config.refresh = r[0];
    touch();
  }

  async function preset(name: string) {
    if (!config) return;
    config = await videoPreset($state.snapshot(config) as VideoConfig, name);
    touch();
  }

  async function save() {
    if (!config) return;
    try {
      await saveVideo($state.snapshot(config) as VideoConfig);
      dirty = false;
      status = { kind: "ok", text: "Saved to video.ini. Restart the game for it to take effect." };
    } catch (e) { status = { kind: "err", text: String(e) }; }
  }

  const levels = ["Off", "1", "2", "3", "4", "5"];
  onMount(load);
</script>

<div class="head">
  <div><h2 class="page-title">Video</h2><p class="muted">Display mode and graphics quality.</p></div>
  <div class="actions"><button class="btn" onclick={load}>Reload</button><button class="btn" class:primary={dirty} onclick={save} disabled={!config}>Save</button></div>
</div>
{#if status}<p class="msg {status.kind} gap">{status.text}</p>{/if}

{#if config}
  <div class="stack">
    <div class="card">
      <span class="eyebrow">Display</span>
      <div class="grid3">
        <label class="field">Resolution
          <select value={`${config.width}x${config.height}`} onchange={(e) => setResolution(e.currentTarget.value)}>
            {#each resolutions as r}<option value={r}>{r.replace("x", " × ")}</option>{/each}
          </select></label>
        <label class="field">Refresh rate
          <select bind:value={config.refresh} onchange={touch}>{#each rates as r}<option value={r}>{r} Hz</option>{/each}</select></label>
        <label class="field">Frame limit
          <select bind:value={config.fpsCap} onchange={touch}>
            <option value={0}>Unlimited</option>
            {#each [30, 60, 75, 90, 120, 144, 165, 240] as f}<option value={f}>{f} FPS</option>{/each}
          </select></label>
      </div>
      <label class="srow switch"><span class="what">Vertical sync<small>Prevents tearing, can add input lag</small></span><input type="checkbox" bind:checked={config.vsync} onchange={touch} /></label>
      <label class="srow switch"><span class="what">Fullscreen<small>Use gamescope in Settings if it appears stretched on Wayland</small></span><input type="checkbox" bind:checked={config.fullscreen} onchange={touch} /></label>
    </div>

    <div class="card">
      <span class="eyebrow">Quality preset</span>
      <div class="chips">
        {#each [["low", "Low"], ["medium", "Medium"], ["high", "High"], ["ultra", "Ultra"]] as [id, label]}
          <button class="chip" onclick={() => preset(id)}>{label}</button>
        {/each}
      </div>
      <p class="small">Sets the graphics options below. Resolution, fullscreen, V-sync and frame limit stay as they are. Press Save to apply.</p>
    </div>

    <div class="card">
      <span class="eyebrow">Graphics</span>
      <div class="grid3">
        <label class="field">Anti-aliasing (MSAA)
          <select bind:value={config.aaSamples} onchange={touch}>
            {#each [0, 2, 4, 8] as v}<option value={v}>{v === 0 ? "Off" : `${v}×`}</option>{/each}</select></label>
        <label class="field">Anisotropic filtering
          <select bind:value={config.anisotropic} onchange={touch}>
            {#each [0, 2, 4, 8, 16] as v}<option value={v}>{v === 0 ? "Off" : `${v}×`}</option>{/each}</select></label>
        <label class="field">Shadow resolution
          <select bind:value={config.shadowMapSize} onchange={touch}>
            {#each [512, 1024, 2048, 4096, 8192] as v}<option value={v}>{v}</option>{/each}</select></label>
        <label class="field">Reflection size
          <select bind:value={config.cubemapSize} onchange={touch}>
            {#each [128, 256, 512, 1024, 2048] as v}<option value={v}>{v}</option>{/each}</select></label>
        <label class="field">Mirror resolution
          <select bind:value={config.mirrorSize} onchange={touch}>
            {#each [128, 256, 512, 1024, 2048] as v}<option value={v}>{v}</option>{/each}</select></label>
        <label class="field">Smoke
          <select bind:value={config.smoke} onchange={touch}>{#each levels as l, i}<option value={i}>{l}</option>{/each}</select></label>
      </div>
      <div class="grid2">
        <label class="field">World detail <b>{config.worldDetail}</b>
          <input type="range" min="0" max="5" bind:value={config.worldDetail} oninput={touch} /></label>
        <label class="field">Reflection distance <b>{config.cubemapFarplane} m</b>
          <input type="range" min="100" max="2000" step="50" bind:value={config.cubemapFarplane} oninput={touch} /></label>
      </div>
      <label class="srow switch"><span>High-quality mirrors</span><input type="checkbox" bind:checked={config.mirrorHq} onchange={touch} /></label>
      <label class="srow switch"><span>Smoke in mirrors</span><input type="checkbox" bind:checked={config.smokeInMirror} onchange={touch} /></label>
    </div>

    <div class="card">
      <span class="eyebrow">Effects</span>
      <div class="grid3">
        <label class="field">Effects quality
          <select bind:value={config.ppQuality} onchange={touch}>{#each levels as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Glare
          <select bind:value={config.glare} onchange={touch}>{#each levels as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Depth of field
          <select bind:value={config.dof} onchange={touch}>{#each levels as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Motion blur
          <select bind:value={config.motionBlur} onchange={touch}>{#each levels as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Colour filter
          <select bind:value={config.ppFilter} onchange={touch}>
            {#each filters as f}<option value={f}>{f}</option>{/each}
            {#if !filters.includes(config.ppFilter)}<option value={config.ppFilter}>{config.ppFilter}</option>{/if}
          </select></label>
        <label class="field">Saturation <b>{config.saturation}%</b>
          <input type="range" min="0" max="200" bind:value={config.saturation} oninput={touch} /></label>
      </div>
      <label class="srow switch"><span>Post-processing</span><input type="checkbox" bind:checked={config.ppEnabled} onchange={touch} /></label>
      <label class="srow switch"><span>FXAA</span><input type="checkbox" bind:checked={config.fxaa} onchange={touch} /></label>
      <label class="srow switch"><span>Heat shimmer</span><input type="checkbox" bind:checked={config.heatShimmer} onchange={touch} /></label>
      <label class="srow switch"><span>Light rays</span><input type="checkbox" bind:checked={config.raysOfGod} onchange={touch} /></label>
    </div>

    <div class="card">
      <span class="eyebrow">Cockpit</span>
      <label class="srow switch"><span>Hide driver arms</span><input type="checkbox" bind:checked={config.hideArms} onchange={touch} /></label>
      <label class="srow switch"><span>Hide steering wheel</span><input type="checkbox" bind:checked={config.hideSteer} onchange={touch} /></label>
    </div>
  </div>
{:else}
  <p class="muted">Loading…</p>
{/if}

<style>
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 20px; margin-bottom: 26px; }
  .head p { margin-top: 4px; }
  .actions { display: flex; gap: 8px; }
  .gap { margin-bottom: 16px; }
  .grid3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 18px 20px; margin-bottom: 8px; }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin: 18px 0 4px; }
  .chips { margin-bottom: 14px; }
  @media (max-width: 1000px) { .grid3 { grid-template-columns: 1fr 1fr; } .grid2 { grid-template-columns: 1fr; } }
</style>
