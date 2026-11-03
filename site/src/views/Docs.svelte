<script lang="ts">
  // Documentation (the .md files in docs/ or docs/es/): tree on the left, like the Library.
  import TreeView from "../../../src/lib/components/xp/TreeView.svelte";
  import type { TreeNode } from "../../../src/lib/components/xp/types";
  import { docs, sourcePath } from "../docs";
  import { REPO } from "../content";
  import { language, t } from "../i18n.svelte";

  let { slug }: { slug: string } = $props();

  const go = (s: string) => (location.hash = s ? `#/docs/${s}` : "#/docs");
  const pages = $derived(docs[language()]);
  const index = $derived(pages.find((d) => d.slug === "")!);
  // The TreeView opens the "library" node at the start.
  const nodes = $derived<TreeNode[]>([
    {
      id: "library",
      label: index.title,
      icon: "library",
      onselect: () => go(""),
      children: pages.filter((d) => d.slug).map((d) => ({ id: d.slug, label: d.title, icon: "playlist", onselect: () => go(d.slug) })),
    },
  ]);
  const doc = $derived(pages.find((d) => d.slug === slug) ?? index);

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
      <a class="muted" href="{REPO}/edit/main/docs/{sourcePath(language(), doc.file)}">{t.docs.edit}</a>
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
    background: var(--view-bg);
    border-right: 1px solid var(--rule);
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
      border-bottom: 1px solid var(--rule);
    }
  }
</style>
