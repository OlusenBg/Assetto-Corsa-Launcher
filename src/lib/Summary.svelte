<script lang="ts">
  // Right-hand panel: current selection, skin + layout pickers and the launch button.
  import { fly } from "svelte/transition";
  import { img, type Car, type Skin, type Track } from "./api";

  let { car, skins, skinId, track, layout, running, launchMsg, label, onskin, onlayout, onpick, onstart, onlog }: {
    car?: Car; skins: Skin[]; skinId: string; track?: Track; layout: string;
    running: boolean; launchMsg: { kind: "ok" | "err"; text: string } | null; label: string;
    onskin: (id: string) => void; onlayout: (cfg: string) => void;
    onpick: (t: "car" | "track") => void; onstart: () => void; onlog: () => void;
  } = $props();

  const skin = $derived(skins.find((s) => s.id === skinId));
  const lay = $derived(track?.layouts.find((l) => l.config === layout) ?? track?.layouts[0]);
  const ready = $derived(!!car && !!track && !!skinId && !running);
  const km = (m?: number) => (m ? `${(m / 1000).toFixed(2)} km` : null);
</script>

<aside>
  <div class="hero">
    {#key skin?.preview ?? car?.thumbnail}
      <img src={img(skin?.preview ?? car?.thumbnail)} alt="" in:fly={{ y: 8, duration: 320 }} />
    {/key}
    <div class="shade"></div>
    <div class="title">
      <span class="eyebrow">{car?.brand ?? "Select a car"}</span>
      <h2>{car?.name ?? "—"}</h2>
    </div>
  </div>

  <div class="body">
    <div class="row">
      <span class="eyebrow">Livery <em>{skin?.name ?? ""}</em></span>
      <button class="link" onclick={() => onpick("car")}>Change car</button>
    </div>
    <div class="skins">
      {#each skins as s (s.id)}
        <button class="skin" class:sel={s.id === skinId} title={s.name} onclick={() => onskin(s.id)}>
          {#if s.preview}<img src={img(s.preview)} alt={s.name} loading="lazy" />{:else}<span>{s.name}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="row">
      <span class="eyebrow">Track</span>
      <button class="link" onclick={() => onpick("track")}>Change track</button>
    </div>
    <div class="track">
      {#if lay?.outline}<img src={img(lay.outline)} alt="" />{/if}
      <div>
        <strong>{lay?.name ?? "—"}</strong>
        <small>{[track?.country, km(lay?.lengthM), lay?.pitboxes ? `${lay.pitboxes} pits` : null].filter(Boolean).join("  ·  ")}</small>
      </div>
    </div>
    {#if track && track.layouts.length > 1}
      <div class="chips">
        {#each track.layouts as l (l.config)}
          <button class="chip" class:on={l.config === layout} onclick={() => onlayout(l.config)}>{l.name.replace(track.name, "").replace(/^[\s-]+/, "") || l.name}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="foot">
    {#if launchMsg}
      <p class="msg {launchMsg.kind}" transition:fly={{ y: 6, duration: 200 }}>{launchMsg.text}</p>
    {/if}
    <button class="btn primary lg" disabled={!ready} onclick={onstart}>{running ? "Running…" : label}</button>
    <button class="link" onclick={onlog}>Show logs</button>
  </div>
</aside>

<style>
  aside { width: 380px; flex-shrink: 0; border-left: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; }
  .hero { position: relative; aspect-ratio: 16 / 10; background: #000; flex-shrink: 0; overflow: hidden; }
  .hero img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shade { position: absolute; inset: 0; background: linear-gradient(to top, #000 4%, transparent 65%); }
  .title { position: absolute; left: 24px; bottom: 14px; right: 24px; }
  h2 { font-size: 24px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.15; margin-top: 2px; }
  .body { padding: 6px 24px 10px; overflow-y: auto; flex: 1; min-height: 0; }
  .row { display: flex; justify-content: space-between; align-items: baseline; margin: 20px 0 10px; }
  .eyebrow em { font-style: normal; color: var(--fg); text-transform: none; letter-spacing: 0; margin-left: 8px; font-size: 12px; }
  .skins { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; max-height: 188px; overflow-y: auto; padding: 3px; }
  .skin { aspect-ratio: 16 / 10; border-radius: 8px; overflow: hidden; background: var(--panel); border: 1px solid var(--line); outline: 2px solid transparent; outline-offset: 1px;
    transition: transform 0.25s var(--ease), outline-color 0.2s; display: grid; place-items: center; font-size: 10px; color: var(--fg-3); }
  .skin img { width: 100%; height: 100%; object-fit: cover; display: block; filter: grayscale(1); opacity: 0.7; transition: filter 0.4s, opacity 0.3s; }
  .skin:hover { transform: scale(1.05); }
  .skin:hover img, .skin.sel img { filter: none; opacity: 1; }
  .skin.sel { outline-color: #fff; }
  .track { display: flex; gap: 14px; align-items: center; border: 1px solid var(--line); border-radius: 14px; padding: 12px 14px; }
  .track img { width: 60px; height: 60px; object-fit: contain; filter: invert(1) grayscale(1); opacity: 0.85; }
  .track div { display: flex; flex-direction: column; min-width: 0; }
  .track strong { font-weight: 500; }
  .chips { margin-top: 12px; }
  .foot { padding: 16px 24px 18px; display: flex; flex-direction: column; gap: 10px; border-top: 1px solid var(--line); align-items: stretch; }
  .foot .link { align-self: center; }
</style>
