<script lang="ts">
  // Settings hub: General, Controls, Video and Audio as sub-sections with a side menu.
  import { fade } from "svelte/transition";
  import type { AppState, SettingsTab } from "./api";
  import SettingsPage from "./SettingsPage.svelte";
  import ControlsPage from "./ControlsPage.svelte";
  import VideoPage from "./VideoPage.svelte";
  import AudioPage from "./AudioPage.svelte";

  let { tab = $bindable(), app, onsaved }: { tab: SettingsTab; app: AppState | null; onsaved: (s: AppState) => void } = $props();

  const tabs: { id: SettingsTab; label: string; hint: string }[] = [
    { id: "general", label: "General", hint: "Game paths, Proton, launch" },
    { id: "controls", label: "Controls", hint: "Wheel, pedals, buttons" },
    { id: "video", label: "Video", hint: "Resolution and graphics" },
    { id: "audio", label: "Audio", hint: "Volume levels" },
  ];
</script>

<div class="hub">
  <aside>
    <h1>Settings</h1>
    <nav>
      {#each tabs as t}
        <button class:on={tab === t.id} onclick={() => (tab = t.id)}>
          <span>{t.label}</span><small>{t.hint}</small>
        </button>
      {/each}
    </nav>
  </aside>

  <div class="body">
    {#key tab}
      <div class="inner" in:fade={{ duration: 200 }}>
        {#if tab === "general"}<SettingsPage {app} {onsaved} />
        {:else if tab === "controls"}<ControlsPage />
        {:else if tab === "video"}<VideoPage />
        {:else}<AudioPage />{/if}
      </div>
    {/key}
  </div>
</div>

<style>
  .hub { height: 100%; display: grid; grid-template-columns: 280px 1fr; }
  aside { border-right: 1px solid var(--line); padding: 36px 24px; }
  aside h1 { font-size: 26px; margin-bottom: 26px; padding-left: 14px; }
  nav { display: flex; flex-direction: column; gap: 4px; }
  nav button { text-align: left; padding: 12px 14px; border-radius: 12px; display: flex; flex-direction: column; gap: 1px; color: var(--fg-2); transition: background 0.2s, color 0.2s; }
  nav button span { font-weight: 500; font-size: 15px; }
  nav button small { color: var(--fg-3); }
  nav button:hover { background: var(--hover); color: var(--fg); }
  nav button.on { background: #fff; color: #000; }
  nav button.on small { color: #555; }
  .body { overflow-y: auto; }
  .inner { max-width: 860px; padding: 40px 48px 90px; }
</style>
