<script lang="ts">
  // Barra de título Luna dibujada por la app (la ventana no tiene decoración nativa).
  // `data-tauri-drag-region` permite arrastrar y maximizar con doble clic.
  import { appWindow } from "../../platform";
  import { t } from "../../strings";

  let { title, maximized = false, active = true }: { title: string; maximized?: boolean; active?: boolean } =
    $props();

  const win = appWindow();
</script>

<header class="titlebar" class:inactive={!active} data-tauri-drag-region>
  <svg class="icon" viewBox="0 0 16 16" aria-hidden="true" data-tauri-drag-region>
    <circle cx="8" cy="8" r="7" fill="#f4f4f4" stroke="#1d4ec4" />
    <path d="M6 4.5v7l5.5-3.5z" fill="#1d9b2c" />
  </svg>
  <span class="title" data-tauri-drag-region>{title}</span>

  <div class="buttons">
    <button class="btn" title={t.window.minimize} aria-label={t.window.minimize} onclick={() => win?.minimize()}>
      <svg viewBox="0 0 11 11"><rect x="2" y="7" width="6" height="2" fill="currentColor" /></svg>
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
          <path d="M3 1h7v1.5H3zM1 3h7v1.5H1z" fill="currentColor" />
        </svg>
      {:else}
        <svg viewBox="0 0 11 11">
          <rect x="1.5" y="1.5" width="8" height="8" fill="none" stroke="currentColor" />
          <rect x="1" y="1" width="9" height="2" fill="currentColor" />
        </svg>
      {/if}
    </button>
    <button class="btn close" title={t.window.close} aria-label={t.window.close} onclick={() => win?.close()}>
      <svg viewBox="0 0 11 11"><path d="M2 2l7 7M9 2l-7 7" stroke="currentColor" stroke-width="1.8" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 5px 0 6px;
    background: var(--title-grad);
    color: var(--title-text);
  }
  .titlebar.inactive {
    background: var(--title-grad-inactive);
  }
  .icon {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .title {
    flex: 1;
    font-family: var(--font-title);
    font-weight: bold;
    font-size: 13px;
    text-shadow: var(--title-text-shadow);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .buttons {
    display: flex;
    gap: 2px;
  }
  .btn {
    width: 21px;
    height: 21px;
    padding: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    background: var(--btn-grad);
    color: #fff;
    box-shadow: inset 1px 1px rgb(255 255 255 / 0.35);
  }
  :global([data-theme="silver"]) .btn:not(.close) {
    color: #000;
    border-color: #888;
  }
  .btn:hover {
    filter: brightness(1.15);
  }
  .btn:active {
    filter: brightness(0.85);
  }
  .btn.close {
    background: var(--close-grad);
  }
  .btn svg {
    width: 11px;
    height: 11px;
  }
</style>
