<script lang="ts">
  // Franja superior del cuerpo del skin WMP 9: botón redondo con todos los menús a
  // la izquierda, zona para arrastrar y tres botones redondos a la derecha. Los
  // botones de ventana están en la pestaña de la lista (PlaylistTab).
  import { ui, type MenuItem } from "../../stores/ui.svelte";
  import { menuBar } from "../../menus";
  import { t } from "../../strings";

  let { title }: { title: string } = $props();

  function openMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = menuBar().map((m) => ({ kind: "sub", label: m.label, items: m.items() }));
    ui.openMenu({ x: r.left, y: r.bottom + 2 }, items);
  }
</script>

<header class="top" data-tauri-drag-region>
  <button class="menu" title={t.top.menu} aria-label={t.top.menu} onclick={openMenu}>
    <svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="4.2" fill="none" stroke="currentColor" stroke-width="1.8" /><circle cx="8" cy="8" r="1.4" /></svg>
  </button>
  <span class="title" data-tauri-drag-region>{title}</span>
  <div class="rounds">
    <button title={ui.sideCollapsed ? t.side.show : t.side.hide} aria-label={ui.sideCollapsed ? t.side.show : t.side.hide} onclick={() => ui.toggleSide()}>
      <svg viewBox="0 0 16 16"><rect x="2.5" y="3" width="11" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.4" /><rect x="3" y="3.5" width="3.5" height="9" /></svg>
    </button>
    <button title={t.top.options} aria-label={t.top.options} onclick={() => (ui.optionsOpen = true)}>
      <svg viewBox="0 0 16 16"><path d="M3 4h10M3 8h10M3 12h10" stroke="currentColor" stroke-width="1.6" /><circle cx="6" cy="4" r="1.6" /><circle cx="10" cy="8" r="1.6" /><circle cx="6" cy="12" r="1.6" /></svg>
    </button>
    <button title={t.top.compact} aria-label={t.top.compact} onclick={() => ui.setCompact(true)}>
      <svg viewBox="0 0 16 16"><rect x="2" y="5" width="12" height="6" rx="3" fill="none" stroke="currentColor" stroke-width="1.5" /><circle cx="5.5" cy="8" r="1.6" /></svg>
    </button>
  </div>
</header>

<style>
  .top {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 12px;
  }
  .menu,
  .rounds button {
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--skin-edge);
    color: var(--skin-deep);
    background: radial-gradient(circle at 50% 30%, #fff 0%, #dfe7f8 40%, #9fb3df 80%, #7b93c9 100%);
    box-shadow: inset 0 -2px 3px rgb(0 0 40 / 0.25), 0 1px 1px rgb(0 0 40 / 0.35);
    cursor: pointer;
  }
  .menu {
    width: 21px;
    height: 21px;
    color: #fff;
    background: radial-gradient(circle at 50% 30%, #bfe0ff 0%, #4a8fe8 45%, #1747b0 100%);
  }
  .menu svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
  .title {
    flex: 1;
    min-width: 0;
    color: var(--skin-deep);
    font: bold 11px var(--font-ui);
    text-shadow: 0 1px rgb(255 255 255 / 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rounds {
    display: flex;
    gap: 4px;
  }
  .rounds button {
    width: 19px;
    height: 19px;
  }
  .rounds svg {
    width: 11px;
    height: 11px;
    fill: currentColor;
  }
  .menu:hover,
  .rounds button:hover {
    filter: brightness(1.1);
  }
  .menu:active,
  .rounds button:active {
    filter: brightness(0.85);
  }
</style>
