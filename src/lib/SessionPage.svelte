<script lang="ts">
  // Session options: weekend format, weather/time, opponents, rules and assists.
  import type { AssistPreset, Assists, Car, SessionKind, SessionOptions, Weather } from "./api";

  let { options = $bindable(), weather, presets, cars, maxAi }: {
    options: SessionOptions; weather: Weather[]; presets: AssistPreset[]; cars: Car[]; maxAi: number;
  } = $props();

  const kinds: { kind: SessionKind; label: string; hint: string }[] = [
    { kind: "practice", label: "Practice", hint: "Start from the pits. 0 min = unlimited." },
    { kind: "qualify", label: "Qualify", hint: "Timed, from the pits." },
    { kind: "race", label: "Race", hint: "Standing start on the grid." },
  ];
  const has = (k: SessionKind) => options.sessions.some((s) => s.kind === k);
  const get = (k: SessionKind) => options.sessions.find((s) => s.kind === k)!;

  function toggle(k: SessionKind) {
    if (has(k)) {
      if (options.sessions.length === 1) return; // keep at least one session
      options.sessions = options.sessions.filter((s) => s.kind !== k);
    } else {
      const spec = { kind: k, minutes: k === "qualify" ? 10 : 0, laps: k === "race" ? 5 : 0 };
      // Fixed weekend order: practice → qualify → race
      options.sessions = [...options.sessions, spec].sort((a, b) => kinds.findIndex((x) => x.kind === a.kind) - kinds.findIndex((x) => x.kind === b.kind));
    }
  }

  const clock = (h: number) => `${String(Math.floor(h)).padStart(2, "0")}:${String(Math.round((h % 1) * 60)).padStart(2, "0")}`;
  const aiCount = $derived(Math.min(options.ai.count, maxAi));
  const level = ["Off", "Factory", "On"];

  function applyPreset(p: AssistPreset) {
    options.assists = { ...p.assists };
    options.assistPreset = p.id;
  }
  /** Edit one assist; the preset badge flips to "custom". */
  function setAssist<K extends keyof Assists>(key: K, value: Assists[K]) {
    options.assists = { ...options.assists, [key]: value };
    options.assistPreset = "custom";
  }
  const toggles: [keyof Assists, string][] = [
    ["idealLine", "Racing line"], ["autoShifter", "Automatic gearbox"], ["autoClutch", "Auto clutch"],
    ["autoBlip", "Auto blip"], ["autoBrake", "Auto brake"], ["tyreBlankets", "Tyre blankets"],
  ];
</script>

