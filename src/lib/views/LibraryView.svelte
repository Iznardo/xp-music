<script lang="ts">
  // Library: tree on the left and content on the right (FR-004-04/05).
  import { router, type LibrarySection, type LocalSection } from "../router.svelte";
  import { library } from "../stores/library.svelte";
  import { auth } from "../stores/auth.svelte";
  import { t } from "../i18n.svelte";
  import TreeView from "../components/xp/TreeView.svelte";
  import type { TreeNode } from "../components/xp/types";
  import SectionView from "./SectionView.svelte";
  import PlaylistView from "./PlaylistView.svelte";
  import AlbumView from "./AlbumView.svelte";
  import ArtistView from "./ArtistView.svelte";
  import LocalView from "./LocalView.svelte";

  const go = (section: LibrarySection) => () => router.go({ name: "library", section });
  const goLocal = (view: LocalSection) => () => router.go({ name: "local", view });

  // Files on disk: always there, with or without a YouTube session.
  const myMusic = $derived<TreeNode>({
    id: "local",
    label: t.tree.myMusic,
    icon: "folder",
    onselect: goLocal("songs"),
    children: [
      { id: "local:songs", label: t.tree.songs, icon: "note", onselect: goLocal("songs") },
      { id: "local:albums", label: t.tree.albums, icon: "disc", onselect: goLocal("albums") },
      { id: "local:artists", label: t.tree.artists, icon: "person", onselect: goLocal("artists") },
    ],
  });

  const youtube = $derived<TreeNode>({
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
        // "Liked Music" already has its own node.
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
  });

  const nodes = $derived<TreeNode[]>(auth.signedIn ? [youtube, myMusic] : [myMusic]);

  const selected = $derived.by(() => {
    const r = router.route;
    if (r.name === "library") return r.section;
    if (r.name === "playlist") return `pl:${r.id}`;
    if (r.name === "local") return `local:${r.view}`;
    if (r.name === "localAlbum") return "local:albums";
    if (r.name === "localArtist") return "local:artists";
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
    {#if router.route.name === "local" || router.route.name === "localAlbum" || router.route.name === "localArtist"}
      {#key JSON.stringify(router.route)}<LocalView route={router.route} />{/key}
    {:else if !auth.signedIn}
      <div class="hint">
        <p>{t.views.signInFirst}</p>
        <p>{t.local.signedOutHint}</p>
        <button class="xp-btn" onclick={goLocal("songs")}>{t.tree.myMusic}</button>
      </div>
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
    background: linear-gradient(to right, var(--pane-1), var(--pane-2));
    border-right: 1px solid var(--rule);
    overflow: auto;
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 8px;
    background: var(--view-alt);
    overflow: auto;
  }
  .hint {
    color: var(--text-muted);
    padding: 20px;
    text-align: center;
  }
</style>
