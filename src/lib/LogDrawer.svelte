<script lang="ts">
  // Bottom drawer showing Proton's output and the game's own log; essential for debugging launches.
  import { fly } from "svelte/transition";
  import { readGameLog, readLaunchLog } from "./api";

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();
  let which = $state<"proton" | "game">("proton");
  let text = $state("");

  async function refresh() {
    text = (which === "proton" ? await readLaunchLog() : await readGameLog()) || "(empty)";
  }
  $effect(() => { if (open) { which; refresh(); } });
</script>

{#if open}
  <div class="drawer" transition:fly={{ y: 300, duration: 300 }}>
    <div class="head">
      <button class="chip" class:on={which === "proton"} onclick={() => (which = "proton")}>Proton output</button>
      <button class="chip" class:on={which === "game"} onclick={() => (which = "game")}>Game log</button>
      <span class="sp"></span>
      <button class="btn sm" onclick={refresh}>Refresh</button>
      <button class="btn sm" onclick={onclose}>Close</button>
    </div>
    <pre>{text}</pre>
  </div>
{/if}

<style>
  .drawer { position: fixed; left: 0; right: 0; bottom: 0; height: 300px; background: #000; border-top: 1px solid var(--line-2); display: flex; flex-direction: column; z-index: 20; box-shadow: 0 -30px 60px rgba(0, 0, 0, 0.8); }
  .head { display: flex; gap: 8px; padding: 10px 20px; border-bottom: 1px solid var(--line); align-items: center; }
  .sp { flex: 1; }
  pre { margin: 0; padding: 14px 20px; overflow: auto; flex: 1; font: 11.5px/1.55 "JetBrains Mono", "JetBrainsMono Nerd Font Mono", monospace; color: var(--fg-2); white-space: pre-wrap; }
</style>