<div class="page-scroll"><div class="page-wrap">
  <div class="page-head"><div><h1>Session</h1><p>Format, conditions, opponents and driving aids for the next race.</p></div></div>
  <div class="cols">

  <section class="card full">
    <h2 class="eyebrow">Weekend</h2>
    <div class="cards">
      {#each kinds as k}
        <div class="opt" class:on={has(k.kind)}>
          <button class="head" onclick={() => toggle(k.kind)}>
            <span class="check">{has(k.kind) ? "✓" : ""}</span><strong>{k.label}</strong>
          </button>
          <small>{k.hint}</small>
          {#if has(k.kind)}
            {@const spec = get(k.kind)}
            {#if k.kind === "race"}
              <label class="inline">Laps <input type="number" min="1" max="99" bind:value={spec.laps} /></label>
            {:else}
              <label class="inline">Minutes <input type="number" min={k.kind === "qualify" ? 1 : 0} max="180" bind:value={spec.minutes} /></label>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  </section>

  <section class="card">
    <h2 class="eyebrow">Weather & time</h2>
    <div class="chips">
      {#each weather as w}
        <button class="chip" class:on={options.weather.id === w.id} onclick={() => (options.weather.id = w.id)}>{w.name}</button>
      {/each}
    </div>
    <div class="sliders">
      <label class="field">Time of day <b>{clock(options.weather.hour)}</b>
        <input type="range" min="8" max="18" step="0.25" bind:value={options.weather.hour} /></label>
      <label class="field">Ambient temperature <b>{options.weather.ambient}°C</b>
        <input type="range" min="0" max="36" bind:value={options.weather.ambient} /></label>
      <label class="field">Clock speed
        <select bind:value={options.weather.timeMult}>
          {#each [1, 2, 5, 10, 30, 60] as m}<option value={m}>{m === 1 ? "Real time" : `${m}×`}</option>{/each}
        </select></label>
    </div>
  </section>

  <section class="card">
    <h2 class="eyebrow">Opponents</h2>
    <div class="sliders">
      <label class="field">AI cars <b>{aiCount}</b> <small>(max {maxAi} on this layout)</small>
        <input type="range" min="0" max={Math.max(maxAi, 0)} bind:value={options.ai.count} /></label>
      <label class="field">Skill <b>{options.ai.strength}%</b>
        <input type="range" min="30" max="100" bind:value={options.ai.strength} /></label>
      <label class="field">Aggression <b>{options.ai.aggression}%</b>
        <input type="range" min="0" max="100" bind:value={options.ai.aggression} /></label>
      <label class="field">Opponent car
        <select value={options.ai.cars[0] ?? ""} onchange={(e) => (options.ai.cars = e.currentTarget.value ? [e.currentTarget.value] : [])}>
          <option value="">Same as mine</option>
          {#each cars as c}<option value={c.id}>{c.name}</option>{/each}
        </select></label>
    </div>
  </section>

  <section class="card full">
    <h2 class="eyebrow">Rules</h2>
    <div class="sliders">
      <label class="srow switch"><span class="what">Penalties<small>Track cutting and jump starts</small></span><input type="checkbox" bind:checked={options.penalties} /></label>
      {#if has("race")}
        <label class="field">Grid position <b>{Math.min(options.startPosition, aiCount + 1)}</b>
          <input type="range" min="1" max={aiCount + 1} bind:value={options.startPosition} /></label>
      {/if}
    </div>
  </section>

  <section class="card full">
    <h2 class="eyebrow">Assists</h2>
    <div class="chips">
      {#each presets as p}
        <button class="chip" class:on={options.assistPreset === p.id} onclick={() => applyPreset(p)}>{p.name}</button>
      {/each}
      <button class="chip" class:on={options.assistPreset === "custom"} disabled>Custom</button>
    </div>
    <div class="grid2">
      <div class="tg">
        {#each toggles as [key, label]}
          <label class="srow switch"><span>{label}</span><input type="checkbox" checked={options.assists[key] as boolean} onchange={(e) => setAssist(key, e.currentTarget.checked as never)} /></label>
        {/each}
      </div>
      <div class="sliders">
        <label class="field">ABS
          <select value={options.assists.abs} onchange={(e) => setAssist("abs", +e.currentTarget.value)}>{#each level as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Traction control
          <select value={options.assists.tractionControl} onchange={(e) => setAssist("tractionControl", +e.currentTarget.value)}>{#each level as l, i}<option value={i}>{l}</option>{/each}</select></label>
        <label class="field">Stability control <b>{options.assists.stabilityControl}%</b>
          <input type="range" min="0" max="100" value={options.assists.stabilityControl} oninput={(e) => setAssist("stabilityControl", +e.currentTarget.value)} /></label>
        <label class="field">Mechanical damage <b>{options.assists.damage}%</b>
          <input type="range" min="0" max="100" value={options.assists.damage} oninput={(e) => setAssist("damage", +e.currentTarget.value)} /></label>
        <label class="field">Fuel consumption
          <select value={options.assists.fuelRate} onchange={(e) => setAssist("fuelRate", +e.currentTarget.value)}><option value={0}>Off</option><option value={1}>Normal</option><option value={2}>2×</option></select></label>
        <label class="field">Tyre wear
          <select value={options.assists.tyreWear} onchange={(e) => setAssist("tyreWear", +e.currentTarget.value)}><option value={0}>Off</option><option value={1}>Normal</option><option value={2}>2×</option></select></label>
      </div>
    </div>
  </section>
  </div>
</div></div>

<style>
  .cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
  .opt { border: 1px solid var(--line-2); border-radius: 14px; padding: 16px; display: flex; flex-direction: column; gap: 10px; transition: border-color 0.2s, background 0.2s; }
  .opt.on { border-color: #fff; background: #0f0f0f; }
  .head { display: flex; align-items: center; gap: 12px; text-align: left; }
  .head strong { font-size: 16px; font-weight: 500; }
  .check { width: 20px; height: 20px; border-radius: 50%; border: 1.5px solid var(--fg-3); display: grid; place-items: center; font-size: 11px; color: #000; transition: all 0.2s; }
  .opt.on .check { background: #fff; border-color: #fff; }
  .inline { display: flex; align-items: center; gap: 10px; color: var(--fg-3); font-size: 12px; }
  .inline input { width: 84px; }
  .chips { margin-bottom: 20px; }
  .sliders { display: grid; gap: 18px; }
  .grid2 { display: grid; grid-template-columns: 240px 1fr; gap: 40px; }
  .tg { display: flex; flex-direction: column; align-content: start; }
  .tg .srow:first-child { border-top: 0; }
  @media (max-width: 1100px) { .cards { grid-template-columns: 1fr; } .grid2 { grid-template-columns: 1fr; } }
</style>
