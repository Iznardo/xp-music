<script lang="ts">
  // Barra de menús clásica: clic abre, pasar por encima cambia de menú mientras hay uno abierto.
  import { ui } from "../../stores/ui.svelte";
  import { menuBar } from "../../menus";

  /// `dark`: integrada en la franja superior del skin (texto blanco, sin fondo).
  let { dark = false }: { dark?: boolean } = $props();
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

<nav class="menubar" class:dark>
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
  .menubar.dark {
    padding: 0;
    background: none;
    border: 0;
    box-shadow: none;
  }
  .dark .item {
    padding: 1px 6px;
    border-radius: 3px;
    color: #fff;
    text-shadow: 0 1px 1px rgb(0 0 0 / 0.5);
  }
  .dark .item:hover,
  .dark .item.open {
    background: rgb(255 255 255 / 0.22);
  }
</style>
