<script lang="ts">
  // Barra de menús clásica: clic abre, pasar por encima cambia de menú mientras hay uno abierto.
  import { ui } from "../../stores/ui.svelte";
  import { menuBar } from "../../menus";

  const menus = $derived(menuBar());
  let openIndex = $state<number | null>(null);

  function open(i: number, el: HTMLElement) {
    const r = el.getBoundingClientRect();
    openIndex = i;
    ui.openMenu({ x: r.left, y: r.bottom }, menus[i]!.items());
  }

  // Al cerrarse el menú global, ya no hay ninguno abierto.
  $effect(() => {
    if (!ui.menu) openIndex = null;
  });
</script>

<nav class="menubar">
  {#each menus as m, i}
    <button
      class="item"
      class:open={openIndex === i}
      onclick={(e) => (openIndex === i ? (ui.menu = null) : open(i, e.currentTarget))}
      onpointerenter={(e) => openIndex !== null && openIndex !== i && open(i, e.currentTarget)}
    >
      {m.label}
    </button>
  {/each}
</nav>

<style>
  .menubar {
    flex: none;
    display: flex;
    padding: 1px 2px;
    background: var(--surface);
    border-bottom: 1px solid #fff;
    box-shadow: 0 1px var(--border-dark);
    position: relative;
    z-index: 902;
  }
  .item {
    padding: 2px 7px;
    border: 0;
    background: none;
    font: inherit;
    color: #000;
    cursor: default;
  }
  .item:hover,
  .item.open {
    background: var(--selection);
    color: #fff;
  }
</style>
