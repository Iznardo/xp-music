<script lang="ts">
  // Biblioteca: árbol a la izquierda y contenido a la derecha (FR-004-04/05).
  import { router, type LibrarySection } from "../router.svelte";
  import { library } from "../stores/library.svelte";
  import { auth } from "../stores/auth.svelte";
  import { t } from "../strings";
  import TreeView from "../components/xp/TreeView.svelte";
  import type { TreeNode } from "../components/xp/types";
  import SectionView from "./SectionView.svelte";
  import PlaylistView from "./PlaylistView.svelte";
  import AlbumView from "./AlbumView.svelte";
  import ArtistView from "./ArtistView.svelte";

  const go = (section: LibrarySection) => () => router.go({ name: "library", section });

  const nodes = $derived<TreeNode[]>([
    {
      id: "library",
      label: t.tree.library,
      icon: "library",
      onselect: go("playlists"),
      children: [
        {
          id: "playlists",
          label: t.tree.playlists,
          icon: "folder",
          onselect: go("playlists"),
          // "Música que me gusta" ya tiene su propio nodo.
          children: library.playlists.filter((p) => p.id !== "VLLM").map((p) => ({
            id: `pl:${p.id}`,
            label: p.title,
            icon: "playlist",
            onselect: () => router.go({ name: "playlist", id: p.id }),
          })),
        },
        { id: "liked", label: t.tree.liked, icon: "heart", onselect: go("liked") },
        { id: "songs", label: t.tree.songs, icon: "note", onselect: go("songs") },
        { id: "albums", label: t.tree.albums, icon: "disc", onselect: go("albums") },
        { id: "artists", label: t.tree.artists, icon: "person", onselect: go("artists") },
        { id: "history", label: t.tree.history, icon: "clock", onselect: go("history") },
      ],
    },
  ]);

  const selected = $derived.by(() => {
    const r = router.route;
    if (r.name === "library") return r.section;
    if (r.name === "playlist") return `pl:${r.id}`;
    return null;
  });

  $effect(() => {
    if (auth.signedIn && !library.loaded) void library.refresh();
  });
</script>

<div class="library">
  <aside class="pane">
    <TreeView {nodes} {selected} />
  </aside>
  <section class="content">
    {#if !auth.signedIn}
      <p class="hint">{t.views.signInFirst}</p>
    {:else if router.route.name === "library"}
      {#key router.route.section}<SectionView section={router.route.section} />{/key}
    {:else if router.route.name === "playlist"}
      {#key router.route.id}<PlaylistView id={router.route.id} />{/key}
    {:else if router.route.name === "album"}
      {#key router.route.id}<AlbumView id={router.route.id} />{/key}
    {:else if router.route.name === "artist"}
      {#key router.route.id}<ArtistView id={router.route.id} />{/key}
    {/if}
  </section>
</div>

<style>
  .library {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .pane {
    flex: none;
    width: 210px;
    display: flex;
    flex-direction: column;
    background: linear-gradient(to right, #f4f7fd, #e5ecf8);
    border-right: 1px solid #a6b9de;
    overflow: auto;
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 8px;
    background: #f5f6fa;
    overflow: auto;
  }
  .hint {
    color: #555;
    padding: 20px;
    text-align: center;
  }
</style>
