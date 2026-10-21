<script lang="ts">
  // Pestaña que sobresale arriba a la derecha (skin WMP 9): selector de la lista
  // actual y los botones de ventana, pequeños y desplazados como en el original.
  import { appWindow } from "../../platform";
  import { player } from "../../stores/player.svelte";
  import { router } from "../../router.svelte";
  import { t } from "../../strings";

  let { maximized = false }: { maximized?: boolean } = $props();
  const win = appWindow();
  const count = $derived(player.queue?.tracks.length ?? 0);
</script>

<div class="tab" data-tauri-drag-region>
  <button class="combo" onclick={() => router.go({ name: "now" })} title={t.views.queue}>
    <svg class="doc" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 1.5h7l3 3v10H3z" fill="#fff" stroke="#5a6f9e" />
      <path d="M5 6h6M5 8.5h6M5 11h4" stroke="#5a6f9e" />
    </svg>
    <span class="label">{t.views.queue}</span>
    <span class="count">{count}</span>
    <span class="arrow"><svg viewBox="0 0 8 5"><path d="M0 0h8L4 5z" /></svg></span>
  </button>
  <div class="win">
    <button title={t.window.minimize} aria-label={t.window.minimize} onclick={() => win?.minimize()}>
      <svg viewBox="0 0 9 9"><path d="M1 7h7" stroke="currentColor" stroke-width="1.6" /></svg>
    </button>
    <button title={maximized ? t.window.restore : t.window.maximize} aria-label={maximized ? t.window.restore : t.window.maximize} onclick={() => win?.toggleMaximize()}>
      <svg viewBox="0 0 9 9"><rect x="1" y="1.5" width="7" height="6" fill="none" stroke="currentColor" stroke-width="1.3" /></svg>
    </button>
    <button class="close" title={t.window.close} aria-label={t.window.close} onclick={() => win?.close()}>
      <svg viewBox="0 0 9 9"><path d="M1.5 1.5l6 6M7.5 1.5l-6 6" stroke="currentColor" stroke-width="1.6" /></svg>
    </button>
  </div>
</div>

<style>
  .tab {
    height: 100%;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 9px 8px 0 10px;
  }
  .combo {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 23px;
    padding: 0 0 0 4px;
    border: 1px solid #7f8fb8;
    border-radius: 2px;
    background: #fff;
    color: #000;
    font: 12px var(--font-ui);
    text-align: left;
    box-shadow: inset 1px 1px 2px rgb(0 0 40 / 0.2);
    cursor: pointer;
  }
  .doc {
    width: 15px;
    height: 15px;
    flex: none;
  }
  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    color: #666;
    font-size: 10px;
  }
  .arrow {
    display: grid;
    place-items: center;
    width: 19px;
    height: 100%;
    border-left: 1px solid #b7c3df;
    background: linear-gradient(to bottom, #f7f9ff, #c9d5ef);
  }
  .arrow svg {
    width: 8px;
    height: 5px;
    fill: #2b3f75;
  }
  .win {
    display: flex;
    gap: 1px;
    margin-top: -5px;
  }
  .win button {
    display: grid;
    place-items: center;
    width: 15px;
    height: 13px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--skin-deep);
    cursor: pointer;
  }
  .win button:hover {
    color: #fff;
    background: var(--skin-mid);
    border-radius: 2px;
  }
  .win .close:hover {
    background: #d75c32;
  }
  .win svg {
    width: 9px;
    height: 9px;
  }
</style>
