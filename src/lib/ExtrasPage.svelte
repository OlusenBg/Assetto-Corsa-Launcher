<script lang="ts">
  // Extras: replays, setups and installing cars/tracks/skins from archives.
  import { onMount, untrack } from "svelte";
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import {
    listReplays, keepReplay, deleteReplay, listSetups, importSetup, exportSetup, adoptSetup, deleteSetup,
    modInspect, modInstall, modDiscard,
    type Car, type ModPlan, type Replay, type SetupFile, type Track,
  } from "./api";

  let { cars, tracks, carId, running, launchMsg, initialTab = "replays", onchanged, onplay }: {
    cars: Car[]; tracks: Track[]; carId: string; running: boolean;
    launchMsg: { kind: "ok" | "err"; text: string } | null; initialTab?: "replays" | "setups" | "install";
    onchanged: () => void; onplay: (name: string, temp: boolean) => void;
  } = $props();

  let tab = $state<"replays" | "setups" | "install">(untrack(() => initialTab));
  let status = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  const ok = (text: string) => (status = { kind: "ok", text });
  const err = (e: unknown) => (status = { kind: "err", text: String(e) });

  const carName = (id?: string) => cars.find((c) => c.id === id)?.name ?? id ?? "Unknown car";
  const trackName = (id?: string) => tracks.find((t) => t.id === id)?.name ?? id ?? "Unknown track";
  const when = (s: number) => new Date(s * 1000).toLocaleString();
  const mb = (n: number) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);

  // ------------------------------------------------------------- replays
  let replays = $state<Replay[]>([]);
  async function loadReplays() { try { replays = await listReplays(); } catch (e) { err(e); } }
  async function keep(r: Replay) {
    try { const n = await keepReplay(r.name); ok(`Saved as ${n}`); await loadReplays(); } catch (e) { err(e); }
  }
  async function removeReplay(r: Replay) {
    if (!(await ask(`Delete this replay?\n${r.name}`, { title: "Delete replay", kind: "warning" }))) return;
    try { await deleteReplay(r.name, r.temp); await loadReplays(); } catch (e) { err(e); }
  }

  // -------------------------------------------------------------- setups
  let setupCar = $state(untrack(() => carId)); // starts on the car picked on the Drive page
  let setups = $state<SetupFile[]>([]);
  let importScope = $state("generic");
  async function loadSetups() { try { setups = await listSetups(setupCar || null); } catch (e) { err(e); } }
  async function doImport() {
    const file = await open({ multiple: false, filters: [{ name: "Setup", extensions: ["ini"] }] });
    if (typeof file !== "string") return;
    try {
      const s = await importSetup(file, setupCar || null, importScope);
      ok(`Imported "${s.name}" for ${carName(s.car)} (${s.scope})`);
      if (s.car !== setupCar) setupCar = s.car;
      await loadSetups();
    } catch (e) { err(e); }
  }
  async function doExport(s: SetupFile) {
    const dest = await save({ defaultPath: `${s.name}.ini`, filters: [{ name: "Setup", extensions: ["ini"] }] });
    if (!dest) return;
    try { await exportSetup(s.path, dest); ok(`Exported to ${dest}`); } catch (e) { err(e); }
  }
  async function adopt(s: SetupFile) {
    try { await adoptSetup(s.path); ok(`Copied "${s.name}" to your setups`); await loadSetups(); } catch (e) { err(e); }
  }
  async function removeSetup(s: SetupFile) {
    if (!(await ask(`Delete setup "${s.name}"?`, { title: "Delete setup", kind: "warning" }))) return;
    try { await deleteSetup(s.path); await loadSetups(); } catch (e) { err(e); }
  }

  // ------------------------------------------------------------- install
  let plan = $state<ModPlan | null>(null);
  let overwrite = $state(false);
  let skinCar = $state("");
  let report = $state<string[]>([]);
  let busy = $state(false);

  async function choose() {
    const file = await open({ multiple: false, filters: [{ name: "Archive", extensions: ["zip", "rar", "7z", "tar", "gz", "xz", "bz2", "tgz"] }] });
    if (typeof file !== "string") return;
    busy = true; report = []; status = null;
    if (plan) await modDiscard(plan.id);
    try { plan = await modInspect(file); skinCar = carId; } catch (e) { plan = null; err(e); }
    busy = false;
  }
  async function cancel() { if (plan) await modDiscard(plan.id); plan = null; }
  async function install() {
    if (!plan) return;
    busy = true;
    try {
      report = await modInstall(plan.id, overwrite, skinCar || null);
      plan = null;
      onchanged(); // rescan cars and tracks
    } catch (e) { err(e); }
    busy = false;
  }

  $effect(() => { if (tab === "setups") { setupCar; loadSetups(); } });
  onMount(loadReplays);
</script>

