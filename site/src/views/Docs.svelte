<script lang="ts">
  // Documentación (los .md de docs/): árbol a la izquierda, como la Biblioteca.
  import TreeView from "../../../src/lib/components/xp/TreeView.svelte";
  import type { TreeNode } from "../../../src/lib/components/xp/types";
  import { docs } from "../docs";
  import { REPO } from "../content";

  let { slug }: { slug: string } = $props();

  const go = (s: string) => (location.hash = s ? `#/docs/${s}` : "#/docs");
  const index = docs.find((d) => d.slug === "")!;
  // El TreeView abre de inicio el nodo "library".
  const nodes: TreeNode[] = [
    {
      id: "library",
      label: index.title,
      icon: "library",
      onselect: () => go(""),
      children: docs.filter((d) => d.slug).map((d) => ({ id: d.slug, label: d.title, icon: "playlist", onselect: () => go(d.slug) })),
    },
  ];
  const doc = $derived(docs.find((d) => d.slug === slug) ?? index);

  let article = $state<HTMLElement>();
  $effect(() => {
    void doc;
    article?.scrollTo(0, 0);
  });
</script>

<div class="docs">
  <div class="tree"><TreeView {nodes} selected={doc.slug || "library"} /></div>
  <article class="view" bind:this={article}>
    <header class="vhead">
      <div><h1>{doc.title}</h1></div>
      <span class="grow"></span>
      <a class="muted" href="{REPO}/edit/main/docs/{doc.file}">Editar en GitHub</a>
    </header>
    <div class="md">{@html doc.html}</div>
  </article>
</div>

<style>
  .docs {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .tree {
    width: 210px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: #fff;
    border-right: 1px solid #a7b5d6;
  }
  .tree :global(.tree) {
    flex: 1;
  }
  @media (max-width: 759px) {
    .docs {
      flex-direction: column;
    }
    .tree {
      width: auto;
      max-height: 40%;
      border-right: 0;
      border-bottom: 1px solid #a7b5d6;
    }
  }
</style>
