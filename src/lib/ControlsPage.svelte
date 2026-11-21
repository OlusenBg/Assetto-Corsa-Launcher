<script lang="ts">
  // Controls: detect devices, bind axes/buttons by moving them, tune wheel + force feedback.
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    listInputDevices, startInputMonitor, stopInputMonitor, readControls, saveControls, g923Preset,
    type AxisBinding, type ControlsConfig, type InputDevice, type InputSnapshot, type WheelTuning,
  } from "./api";

  type AxisKey = "steer" | "throttle" | "brakes" | "clutch";
  type Target = { kind: "axis"; key: AxisKey } | { kind: "button"; action: string };

  let devices = $state<InputDevice[]>([]);
  let unreadable = $state(0);
  let config = $state<ControlsConfig | null>(null);
  let snaps = $state<Record<string, InputSnapshot>>({});
  let status = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  let dirty = $state(false);

  // Binding in progress: what we are waiting for, and the device state when it started.
  let target = $state<Target | null>(null);
  let baseline: Record<string, InputSnapshot> = {};
  let timeout: ReturnType<typeof setTimeout> | undefined;

  const axisRows: { key: AxisKey; label: string; hint: string }[] = [
    { key: "steer", label: "Steering", hint: "Turn the wheel" },
    { key: "throttle", label: "Throttle", hint: "Press the throttle pedal" },
    { key: "brakes", label: "Brake", hint: "Press the brake pedal" },
    { key: "clutch", label: "Clutch", hint: "Press the clutch pedal" },
  ];

  async function refresh() {
    const d = await listInputDevices();
    devices = d.devices;
    unreadable = d.unreadable;
    config = await readControls();
    dirty = false;
  }

  /** Detected device that a `JOY=n` index in the config refers to. */
  function deviceFor(joy: number): InputDevice | undefined {
    const ref = config?.devices[joy];
    return ref ? devices.find((d) => d.pguid === ref.pguid || d.name === ref.name) : undefined;
  }

  /** Index of `d` in the config's device list, adding it if needed. */
  function ensureDevice(d: InputDevice): number {
    const c = config!;
    let i = c.devices.findIndex((x) => x.pguid === d.pguid);
    if (i < 0) { c.devices.push({ name: d.name, pguid: d.pguid }); i = c.devices.length - 1; }
    return i;
  }

  function norm(d: InputDevice, axle: number, raw: number | undefined): number | null {
    const a = d.axes.find((x) => x.index === axle);
    if (!a || raw === undefined || a.max === a.min) return null;
    return (raw - a.min) / (a.max - a.min); // 0..1
  }

  /** Live 0..1 position of a bound axis (null if not available). */
  function live(b: AxisBinding): number | null {
    const d = deviceFor(b.joy);
    if (!d) return null;
    const raw = snaps[d.path]?.axes[String(b.axle)] ?? d.axes.find((a) => a.index === b.axle)?.value;
    return norm(d, b.axle, raw);
  }

  function begin(t: Target) {
    cancel();
    baseline = JSON.parse(JSON.stringify(snaps));
    // Devices that have not produced events yet: use the values read at list time.
    for (const d of devices) {
      baseline[d.path] ??= { path: d.path, axes: Object.fromEntries(d.axes.map((a) => [String(a.index), a.value])), buttons: [] };
    }
    target = t;
    timeout = setTimeout(cancel, 12000);
  }
  function cancel() { target = null; clearTimeout(timeout); }

  function onSnapshot(s: InputSnapshot) {
    snaps[s.path] = s;
    const t = target;
    const d = devices.find((x) => x.path === s.path);
    if (!t || !d || !config) return;
    const base = baseline[s.path];
    if (t.kind === "axis") {
      let best: { index: number; delta: number; start: number } | null = null;
      for (const a of d.axes) {
        const now = norm(d, a.index, s.axes[String(a.index)]);
        const was = norm(d, a.index, base?.axes[String(a.index)]);
        if (now === null || was === null) continue;
        const delta = Math.abs(now - was);
        if (delta > 0.3 && (!best || delta > best.delta)) best = { index: a.index, delta, start: was };
      }
      if (best) {
        const b = config[t.key];
        b.joy = ensureDevice(d);
        b.axle = best.index;
        if (t.key !== "steer") b.invert = best.start > 0.5; // pedal rests high => invert
        dirty = true;
        cancel();
      }
    } else {
      const fresh = s.buttons.find((i) => !(base?.buttons ?? []).includes(i));
      if (fresh !== undefined) {
        const b = config.buttons.find((x) => x.action === t.action)!;
        b.joy = ensureDevice(d);
        b.button = fresh;
        dirty = true;
        cancel();
      }
    }
  }

  async function save() {
    if (!config) return;
    try {
      await saveControls($state.snapshot(config) as ControlsConfig);
      dirty = false;
      status = { kind: "ok", text: "Saved to controls.ini. Restart the game for it to take effect." };
    } catch (e) { status = { kind: "err", text: String(e) }; }
  }

  async function preset() {
    config = await g923Preset();
    dirty = true;
    status = { kind: "ok", text: "G923 preset loaded. Press Save to write it." };
  }

  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const sliders: { key: keyof WheelTuning; label: string; min: number; max: number; step: number; fmt?: (v: number) => string; group: "wheel" | "ff" }[] = [
    { key: "lock", label: "Wheel rotation", min: 180, max: 1080, step: 10, fmt: (v) => `${v}°`, group: "wheel" },
    { key: "steerGamma", label: "Steering linearity", min: 0.5, max: 2, step: 0.05, group: "wheel" },
    { key: "steerFilter", label: "Steering filter", min: 0, max: 1, step: 0.05, fmt: pct, group: "wheel" },
    { key: "speedSensitivity", label: "Speed sensitivity", min: 0, max: 1, step: 0.05, fmt: pct, group: "wheel" },
    { key: "ffGain", label: "Force feedback strength", min: 0, max: 2, step: 0.05, fmt: pct, group: "ff" },
    { key: "filterFf", label: "Force feedback filter", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "minFf", label: "Minimum force", min: 0, max: 0.3, step: 0.01, fmt: pct, group: "ff" },
    { key: "centerBoostGain", label: "Center boost", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "curbs", label: "Curb effect", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "road", label: "Road effect", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "slips", label: "Slip effect", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "abs", label: "ABS vibration", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
    { key: "understeer", label: "Understeer effect", min: 0, max: 1, step: 0.05, fmt: pct, group: "ff" },
  ];

  /** Buttons >= 128 are D-pad (hat) directions: 128 + 4*hat + (up, right, down, left). */
  function buttonName(button: number) {
    if (button < 128) return `Button ${button}`;
    const hat = Math.floor((button - 128) / 4);
    return `D-pad${hat ? ` ${hat + 1}` : ""} ${["up", "right", "down", "left"][(button - 128) % 4]}`;
  }
  function bindingLabel(joy: number, button: number) {
    if (joy < 0 || button < 0) return "Not bound";
    const d = config?.devices[joy];
    return `${buttonName(button)}${(config?.devices.length ?? 0) > 1 && d ? ` · ${d.name}` : ""}`;
  }
  function pressed(joy: number, button: number): boolean {
    const d = deviceFor(joy);
    return !!d && (snaps[d.path]?.buttons ?? []).includes(button);
  }
  function axisLabel(b: AxisBinding) {
    const d = deviceFor(b.joy);
    if (b.joy < 0 || b.axle < 0) return "Not bound";
    const name = d?.axes.find((a) => a.index === b.axle)?.name ?? "";
    return `Axis ${b.axle}${name ? ` (${name})` : ""}${d ? ` · ${d.name}` : config?.devices[b.joy] ? " · device not connected" : ""}`;
  }

  onMount(() => {
    refresh().then(startInputMonitor).catch((e) => (status = { kind: "err", text: String(e) }));
    const un = listen<InputSnapshot>("input-state", (e) => onSnapshot(e.payload));
    const key = (e: KeyboardEvent) => { if (e.key === "Escape") cancel(); };
    window.addEventListener("keydown", key);
    return () => { un.then((f) => f()); stopInputMonitor(); window.removeEventListener("keydown", key); clearTimeout(timeout); };
  });
</script>

<div class="head">
  <div><h2 class="page-title">Controls</h2><p class="muted">Bind your wheel, pedals and buttons by moving them.</p></div>
  <div class="actions">
    <button class="btn" onclick={preset} title="Load the measured Logitech G923 bindings">G923 preset</button>
    <button class="btn" onclick={refresh}>Reload</button>
    <button class="btn" class:primary={dirty} onclick={save} disabled={!config}>Save</button>
  </div>
</div>
{#if status}<p class="msg {status.kind} gap">{status.text}</p>{/if}
{#if target}<p class="msg waiting gap"><i class="pulse"></i>{target.kind === "axis" ? "Move the control fully" : "Press the button"} · Esc to cancel</p>{/if}

{#if config}
  <div class="stack">
    <div class="card">
      <span class="eyebrow">Input method</span>
      <div class="chips">
        {#each [["WHEEL", "Steering wheel"], ["X360", "Gamepad"], ["KEYBOARD", "Keyboard"]] as [v, l]}
          <button class="chip" class:on={config.inputMethod === v} onclick={() => { config!.inputMethod = v as ControlsConfig["inputMethod"]; dirty = true; }}>{l}</button>
        {/each}
      </div>
    </div>

    <div class="card">
      <span class="eyebrow">Detected devices</span>
      {#each devices as d (d.path)}
        <div class="dev">
          <div class="devhead">
            <strong>{d.name}</strong>
            <small>{d.vendor.toString(16).padStart(4, "0")}:{d.product.toString(16).padStart(4, "0")} · {d.axes.length} axes · {d.buttons.length} buttons{d.hats ? ` · ${d.hats} D-pad` : ""}
              {#if config.devices.some((x) => x.pguid === d.pguid)}<span class="tag">in use</span>{/if}</small>
          </div>
          <div class="axes">
            {#each d.axes as a}
              {@const n = norm(d, a.index, snaps[d.path]?.axes[String(a.index)] ?? a.value) ?? 0}
              <div class="ax"><span>{a.name}</span><div class="bar"><i style="width:{n * 100}%"></i></div></div>
            {/each}
          </div>
        </div>
      {:else}
        <p class="muted">
          No wheel or joystick found.
          {#if unreadable > 0}{unreadable} input devices could not be opened: add yourself to the <code>input</code> group.{/if}
        </p>
      {/each}
    </div>

    {#if config.inputMethod === "WHEEL"}
      <div class="card">
        <span class="eyebrow">Axes</span>
        {#each axisRows as r}
          {@const b = config[r.key]}
          {@const v = live(b)}
          <div class="arow">
            <div class="lab"><strong>{r.label}</strong><small>{axisLabel(b)}</small></div>
            <div class="bar wide">
              {#if v !== null}<i style="width:{(r.key !== "steer" && b.invert ? 1 - v : v) * 100}%"></i>{/if}
            </div>
            {#if r.key !== "steer"}
              <label class="inv"><span>Invert</span><input type="checkbox" bind:checked={b.invert} onchange={() => (dirty = true)} /></label>
            {:else}<span></span>{/if}
            <button class="btn sm" class:primary={target?.kind === "axis" && target.key === r.key} onclick={() => begin({ kind: "axis", key: r.key })}>
              {target?.kind === "axis" && target.key === r.key ? "Listening…" : "Bind"}
            </button>
          </div>
        {/each}
      </div>

      <div class="cols">
        <div class="card">
          <span class="eyebrow">Steering</span>
          {#each sliders.filter((s) => s.group === "wheel") as s}
            <label class="field sl">{s.label} <b>{(s.fmt ?? String)(config.tuning[s.key])}</b>
              <input type="range" min={s.min} max={s.max} step={s.step} bind:value={config.tuning[s.key]} oninput={() => (dirty = true)} /></label>
          {/each}
          {#if config.brakes.gamma !== null}
            <label class="field sl">Brake curve <b>{config.brakes.gamma}</b>
              <input type="range" min="1" max="3" step="0.1" bind:value={config.brakes.gamma} oninput={() => (dirty = true)} /></label>
          {/if}
        </div>
        <div class="card">
          <span class="eyebrow">Force feedback</span>
          {#each sliders.filter((s) => s.group === "ff") as s}
            <label class="field sl">{s.label} <b>{(s.fmt ?? String)(config.tuning[s.key])}</b>
              <input type="range" min={s.min} max={s.max} step={s.step} bind:value={config.tuning[s.key]} oninput={() => (dirty = true)} /></label>
          {/each}
        </div>
      </div>
    {/if}

    {#if config.inputMethod !== "KEYBOARD"}
      <div class="card">
        <span class="eyebrow">Buttons</span>
        <div class="btns">
          {#each config.buttons as b (b.action)}
            <div class="brow" class:hit={pressed(b.joy, b.button)}>
              <div class="lab"><span>{b.label}</span><small>{bindingLabel(b.joy, b.button)}</small></div>
              <button class="btn sm" class:primary={target?.kind === "button" && target.action === b.action} onclick={() => begin({ kind: "button", action: b.action })}>
                {target?.kind === "button" && target.action === b.action ? "Press…" : "Bind"}
              </button>
              <button class="btn sm icon" title="Clear" onclick={() => { b.joy = -1; b.button = -1; dirty = true; }}>✕</button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{:else}
  <p class="muted">Loading…</p>
{/if}

<style>
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 20px; margin-bottom: 26px; }
  .head p { margin-top: 4px; }
  .actions { display: flex; gap: 8px; }
  .gap { margin-bottom: 16px; }
  .waiting { display: flex; align-items: center; gap: 10px; border-color: #fff; }
  .pulse { width: 8px; height: 8px; border-radius: 50%; background: #fff; animation: pulse 1s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.2; transform: scale(0.7); } }
  .chips { margin-top: 0; }
  .dev { border: 1px solid var(--line); border-radius: 12px; padding: 14px 16px; margin-bottom: 10px; }
  .devhead { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 10px; }
  .devhead strong { font-weight: 500; }
  .tag { margin-left: 8px; border: 1px solid var(--line-2); color: var(--fg-2); padding: 0 8px; border-radius: 99px; font-size: 11px; }
  .axes { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 8px 18px; }
  .ax { display: flex; align-items: center; gap: 10px; font-size: 12px; color: var(--fg-3); }
  .ax span { width: 28px; }
  .bar { flex: 1; height: 4px; border-radius: 4px; background: #1d1d1d; overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; border-radius: 4px; }
  .bar.wide { height: 6px; min-width: 120px; }
  .arow { display: grid; grid-template-columns: 240px 1fr 100px 96px; gap: 20px; align-items: center; padding: 14px 0; border-top: 1px solid var(--line); }
  .eyebrow + .arow { border-top: 0; padding-top: 4px; }
  .lab { display: flex; flex-direction: column; min-width: 0; }
  .lab strong { font-weight: 500; }
  .lab small { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .inv { display: flex; align-items: center; justify-content: space-between; gap: 10px; color: var(--fg-2); font-size: 12.5px; cursor: pointer; }
  .sl { margin-bottom: 16px; }
  .btns { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 28px; }
  .brow { display: grid; grid-template-columns: 1fr auto auto; gap: 8px; align-items: center; padding: 8px 12px; border-radius: 12px; border: 1px solid transparent; transition: background 0.15s, border-color 0.15s; }
  .brow.hit { background: #fff; color: #000; }
  .brow.hit small { color: #444; }
  @media (max-width: 1000px) { .btns { grid-template-columns: 1fr; } .arow { grid-template-columns: 1fr auto; } .bar.wide { display: none; } }
</style>
