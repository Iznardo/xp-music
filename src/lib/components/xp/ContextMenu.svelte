<script lang="ts">
  // XP-style context / dropdown menu, with submenus and keyboard (FR-004-09, NFR-004-03).
  import { ui, type MenuItem } from "../../stores/ui.svelte";
  import Self from "./ContextMenu.svelte";

  let { items = null, x = 0, y = 0, onclose }: { items?: MenuItem[] | null; x?: number; y?: number; onclose?: () => void } = $props();

  // No props: it is the global menu from `ui.menu`.
  const root = $derived(items === null);
  const list = $derived(root ? (ui.menu?.items ?? []) : (items ?? []));
  const pos = $derived(root ? { x: ui.menu?.x ?? 0, y: ui.menu?.y ?? 0 } : { x, y });
  let el = $state<HTMLDivElement>();
  let sub = $state<{ i: number; x: number; y: number } | null>(null);
  let focus = $state(-1);
  let place = $state({ x: 0, y: 0 });

  function close() {
    sub = null;
    focus = -1;
    if (root) ui.menu = null;
    onclose?.();
  }

  // Reposition if it goes off the window.
  $effect(() => {
    if (!el || !list.length) return;
    const r = el.getBoundingClientRect();
    place = {
      x: Math.max(0, Math.min(pos.x, innerWidth - r.width - 2)),
      y: Math.max(0, Math.min(pos.y, innerHeight - r.height - 2)),
    };
    if (root) el.focus();
  });

  function run(item: MenuItem) {
    if (item.kind === "item" && !item.disabled) {
      close();
      item.action();
    } else if (item.kind === "check") {
      close();
      item.action();
    }
  }

  function openSub(i: number, target: HTMLElement) {
    const r = target.getBoundingClientRect();
    sub = { i, x: r.right - 3, y: r.top - 3 };
  }

  function selectable(i: number) {
    const it = list[i];
    return it && it.kind !== "sep" && !("disabled" in it && it.disabled);
  }

  function key(e: KeyboardEvent) {
    const n = list.length;
    if (e.key === "Escape") close();
    else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const d = e.key === "ArrowDown" ? 1 : -1;
      let i = focus;
      for (let k = 0; k < n; k++) {
        i = (i + d + n) % n;
        if (selectable(i)) break;
      }
      focus = i;
    } else if ((e.key === "Enter" || e.key === "ArrowRight") && focus >= 0) {
      const it = list[focus]!;
      if (it.kind === "sub") {
        const btn = el?.querySelectorAll<HTMLElement>(":scope > .row")[focus];
        if (btn) openSub(focus, btn);
      } else if (e.key === "Enter") run(it);
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

{#if list.length}
  {#if root}
    <div class="backdrop" role="presentation" onpointerdown={close} oncontextmenu={(e) => { e.preventDefault(); close(); }}></div>
  {/if}
  <div class="menu" bind:this={el} style:left="{place.x}px" style:top="{place.y}px" role="menu" tabindex="-1" onkeydown={key}>
    {#each list as item, i}
      {#if item.kind === "sep"}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          class="row"
          class:focus={focus === i || sub?.i === i}
          class:bold={item.kind === "item" && item.bold}
          disabled={"disabled" in item && item.disabled}
          role="menuitem"
          onpointerenter={(e) => {
            focus = i;
            if (item.kind === "sub") openSub(i, e.currentTarget);
            else sub = null;
          }}
          onclick={(e) => (item.kind === "sub" ? openSub(i, e.currentTarget) : run(item))}
        >
          <span class="check">{item.kind === "check" && item.checked ? "✓" : ""}</span>
          <span class="label">{item.label}</span>
          <span class="extra">{item.kind === "sub" ? "▶" : item.kind === "item" ? (item.shortcut ?? "") : ""}</span>
        </button>
      {/if}
    {/each}
  </div>
  {#if sub && list[sub.i]?.kind === "sub"}
    {@const s = list[sub.i]}
    {#if s && s.kind === "sub"}
      <Self items={s.items} x={sub.x} y={sub.y} onclose={close} />
    {/if}
  {/if}
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
  }
  .menu {
    position: fixed;
    z-index: 901;
    min-width: 170px;
    padding: 2px;
    background: var(--view-bg);
    border: 1px solid var(--border-dark);
    box-shadow: 2px 2px 3px rgb(0 0 0 / 0.35);
    outline: none;
  }
  .row {
    display: flex;
    align-items: center;
    width: 100%;
    height: 19px;
    padding: 0 6px 0 0;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    font: inherit;
    cursor: default;
  }
  .row.focus:not(:disabled) {
    background: var(--selection);
    color: #fff;
  }
  .row:disabled {
    color: var(--text-disabled);
  }
  .bold .label {
    font-weight: bold;
  }
  .check {
    width: 20px;
    text-align: center;
  }
  .label {
    flex: 1;
    white-space: nowrap;
  }
  .extra {
    margin-left: 18px;
    font-size: 8px;
  }
  .sep {
    height: 1px;
    margin: 3px 1px;
    background: var(--border-dark);
  }
</style>
