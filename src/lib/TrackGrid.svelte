<script lang="ts">
  // Grid of tracks; layouts are chosen in the summary panel.
  import { fade } from "svelte/transition";
  import { img, type Track } from "./api";

  let { tracks, selected, onselect }: { tracks: Track[]; selected: string; onselect: (id: string) => void } = $props();
  let q = $state("");
  const shown = $derived(
    tracks.filter((t) => !q || `${t.name} ${t.country ?? ""} ${t.city ?? ""}`.toLowerCase().includes(q.toLowerCase())),
  );
</script>

<div class="bar"><input class="search" placeholder="Search {tracks.length} tracks" bind:value={q} /><span class="count">{shown.length}</span></div>
<div class="grid">
  {#each shown as t (t.id)}
    <button class="card" class:sel={t.id === selected} onclick={() => onselect(t.id)} in:fade={{ duration: 160 }}>
      <div class="img">
        {#if t.layouts[0].preview}<img src={img(t.layouts[0].preview)} alt="" loading="lazy" />{:else}<div class="ph">No preview</div>{/if}
      </div>
      <div class="meta">
        <strong>{t.name}</strong>
        <small>{[t.country, t.layouts.length > 1 ? `${t.layouts.length} layouts` : null].filter(Boolean).join("  ·  ")}</small>
      </div>
    </button>
  {/each}
</div>

<style>
  .bar { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; padding-right: 24px; }
  .search { flex: 1; }
  .count { color: var(--fg-3); font-variant-numeric: tabular-nums; min-width: 28px; text-align: right; font-size: 12px; }
  .grid { overflow-y: auto; display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 14px;
    padding: 2px 24px 40px 0; align-content: start; grid-auto-rows: max-content; flex: 1; min-height: 0; }
  .card { display: flex; flex-direction: column; text-align: left; background: var(--panel); border: 1px solid var(--line); border-radius: 14px; overflow: hidden;
    transition: transform 0.3s var(--ease), border-color 0.2s, background 0.2s; }
  .card:hover { transform: translateY(-3px); border-color: var(--line-2); background: var(--panel-2); }
  .card.sel { border-color: #fff; }
  .img { width: 100%; aspect-ratio: 16 / 10; flex-shrink: 0; background: #000; overflow: hidden; }
  .img img { width: 100%; height: 100%; object-fit: cover; display: block; filter: grayscale(1) contrast(1.1); opacity: 0.8;
    transition: filter 0.5s, opacity 0.4s, transform 0.6s var(--ease); }
  .card:hover .img img, .card.sel .img img { filter: none; opacity: 1; transform: scale(1.03); }
  .ph { height: 100%; display: grid; place-items: center; color: var(--fg-3); font-size: 12px; }
  .meta { padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 2px; }
  .meta strong { font-weight: 500; }
</style>
