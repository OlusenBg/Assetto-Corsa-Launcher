<script lang="ts">
  // Searchable grid of installed cars with brand filter.
  import { fade } from "svelte/transition";
  import { img, type Car } from "./api";

  let { cars, selected, onselect }: { cars: Car[]; selected: string; onselect: (id: string) => void } = $props();

  let q = $state("");
  let brand = $state("");

  const brands = $derived([...new Set(cars.map((c) => c.brand).filter((b): b is string => !!b))].sort());
  const shown = $derived(
    cars.filter(
      (c) =>
        (!brand || c.brand === brand) &&
        (!q || `${c.name} ${c.brand ?? ""} ${c.id} ${c.tags.join(" ")}`.toLowerCase().includes(q.toLowerCase())),
    ),
  );
</script>

<div class="bar">
  <input class="search" placeholder="Search {cars.length} cars" bind:value={q} />
  <select bind:value={brand}>
    <option value="">All brands</option>
    {#each brands as b}<option>{b}</option>{/each}
  </select>
  <span class="count">{shown.length}</span>
</div>

<div class="grid">
  {#each shown as c (c.id)}
    <button class="card" class:sel={c.id === selected} onclick={() => onselect(c.id)} in:fade={{ duration: 160 }}>
      <div class="img">
        {#if c.thumbnail}<img src={img(c.thumbnail)} alt="" loading="lazy" />{:else}<div class="ph">No preview</div>{/if}
        {#if c.badge}<img class="badge" src={img(c.badge)} alt="" loading="lazy" />{/if}
      </div>
      <div class="meta">
        <strong>{c.name}</strong>
        <small>{[c.specs.bhp, c.specs.weight, c.class].filter(Boolean).join("  ·  ") || c.id}</small>
      </div>
    </button>
  {:else}
    <p class="none">No cars match.</p>
  {/each}
</div>

<style>
  .bar { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; padding-right: 24px; }
  .search { flex: 1; }
  select { min-width: 150px; }
  .count { color: var(--fg-3); font-variant-numeric: tabular-nums; min-width: 28px; text-align: right; font-size: 12px; }
  .grid { overflow-y: auto; display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 14px;
    padding: 2px 24px 40px 0; align-content: start; grid-auto-rows: max-content; flex: 1; min-height: 0; }
  .card { display: flex; flex-direction: column; text-align: left; background: var(--panel); border: 1px solid var(--line); border-radius: 14px; overflow: hidden;
    transition: transform 0.3s var(--ease), border-color 0.2s, background 0.2s; }
  .card:hover { transform: translateY(-3px); border-color: var(--line-2); background: var(--panel-2); }
  .card.sel { border-color: #fff; }
  .img { position: relative; width: 100%; aspect-ratio: 16 / 9; flex-shrink: 0; background: #000; }
  .img > img:not(.badge) { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block;
    filter: grayscale(1) contrast(1.05); opacity: 0.8; transition: filter 0.5s, opacity 0.4s, transform 0.6s var(--ease); }
  .card:hover .img > img:not(.badge), .card.sel .img > img:not(.badge) { filter: none; opacity: 1; transform: scale(1.03); }
  .badge { position: absolute; top: 10px; left: 10px; width: 24px; height: 24px; object-fit: contain; filter: grayscale(1) brightness(1.6) drop-shadow(0 1px 4px #000); }
  .ph { height: 100%; display: grid; place-items: center; color: var(--fg-3); font-size: 12px; }
  .meta { padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 2px; }
  .meta strong { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  small { text-transform: capitalize; }
  .none { color: var(--fg-3); grid-column: 1 / -1; padding: 30px 0; }
</style>