<div class="page-scroll"><div class="page-wrap">
  <div class="page-head"><div><h1>Extras</h1><p>Replays, car setups and installing new content.</p></div></div>
  <div class="tabs">
    {#each [["replays", "Replays"], ["setups", "Setups"], ["install", "Install content"]] as [id, label]}
      <button class:on={tab === id} onclick={() => { tab = id as typeof tab; status = null; }}>{label}</button>
    {/each}
  </div>
  {#if status}<p class="msg {status.kind} gap">{status.text}</p>{/if}
  {#if launchMsg && tab === "replays"}<p class="msg {launchMsg.kind} gap">{launchMsg.text}</p>{/if}

  {#if tab === "replays"}
    <section class="card">
      <div class="head"><span class="eyebrow">Recorded sessions</span><button class="btn sm" onclick={loadReplays}>Refresh</button></div>
      {#each replays as r (r.temp + r.name)}
        <div class="item">
          <div class="info">
            <strong>{r.car ? carName(r.car) : r.name}</strong>
            <small>{r.track ? `${trackName(r.track)}${r.layout ? ` · ${r.layout}` : ""} · ` : ""}{when(r.modified)} · {mb(r.size)}
              {#if r.temp}<span class="tag">last session</span>{/if}</small>
          </div>
          <button class="btn sm primary" disabled={running} onclick={() => onplay(r.name, r.temp)}>Play</button>
          {#if r.temp}<button class="btn sm" onclick={() => keep(r)} title="Keep a copy in your replays">Save</button>{/if}
          <button class="btn sm icon" title="Delete" onclick={() => removeReplay(r)}>✕</button>
        </div>
      {:else}
        <p class="empty">No replays yet. The game records your last session automatically.</p>
      {/each}
    </section>
  {/if}

  {#if tab === "setups"}
    <section class="card">
      <div class="head">
        <span class="eyebrow">Car setups</span>
        <div class="bar">
          <select bind:value={setupCar}>
            <option value="">All cars</option>
            {#each cars as c}<option value={c.id}>{c.name}</option>{/each}
          </select>
          <select bind:value={importScope} title="Track the imported setup is for">
            <option value="generic">Any track</option>
            {#each tracks as t}<option value={t.id}>{t.name}</option>{/each}
          </select>
          <button class="btn sm primary" onclick={doImport}>Import</button>
        </div>
      </div>
      {#each setups as s (s.path)}
        <div class="item">
          <div class="info">
            <strong>{s.name}</strong>
            <small>{carName(s.car)} · {s.scope === "generic" ? "any track" : trackName(s.scope)} · {s.builtin ? "ships with the game" : when(s.modified)} · {mb(s.size)}</small>
          </div>
          {#if s.builtin}<button class="btn sm" onclick={() => adopt(s)}>Copy to mine</button>{/if}
          <button class="btn sm" onclick={() => doExport(s)}>Export</button>
          {#if !s.builtin}<button class="btn sm icon" title="Delete" onclick={() => removeSetup(s)}>✕</button>{/if}
        </div>
      {:else}
        <p class="empty">No setups for this selection. Save one in the game's garage, or import a file.</p>
      {/each}
    </section>
  {/if}

  {#if tab === "install"}
    <section class="card">
      <div class="head"><span class="eyebrow">Install cars, tracks and skins</span>
        <button class="btn sm primary" disabled={busy} onclick={choose}>{busy ? "Working…" : "Choose archive"}</button></div>
      <p class="muted">Pick a .zip, .rar or .7z. The launcher looks inside, shows what it found, and only copies after you confirm.</p>

      {#if plan}
        <div class="plan">
          <strong>{plan.archive}</strong>
          {#each plan.warnings as w}<p class="msg err">{w}</p>{/each}
          {#each plan.items as it}
            <div class="item slim">
              <span class="kind">{it.kind}</span>
              <div class="info">
                <strong>{it.id}</strong>
                <small>{it.kind === "skin" ? (it.car ? `for ${carName(it.car)}` : "car not specified in the archive") : ""}{it.exists ? " · already installed" : " · new"}</small>
              </div>
            </div>
          {/each}
          {#if plan.items.some((i) => i.needsCar)}
            <label class="field">Install skins without a car into
              <select bind:value={skinCar}>{#each cars as c}<option value={c.id}>{c.name}</option>{/each}</select></label>
          {/if}
          {#if plan.items.some((i) => i.exists)}
            <label class="srow switch"><span>Overwrite what is already installed</span><input type="checkbox" bind:checked={overwrite} /></label>
          {/if}
          <div class="acts">
            <button class="btn primary" disabled={busy || plan.items.length === 0} onclick={install}>Install</button>
            <button class="btn" onclick={cancel}>Cancel</button>
          </div>
        </div>
      {/if}

      {#if report.length}
        <div class="plan">
          <strong>Result</strong>
          {#each report as line}<p class="line">{line}</p>{/each}
        </div>
      {/if}
    </section>
  {/if}
</div></div>

<style>
  .tabs { display: flex; gap: 28px; border-bottom: 1px solid var(--line); margin-bottom: 22px; }
  .tabs button { padding: 0 0 12px; color: var(--fg-3); font-weight: 500; border-bottom: 2px solid transparent; margin-bottom: -1px; transition: color 0.2s, border-color 0.2s; }
  .tabs button:hover { color: var(--fg); }
  .tabs button.on { color: var(--fg); border-bottom-color: #fff; }
  .gap { margin-bottom: 16px; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 8px; flex-wrap: wrap; }
  .bar { display: flex; gap: 8px; flex-wrap: wrap; }
  .bar select { max-width: 200px; }
  .item { display: flex; align-items: center; gap: 10px; padding: 14px 0; border-top: 1px solid var(--line); }
  .item.slim { padding: 8px 0; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .info strong { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tag { margin-left: 8px; border: 1px solid var(--line-2); color: var(--fg-2); padding: 0 8px; border-radius: 99px; font-size: 11px; }
  .empty { color: var(--fg-3); padding: 14px 0 6px; }
  .plan { margin-top: 18px; border: 1px solid var(--line-2); border-radius: 14px; padding: 16px 18px; display: flex; flex-direction: column; gap: 6px; }
  .kind { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.14em; border: 1px solid var(--line-2); color: var(--fg-2); padding: 2px 10px; border-radius: 99px; min-width: 58px; text-align: center; }
  .acts { display: flex; gap: 8px; margin-top: 10px; }
  .line { font-size: 13px; color: var(--fg-2); }
</style>
