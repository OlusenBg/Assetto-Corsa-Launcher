<script lang="ts">
  // Loading screen: a thin ring draws itself around the monogram while the app loads.
  import { fade } from "svelte/transition";
</script>

<div class="splash" out:fade={{ duration: 600 }}>
  <div class="mark">
    <svg viewBox="0 0 120 120" aria-hidden="true">
      <circle class="track" cx="60" cy="60" r="54" />
      <circle class="arc" cx="60" cy="60" r="54" />
    </svg>
    <span class="mono">AC</span>
  </div>
  <p class="word">ASSETTO CORSA</p>
  <p class="sub">LAUNCHER</p>
  <div class="bar"><i></i></div>
</div>

<style>
  .splash { position: fixed; inset: 0; z-index: 50; background: #000; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .mark { position: relative; width: 120px; height: 120px; display: grid; place-items: center; animation: pop 0.9s var(--ease) both; }
  svg { position: absolute; inset: 0; transform: rotate(-90deg); }
  circle { fill: none; stroke-width: 1.2; }
  .track { stroke: #1d1d1d; }
  .arc { stroke: #fff; stroke-linecap: round; stroke-dasharray: 340; stroke-dashoffset: 340; animation: draw 1.8s var(--ease) infinite; transform-origin: center; }
  .mono { font-size: 30px; font-weight: 300; letter-spacing: 0.16em; padding-left: 0.16em; animation: fadein 1s 0.3s both; }
  .word { margin-top: 34px; font-size: 12px; font-weight: 500; letter-spacing: 0.5em; padding-left: 0.5em; animation: fadein 1s 0.5s both; }
  .sub { margin-top: 6px; font-size: 10px; letter-spacing: 0.5em; padding-left: 0.5em; color: var(--fg-3); animation: fadein 1s 0.7s both; }
  .bar { margin-top: 38px; width: 120px; height: 1px; background: #1d1d1d; overflow: hidden; animation: fadein 1s 0.8s both; }
  .bar i { display: block; height: 100%; width: 40%; background: #fff; animation: slide 1.3s var(--ease) infinite; }
  @keyframes draw { 0% { stroke-dashoffset: 340; } 55% { stroke-dashoffset: 0; } 100% { stroke-dashoffset: -340; } }
  @keyframes slide { from { transform: translateX(-100%); } to { transform: translateX(260%); } }
  @keyframes fadein { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: none; } }
  @keyframes pop { from { opacity: 0; transform: scale(0.92); } to { opacity: 1; transform: none; } }
</style>
