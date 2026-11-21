<script lang="ts">
  // General settings: game paths, Proton, and how the launcher and game start.
  // Empty path fields mean "auto-detect".
  import { saveSettings, type AppState, type Settings } from "./api";

  let { app, onsaved }: { app: AppState | null; onsaved: (s: AppState) => void } = $props();

  let form = $state<Settings>({ gameDir: null, compatDataDir: null, protonDir: null, steamRoot: null, wrapper: "", extraEnv: "", windowedLauncher: false, useGamescope: false, gamescopeArgs: "", stayOpenOnLaunch: false });
  let status = $state("");

  // Fill the form from stored settings once they arrive.
  $effect(() => { if (app) form = { ...app.settings }; });

  const clean = (v: string | null) => (v && v.trim() ? v.trim() : null);

  async function save() {
    status = "";
    try {
      const next = await saveSettings({
        ...form,
        gameDir: clean(form.gameDir), compatDataDir: clean(form.compatDataDir),
        protonDir: clean(form.protonDir), steamRoot: clean(form.steamRoot),
      });
      status = "Saved";
      onsaved(next);
    } catch (e) { status = String(e); }
  }
  const r = $derived(app?.resolved);
</script>

<div class="head">
  <div><h2 class="page-title">General</h2><p class="muted">Leave a path empty to detect it automatically. Detected values are shown as placeholders.</p></div>
  <div class="actions"><span class="small">{status}</span><button class="btn primary" onclick={save}>Save</button></div>
</div>

<div class="stack">
  {#if r?.issues.length}
    <div class="stack tight">
      {#each r.issues as i}<p class="msg {i.level === 'error' ? 'err' : ''}">{i.message}</p>{/each}
    </div>
  {/if}

  <div class="card">
    <span class="eyebrow">Locations</span>
    <label class="field">Game folder <input bind:value={form.gameDir} placeholder={r?.gameDir ?? "…/steamapps/common/assettocorsa"} /></label>
    <label class="field">Proton prefix <input bind:value={form.compatDataDir} placeholder={r?.compatDataDir ?? "…/steamapps/compatdata/244210"} /></label>
    <label class="field">Steam folder <input bind:value={form.steamRoot} placeholder={r?.steamRoot ?? "~/.local/share/Steam"} /></label>
    <label class="field">Proton version
      <select bind:value={form.protonDir}>
        <option value={null}>Automatic ({r?.protonDir?.split("/").pop() ?? "none found"})</option>
        {#each r?.protonCandidates ?? [] as p}<option value={p.dir}>{p.name}</option>{/each}
      </select>
    </label>
  </div>

  <div class="card">
    <span class="eyebrow">Launching the game</span>
    <label class="srow switch"><span class="what">Run inside gamescope<small>Fixes stretched or unclickable fullscreen on Wayland</small></span><input type="checkbox" bind:checked={form.useGamescope} /></label>
    {#if form.useGamescope}
      <label class="field sub">gamescope arguments <input bind:value={form.gamescopeArgs} placeholder="-W 1920 -H 1080 -r 75 -f" /></label>
    {/if}
    <label class="srow switch"><span class="what">Keep launcher visible while playing<small>Otherwise it hides and returns when the game closes</small></span><input type="checkbox" bind:checked={form.stayOpenOnLaunch} /></label>
    <label class="srow switch"><span class="what">Open launcher in a window<small>Instead of fullscreen. F11 toggles anytime</small></span><input type="checkbox" bind:checked={form.windowedLauncher} /></label>
    <label class="field">Command wrapper <input bind:value={form.wrapper} placeholder="gamemoderun" /></label>
    <label class="field">Extra environment <textarea rows="3" bind:value={form.extraEnv} placeholder="DXVK_HUD=fps  (one KEY=VALUE per line)"></textarea></label>
  </div>
</div>

<style>
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 20px; margin-bottom: 26px; }
  .head p { margin-top: 4px; }
  .actions { display: flex; align-items: center; gap: 14px; }
  .tight { gap: 8px; }
  .card { display: flex; flex-direction: column; gap: 16px; }
  .card > .srow { padding: 4px 0; border: 0; }
  .field.sub { padding-left: 2px; }
  .card .eyebrow { margin-bottom: 2px; }
</style>
