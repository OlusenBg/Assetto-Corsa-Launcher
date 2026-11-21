<script lang="ts">
  // Top navigation: wordmark on the left, five destinations on the right.
  import type { Page } from "./api";
  let { page, onnav }: { page: Page; onnav: (p: Page) => void } = $props();
  const items: { id: Page; label: string }[] = [
    { id: "home", label: "Home" },
    { id: "drive", label: "Drive" },
    { id: "session", label: "Session" },
    { id: "extras", label: "Extras" },
    { id: "settings", label: "Settings" },
  ];
</script>

<header>
  <button class="brand" onclick={() => onnav("home")} aria-label="Home"><span class="mono">AC</span><span class="word">LAUNCHER</span></button>
  <nav>
    {#each items as it}
      <button class:on={page === it.id} onclick={() => onnav(it.id)}>{it.label}</button>
    {/each}
  </nav>
</header>

<style>
  header { height: 64px; display: flex; align-items: center; justify-content: space-between; padding: 0 40px; border-bottom: 1px solid var(--line); flex-shrink: 0; }
  .brand { display: flex; align-items: baseline; gap: 12px; }
  .mono { font-size: 18px; font-weight: 600; letter-spacing: 0.14em; }
  .word { font-size: 10px; letter-spacing: 0.4em; color: var(--fg-3); }
  nav { display: flex; gap: 6px; }
  nav button { position: relative; padding: 8px 16px; color: var(--fg-3); font-weight: 500; border-radius: 999px; transition: color 0.2s, background 0.2s; }
  nav button:hover { color: var(--fg); background: var(--hover); }
  nav button.on { color: #000; background: #fff; }
</style>
