<script lang="ts">
  // XP Explorer-style tree (FR-004-04): nodes with icon, expand/collapse and keyboard.
  import Icon from "./Icon.svelte";
  import type { TreeNode } from "./types";
  import { t } from "../../i18n.svelte";

  let { nodes, selected }: { nodes: TreeNode[]; selected: string | null } = $props();
  let expanded = $state<Set<string>>(new Set(["library", "playlists", "local"]));

  // Flat list of visible nodes (for the keyboard).
  const visible = $derived.by(() => {
    const out: { node: TreeNode; depth: number }[] = [];
    const walk = (list: TreeNode[], depth: number) => {
      for (const n of list) {
        out.push({ node: n, depth });
        if (n.children?.length && expanded.has(n.id)) walk(n.children, depth + 1);
      }
    };
    walk(nodes, 0);
    return out;
  });

  function toggle(id: string) {
    const s = new Set(expanded);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    expanded = s;
  }

  function key(e: KeyboardEvent) {
    const i = visible.findIndex((v) => v.node.id === selected);
    const cur = visible[i]?.node;
    let next: TreeNode | undefined;
    if (e.key === "ArrowDown") next = visible[Math.min(visible.length - 1, i + 1)]?.node;
    else if (e.key === "ArrowUp") next = visible[Math.max(0, i - 1)]?.node;
    else if (e.key === "ArrowRight" && cur?.children?.length) {
      if (!expanded.has(cur.id)) toggle(cur.id);
      else next = cur.children[0];
    } else if (e.key === "ArrowLeft" && cur && expanded.has(cur.id)) toggle(cur.id);
    else return;
    e.preventDefault();
    next?.onselect?.();
  }
</script>

<div class="tree" role="tree" tabindex="0" onkeydown={key}>
  {#each visible as { node, depth } (node.id)}
    <div class="node" style:padding-left="{depth * 16 + 2}px" role="treeitem" aria-selected={selected === node.id} aria-expanded={node.children?.length ? expanded.has(node.id) : undefined}>
      {#if node.children?.length}
        <button class="exp" aria-label={expanded.has(node.id) ? t.tree.collapse : t.tree.expand} onclick={() => toggle(node.id)}>{expanded.has(node.id) ? "−" : "+"}</button>
      {:else}
        <span class="exp-pad"></span>
      {/if}
      <button
        class="label"
        class:sel={selected === node.id}
        onclick={() => node.onselect?.()}
        ondblclick={() => node.children?.length && toggle(node.id)}
        oncontextmenu={(e) => {
          if (node.onmenu) {
            e.preventDefault();
            node.onmenu(e);
          }
        }}
      >
        <Icon name={node.icon} />
        <span>{node.label}</span>
      </button>
    </div>
  {/each}
</div>

<style>
  .tree {
    padding: 4px 2px;
    outline: none;
    overflow: auto;
  }
  .node {
    display: flex;
    align-items: center;
    height: 19px;
    white-space: nowrap;
  }
  .exp,
  .exp-pad {
    flex: none;
    width: 11px;
    height: 11px;
    margin-right: 3px;
  }
  .exp {
    display: grid;
    place-items: center;
    padding: 0;
    border: 1px solid var(--input-border);
    background: linear-gradient(to bottom right, var(--view-bg), var(--pane-2));
    font: 9px/1 Tahoma;
    color: var(--heading);
    cursor: default;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 1px 3px;
    border: 1px solid transparent;
    background: none;
    font: inherit;
    color: inherit;
    cursor: default;
  }
  .label span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .label:hover span {
    text-decoration: underline;
    color: var(--link);
  }
  .label.sel {
    background: var(--sel-soft);
    border-color: var(--rule);
  }
  .tree:focus .label.sel {
    background: var(--selection);
    color: #fff;
  }
  .tree:focus .label.sel span {
    color: #fff;
  }
</style>
