<script lang="ts">
  // Audio: volume levels (writes audio.ini).
  import { onMount } from "svelte";
  import { readAudio, saveAudio, type AudioConfig } from "./api";

  let config = $state<AudioConfig | null>(null);
  let dirty = $state(false);
  let status = $state<{ kind: "ok" | "err"; text: string } | null>(null);

  const rows: { key: keyof AudioConfig; label: string; hint: string }[] = [
    { key: "master", label: "Master", hint: "Overall volume" },
    { key: "engine", label: "Engine", hint: "Your car's engine" },
    { key: "opponents", label: "Opponents", hint: "Other cars" },
    { key: "tyres", label: "Tyres", hint: "Squeal and grip noise" },
    { key: "surfaces", label: "Surfaces", hint: "Road, gravel, grass" },
    { key: "wind", label: "Wind", hint: "Air rushing past" },
    { key: "dirtBottom", label: "Underbody", hint: "Scraping and bottoming out" },
    { key: "uiSounds", label: "Interface", hint: "Menu and app sounds" },
    { key: "music", label: "Music", hint: "Menu music" },
  ];

  async function load() {
    try { config = await readAudio(); dirty = false; }
    catch (e) { status = { kind: "err", text: String(e) }; }
  }
  async function save() {
    if (!config) return;
    try {
      await saveAudio($state.snapshot(config) as AudioConfig);
      dirty = false;
      status = { kind: "ok", text: "Saved. Restart the game to apply." };
    } catch (e) { status = { kind: "err", text: String(e) }; }
  }
  onMount(load);
</script>

<div class="head">
  <div><h2 class="page-title">Audio</h2><p class="muted">Volume levels used by the game.</p></div>
  <div class="actions"><button class="btn" onclick={load}>Reload</button><button class="btn" class:primary={dirty} onclick={save} disabled={!config}>Save</button></div>
</div>
{#if status}<p class="msg {status.kind} gap">{status.text}</p>{/if}

{#if config}
  <div class="stack">
    <div class="card">
      <span class="eyebrow">Volume</span>
      {#each rows as r}
        <label class="slider">
          <span class="what"><strong>{r.label}</strong><small>{r.hint}</small></span>
          <input type="range" min="0" max="1" step="0.01" bind:value={config[r.key] as number} oninput={() => (dirty = true)} />
          <b>{Math.round((config[r.key] as number) * 100)}</b>
        </label>
      {/each}
    </div>
    <div class="card">
      <span class="eyebrow">Tyre squeal</span>
      <label class="slider">
        <span class="what"><strong>Threshold</strong><small>Higher means tyres squeal later, closer to the limit</small></span>
        <input type="range" min="0" max="100" bind:value={config.skidEntryPoint} oninput={() => (dirty = true)} />
        <b>{config.skidEntryPoint}</b>
      </label>
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
  .slider { display: grid; grid-template-columns: 220px 1fr 40px; gap: 20px; align-items: center; padding: 12px 0; border-top: 1px solid var(--line); }
  .eyebrow + .slider { border-top: 0; }
  .what { display: flex; flex-direction: column; }
  b { text-align: right; font-weight: 500; font-variant-numeric: tabular-nums; color: var(--fg); }
</style>
