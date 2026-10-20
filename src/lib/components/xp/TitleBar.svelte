<script lang="ts">
  // Franja superior del skin WMP 9: logo, menús integrados, título y botones de ventana.
  // `data-tauri-drag-region` permite arrastrar y maximizar con doble clic.
  import { appWindow } from "../../platform";
  import { t } from "../../strings";
  import MenuBar from "./MenuBar.svelte";

  let { title, maximized = false, active = true }: { title: string; maximized?: boolean; active?: boolean } =
    $props();

  const win = appWindow();
</script>

<header class="titlebar" class:inactive={!active} data-tauri-drag-region>
  <svg class="logo" viewBox="0 0 16 16" aria-hidden="true" data-tauri-drag-region>
    <defs>
      <radialGradient id="tb-disc" cx="40%" cy="30%" r="75%">
        <stop offset="0" stop-color="#bfe0ff" />
        <stop offset="0.5" stop-color="#2f7fe0" />
        <stop offset="1" stop-color="#0b3a9c" />
      </radialGradient>
    </defs>
    <circle cx="8" cy="8" r="7.2" fill="url(#tb-disc)" stroke="#e8f3ff" stroke-width="0.8" />
    <path d="M6.2 4.6v6.8l5.4-3.4z" fill="#ff9d1a" stroke="#8a3a00" stroke-width="0.5" />
  </svg>
  <MenuBar dark />
  <span class="title" data-tauri-drag-region>{title}</span>

  <div class="buttons">
    <button class="btn" title={t.window.minimize} aria-label={t.window.minimize} onclick={() => win?.minimize()}>
      <svg viewBox="0 0 11 11"><rect x="2" y="7" width="7" height="2" fill="currentColor" /></svg>
    </button>
    <button
      class="btn"
      title={maximized ? t.window.restore : t.window.maximize}
      aria-label={maximized ? t.window.restore : t.window.maximize}
      onclick={() => win?.toggleMaximize()}
    >
      {#if maximized}
        <svg viewBox="0 0 11 11">
          <path d="M3.5 1.5h6v6h-1M1.5 3.5h6v6h-6z" fill="none" stroke="currentColor" />
        </svg>
      {:else}
        <svg viewBox="0 0 11 11"><rect x="1.5" y="1.5" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
      {/if}
    </button>
    <button class="btn close" title={t.window.close} aria-label={t.window.close} onclick={() => win?.close()}>
      <svg viewBox="0 0 11 11"><path d="M2.5 2.5l6 6M8.5 2.5l-6 6" stroke="currentColor" stroke-width="1.8" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    position: relative;
    flex: none;
    height: 26px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px 0 46px;
    background: linear-gradient(to bottom, var(--skin-hi) 0%, var(--skin-top) 12%, var(--skin-mid) 55%, var(--skin-low) 100%);
    border-bottom: 1px solid var(--skin-edge);
    color: #fff;
  }
  .titlebar.inactive {
    filter: saturate(0.55) brightness(1.08);
  }
  /* El logo ocupa la "oreja" redonda de la silueta (lib/shape.ts). */
  .logo {
    position: absolute;
    left: 3px;
    top: -5px;
    z-index: 960;
    width: 38px;
    height: 38px;
    filter: drop-shadow(0 1px 2px rgb(0 0 0 / 0.5));
  }
  .title {
    flex: 1;
    min-width: 0;
    padding-left: 6px;
    font-family: var(--font-title);
    font-weight: bold;
    font-size: 12px;
    text-align: center;
    text-shadow: 0 1px 1px rgb(0 0 0 / 0.55);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .buttons {
    display: flex;
    gap: 2px;
    padding: 2px 3px;
    border-radius: 9px;
    background: rgb(0 0 0 / 0.18);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.35), 0 1px rgb(255 255 255 / 0.25);
  }
  .btn {
    width: 17px;
    height: 16px;
    padding: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--skin-edge);
    border-radius: 5px;
    background: linear-gradient(to bottom, var(--skin-hi), var(--skin-mid) 60%, var(--skin-low));
    color: #fff;
    box-shadow: inset 0 1px rgb(255 255 255 / 0.45);
  }
  .btn:hover {
    filter: brightness(1.2);
  }
  .btn:active {
    filter: brightness(0.85);
  }
  .btn.close {
    background: linear-gradient(to bottom, #f3b49e, #d75c32 60%, #a8360f);
  }
  .btn svg {
    width: 9px;
    height: 9px;
  }
</style>
