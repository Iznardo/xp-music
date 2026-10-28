<script lang="ts">
  // Library sections: playlists, songs, liked, albums, artists, history.
  import { commands, type MediaItem, type Page, type Track } from "../bindings";
  import type { LibrarySection } from "../router.svelte";
  import { Resource } from "../resource.svelte";
  import { library } from "../stores/library.svelte";
  import { addToNewPlaylist, playTracks } from "../actions";
  import { t } from "../i18n.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import TileGrid from "../components/wmp/TileGrid.svelte";
  import Loading from "../components/xp/Loading.svelte";

  let { section }: { section: LibrarySection } = $props();

  type Data = { kind: "tracks"; page: Page<Track> } | { kind: "tiles"; items: MediaItem[] } | { kind: "history"; sections: { title: string; tracks: Track[] }[] };

  const res = new Resource<Data>(async () => {
    switch (section) {
      case "playlists": {
        await library.refresh();
        return { status: "ok", data: { kind: "tiles", items: library.playlists.map((p) => ({ Playlist: p }) as MediaItem) } };
      }
      case "albums": {
        const r = await commands.ytLibraryAlbums();
        return r.status === "ok" ? { status: "ok", data: { kind: "tiles", items: r.data.map((a) => ({ Album: a }) as MediaItem) } } : r;
      }
      case "artists": {
        const r = await commands.ytLibraryArtists();
        return r.status === "ok" ? { status: "ok", data: { kind: "tiles", items: r.data.map((a) => ({ Artist: a }) as MediaItem) } } : r;
      }
      case "songs":
      case "liked": {
        const r = section === "songs" ? await commands.ytLibrarySongs() : await commands.ytLikedSongs();
        return r.status === "ok" ? { status: "ok", data: { kind: "tracks", page: r.data } } : r;
      }
      case "history": {
        const r = await commands.ytHistory();
        if (r.status !== "ok") return r;
        const sections = r.data.map((s) => ({ title: s.title, tracks: s.items.flatMap((i) => (i.Track ? [i.Track] : [])) }));
        return { status: "ok", data: { kind: "history", sections } };
      }
    }
  }, () => `section:${section}`);
  void res.load();

  // The lists come from the library: a new list shows up immediately.
  const tiles = $derived<MediaItem[]>(
    section === "playlists" ? library.playlists.map((p) => ({ Playlist: p }) as MediaItem) : res.data?.kind === "tiles" ? res.data.items : [],
  );

  let loadingMore = $state(false);
  async function more() {
    const d = res.data;
    if (d?.kind !== "tracks" || !d.page.continuation) return;
    loadingMore = true;
    const r = await commands.ytMoreTracks(d.page.continuation);
    loadingMore = false;
    if (r.status === "ok") res.data = { kind: "tracks", page: { items: [...d.page.items, ...r.data.items], continuation: r.data.continuation } };
  }

  const titles: Record<LibrarySection, string> = {
    playlists: t.tree.playlists,
    songs: t.tree.songs,
    liked: t.tree.liked,
    albums: t.tree.albums,
    artists: t.tree.artists,
    history: t.tree.history,
  };
</script>

<div class="section">
  <div class="bar">
    <h2>{titles[section]}</h2>
    {#if res.data?.kind === "tracks"}
      {@const tracks = res.data.page.items}
      <span class="count">{t.status.items(tracks.length)}</span>
      <button class="xp-btn" onclick={() => playTracks(tracks, 0, section === "liked" ? { Playlist: "VLLM" } : "Library")}>{t.menu.playAll}</button>
    {:else if res.data?.kind === "tiles"}
      <span class="count">{t.status.items(tiles.length)}</span>
      {#if section === "playlists"}
        <button class="xp-btn" onclick={() => void addToNewPlaylist([])}>{t.menu.newPlaylist}</button>
      {/if}
    {/if}
  </div>

  <Loading {res}>
    {#if res.data?.kind === "tracks"}
      <TrackTable
        tracks={res.data.page.items}
        source={section === "liked" ? { Playlist: "VLLM" } : "Library"}
        hasMore={!!res.data.page.continuation}
        {loadingMore}
        onmore={more}
      />
    {:else if res.data?.kind === "tiles"}
      <div class="scroll"><TileGrid items={tiles} /></div>
    {:else if res.data?.kind === "history"}
      <div class="history">
        {#each res.data.sections as s}
          <h3>{s.title}</h3>
          <div class="hist-table" style:height="{Math.min(s.tracks.length, 12) * 18 + 26}px">
            <TrackTable tracks={s.tracks} source="Library" />
          </div>
        {/each}
      </div>
    {/if}
  </Loading>
</div>

<style>
  .section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }
  h2 {
    margin: 0;
    font: bold 15px var(--font-title);
    color: var(--heading);
  }
  .count {
    flex: 1;
    color: var(--text-faint);
  }
  .scroll {
    flex: 1;
    overflow: auto;
    background: var(--view-bg);
    border: 1px solid var(--input-border);
  }
  .history {
    flex: 1;
    overflow: auto;
  }
  h3 {
    margin: 10px 0 4px;
    font-size: 12px;
    color: var(--heading);
    border-bottom: 1px solid var(--rule);
  }
  .hist-table {
    display: flex;
  }
</style>
