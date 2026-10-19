<script lang="ts" generics="T">
  // ListView "Detalles" de XP, virtualizada (NFR-004-01): columnas ordenables,
  // selección múltiple (Ctrl/Mayús), teclado, menú contextual y arrastrar para reordenar.
  import type { Column } from "./types";

  let {
    rows,
    columns,
    current = null,
    rowKey,
    onactivate,
    onmenu,
    ondelete,
    onmove,
    empty = "",
  }: {
    rows: T[];
    columns: Column<T>[];
    /// Índice (original) de la fila que suena: se pinta en negrita.
    current?: number | null;
    rowKey: (row: T, index: number) => string;
    onactivate?: (index: number) => void;
    onmenu?: (indices: number[], e: MouseEvent | { x: number; y: number }) => void;
    ondelete?: (indices: number[]) => void;
    /// Si se da, se pueden arrastrar filas (solo sin ordenar).
    onmove?: (from: number, to: number) => void;
    empty?: string;
  } = $props();

  const ROW = 18;
  let scroller = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(300);
  let sortKey = $state<string | null>(null);
  let sortDir = $state<1 | -1>(1);
  let selected = $state<Set<number>>(new Set());
  let anchor = $state<number | null>(null);
  let focus = $state<number | null>(null);
  let dragFrom = $state<number | null>(null);
  let dropAt = $state<number | null>(null);

  // Orden visible → índice original.
  const order = $derived.by(() => {
    const idx = rows.map((_, i) => i);
    const col = columns.find((c) => c.key === sortKey);
    if (!col) return idx;
    const val = (i: number) => (col.sort ? col.sort(rows[i]!) : col.text(rows[i]!, i).toLocaleLowerCase());
    return idx.sort((a, b) => {
      const x = val(a);
      const y = val(b);
      return (x < y ? -1 : x > y ? 1 : 0) * sortDir;
    });
  });

  // Al cambiar las filas, la selección deja de tener sentido.
  $effect(() => {
    void rows;
    selected = new Set();
    anchor = null;
    focus = null;
  });

  const template = $derived(columns.map((c) => c.width).join(" "));
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - 8));
  const last = $derived(Math.min(order.length, Math.ceil((scrollTop + height) / ROW) + 8));

  function sortBy(key: string) {
    if (sortKey === key) {
      if (sortDir === 1) sortDir = -1;
      else sortKey = null; // tercer clic: orden original
    } else {
      sortKey = key;
      sortDir = 1;
    }
  }

  function click(pos: number, e: MouseEvent) {
    const i = order[pos]!;
    if (e.shiftKey && anchor !== null) {
      const a = order.indexOf(anchor);
      const [lo, hi] = a < pos ? [a, pos] : [pos, a];
      selected = new Set(order.slice(lo, hi + 1));
    } else if (e.ctrlKey) {
      const s = new Set(selected);
      if (s.has(i)) s.delete(i);
      else s.add(i);
      selected = s;
      anchor = i;
    } else {
      selected = new Set([i]);
      anchor = i;
    }
    focus = pos;
  }

  function menu(pos: number, e: MouseEvent) {
    const i = order[pos]!;
    if (!selected.has(i)) {
      selected = new Set([i]);
      anchor = i;
      focus = pos;
    }
    onmenu?.(selectionInOrder(), e);
  }

  function selectionInOrder(): number[] {
    return order.filter((i) => selected.has(i));
  }

  function ensureVisible(pos: number) {
    if (!scroller) return;
    const top = pos * ROW;
    if (top < scroller.scrollTop) scroller.scrollTop = top;
    else if (top + ROW > scroller.scrollTop + height - ROW) scroller.scrollTop = top - height + 2 * ROW;
  }

  function key(e: KeyboardEvent) {
    const n = order.length;
    if (!n) return;
    let pos = focus ?? -1;
    switch (e.key) {
      case "ArrowDown":
        pos = Math.min(n - 1, pos + 1);
        break;
      case "ArrowUp":
        pos = Math.max(0, pos - 1);
        break;
      case "PageDown":
        pos = Math.min(n - 1, pos + Math.floor(height / ROW));
        break;
      case "PageUp":
        pos = Math.max(0, pos - Math.floor(height / ROW));
        break;
      case "Home":
        pos = 0;
        break;
      case "End":
        pos = n - 1;
        break;
      case "Enter":
        if (focus !== null) onactivate?.(order[focus]!);
        e.preventDefault();
        return;
      case "Delete":
        if (selected.size) ondelete?.(selectionInOrder());
        e.preventDefault();
        return;
      case "a":
        if (e.ctrlKey) {
          selected = new Set(order);
          e.preventDefault();
        }
        return;
      case "ContextMenu":
      case "F10":
        if (e.key === "F10" && !e.shiftKey) return;
        if (focus !== null && scroller) {
          const r = scroller.getBoundingClientRect();
          onmenu?.(selectionInOrder(), { x: r.left + 40, y: r.top + (focus + 1) * ROW - scroller.scrollTop + ROW });
        }
        e.preventDefault();
        return;
      default:
        return;
    }
    e.preventDefault();
    const i = order[pos]!;
    if (e.shiftKey && anchor !== null) {
      const a = order.indexOf(anchor);
      const [lo, hi] = a < pos ? [a, pos] : [pos, a];
      selected = new Set(order.slice(lo, hi + 1));
    } else {
      selected = new Set([i]);
      anchor = i;
    }
    focus = pos;
    ensureVisible(pos);
  }

  // Arrastrar para reordenar (solo en orden original).
  const canDrag = $derived(!!onmove && sortKey === null);
  function dragStart(pos: number, e: DragEvent) {
    if (!canDrag) return;
    dragFrom = pos;
    e.dataTransfer?.setData("text/plain", String(pos));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function dragOver(pos: number, e: DragEvent) {
    if (dragFrom === null) return;
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = e.clientY < r.top + r.height / 2 ? pos : pos + 1;
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    if (dragFrom !== null && dropAt !== null) {
      const to = dropAt > dragFrom ? dropAt - 1 : dropAt;
      if (to !== dragFrom) onmove?.(dragFrom, to);
    }
    dragFrom = null;
    dropAt = null;
  }
</script>

<div class="listview" role="grid" aria-rowcount={rows.length} tabindex="0" onkeydown={key}>
  <div class="header" style:grid-template-columns={template}>
    {#each columns as c}
      <button class="col" style:text-align={c.align ?? "left"} onclick={() => sortBy(c.key)}>
        {c.label}{#if sortKey === c.key}<span class="arrow">{sortDir === 1 ? "▲" : "▼"}</span>{/if}
      </button>
    {/each}
  </div>
  <div
    class="body"
    bind:this={scroller}
    bind:clientHeight={height}
    onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
    role="presentation"
  >
    {#if !rows.length && empty}
      <p class="empty">{empty}</p>
    {/if}
    <div class="spacer" style:height="{order.length * ROW}px">
      {#each order.slice(first, last) as i, k (rowKey(rows[i]!, i))}
        {@const pos = first + k}
        <!-- El teclado lo gestiona la rejilla entera (onkeydown en .listview). -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row"
          class:sel={selected.has(i)}
          class:focus={focus === pos}
          class:current={current === i}
          class:drop-before={dropAt === pos}
          class:drop-after={dropAt === pos + 1 && pos === order.length - 1}
          style:top="{pos * ROW}px"
          style:grid-template-columns={template}
          role="row"
          tabindex="-1"
          aria-selected={selected.has(i)}
          draggable={canDrag}
          onclick={(e) => click(pos, e)}
          ondblclick={() => onactivate?.(i)}
          oncontextmenu={(e) => {
            e.preventDefault();
            menu(pos, e);
          }}
          ondragstart={(e) => dragStart(pos, e)}
          ondragover={(e) => dragOver(pos, e)}
          ondrop={drop}
          ondragend={() => {
            dragFrom = null;
            dropAt = null;
          }}
        >
          {#each columns as c}
            <span class="cell" style:text-align={c.align ?? "left"}>{c.text(rows[i]!, i)}</span>
          {/each}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .listview {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    background: #fff;
    border: 1px solid #7f9db9;
    outline: none;
  }
  .header {
    display: grid;
    flex: none;
    height: 22px;
    background: linear-gradient(to bottom, #ffffff, #f1efe2 80%, #e2ddc9);
    border-bottom: 1px solid #c7c5b2;
    overflow: hidden;
  }
  .col {
    position: relative;
    padding: 0 6px;
    border: 0;
    border-right: 1px solid #dbd8c7;
    background: none;
    font: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: default;
  }
  .col:hover {
    background: linear-gradient(to bottom, #fff, #faf9f4);
    box-shadow: inset 0 -3px #f9b63c;
  }
  .arrow {
    margin-left: 4px;
    font-size: 7px;
    color: #aca899;
  }
  .body {
    position: relative;
    flex: 1;
    overflow: auto;
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 18px;
    display: grid;
    align-items: center;
    cursor: default;
  }
  .row:nth-child(even) {
    background: #f7f9fd;
  }
  .row.current {
    font-weight: bold;
    color: #0b3c99;
  }
  .row.sel {
    background: #ccdaf3;
  }
  .listview:focus-within .row.sel {
    background: var(--selection);
    color: #fff;
  }
  .listview:focus-within .row.focus {
    outline: 1px dotted #000;
    outline-offset: -1px;
  }
  .row.drop-before {
    box-shadow: inset 0 2px #1f4ea6;
  }
  .row.drop-after {
    box-shadow: inset 0 -2px #1f4ea6;
  }
  .cell {
    padding: 0 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    position: absolute;
    inset: 20px 0 auto;
    text-align: center;
    color: #808080;
  }
</style>
