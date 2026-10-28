<script lang="ts">
  import { untrack } from "svelte";
  // "My music": songs on disk, by song, album or artist (WMP 9 Media Library). Works
  // without a YouTube session.
  import { commands, type LocalAlbum, type LocalArtist, type LocalFilter, type MediaItem, type QueueSource, type Track } from "../bindings";
  import { router, type Route } from "../router.svelte";
  import { Resource } from "../resource.svelte";
  import { local } from "../stores/local.svelte";
  import { playTracks } from "../actions";
  import { t } from "../i18n.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import TileGrid from "../components/wmp/TileGrid.svelte";
  import DetailHeader from "../components/wmp/DetailHeader.svelte";
  import Loading from "../components/xp/Loading.svelte";

  type LocalRoute = Extract<Route, { name: "local" | "localAlbum" | "localArtist" }>;
  let { route }: { route: LocalRoute } = $props();

  type Data = { kind: "tracks"; tracks: Track[] } | { kind: "albums"; albums: LocalAlbum[] } | { kind: "artists"; artists: LocalArtist[] };

  const filter = (): LocalFilter =>
    route.name === "localAlbum" ? { Album: { artist: route.artist, title: route.title } } : route.name === "localArtist" ? { Artist: route.artist } : "All";

  // No cache key: the data is on disk (instant) and changes with each scan.
  const res = new Resource<Data>(async () => {
    if (route.name === "local" && route.view === "albums") {
      const r = await commands.localAlbums();
      return r.status === "ok" ? { status: "ok", data: { kind: "albums", albums: r.data } } : r;
    }
    if (route.name === "local" && route.view === "artists") {
      const r = await commands.localArtists();
      return r.status === "ok" ? { status: "ok", data: { kind: "artists", artists: r.data } } : r;
    }
    const r = await commands.localTracks(filter());
    return r.status === "ok" ? { status: "ok", data: { kind: "tracks", tracks: r.data } } : r;
  });
  // Reloads at the start and after every scan.
  $effect(() => {
    void local.version;
    untrack(() => res.load());
  });

  const artistName = (a: string) => a || t.local.unknownArtist;
  const albumName = (a: string) => a || t.local.unknownAlbum;

  const tiles = $derived<MediaItem[]>(
    res.data?.kind === "albums"
      ? res.data.albums.map((a) => ({
          Album: {
            id: "",
            title: albumName(a.title),
            artists: [{ id: null, name: artistName(a.artist) }],
            subtitle: [artistName(a.artist), a.year].filter(Boolean).join(" • "),
            thumbnails: a.thumbnails,
          },
        }))
      : res.data?.kind === "artists"
        ? res.data.artists.map((a) => ({
            Artist: { id: "", name: artistName(a.name), subtitle: t.local.artistStats(a.albumCount, a.trackCount), thumbnails: a.thumbnails },
          }))
        : [],
  );

  function openTile(_item: MediaItem, i: number) {
    const d = res.data;
    if (d?.kind === "albums") {
      const a = d.albums[i]!;
      router.go({ name: "localAlbum", artist: a.artist, title: a.title });
    } else if (d?.kind === "artists") {
      router.go({ name: "localArtist", artist: d.artists[i]!.name });
    }
  }

  const title = $derived(
    route.name === "localAlbum" ? albumName(route.title) : route.name === "localArtist" ? artistName(route.artist) : { songs: t.tree.songs, albums: t.tree.albums, artists: t.tree.artists }[route.view],
  );
  const source: QueueSource = "Local";
  const count = $derived(res.data?.kind === "tracks" ? res.data.tracks.length : tiles.length);
  const cover = $derived(res.data?.kind === "tracks" ? (res.data.tracks.find((x) => x.thumbnails.length)?.thumbnails ?? []) : []);
</script>

<div class="local">
  {#if !local.folders.length}
    <div class="empty">
      <h2>{t.tree.myMusic}</h2>
      <p>{t.local.intro}</p>
      <button class="xp-btn" onclick={() => local.addFolder()}>{t.local.addFolder}</button>
    </div>
  {:else}
    {#if route.name === "local"}
      <div class="bar">
        <h2>{title}</h2>
        <span class="count">
          {#if local.scan.scanning}{t.local.scanning(local.scan.done, local.scan.total)}{:else}{t.status.items(count)}{/if}
        </span>
        {#if res.data?.kind === "tracks" && res.data.tracks.length}
          {@const tracks = res.data.tracks}
          <button class="xp-btn" onclick={() => playTracks(tracks, 0, source)}>{t.menu.playAll}</button>
        {/if}
        <button class="xp-btn" onclick={() => local.addFolder()}>{t.local.addFolder}</button>
        <button class="xp-btn" disabled={local.scan.scanning} onclick={() => local.rescan()}>{t.local.rescan}</button>
      </div>
    {:else if res.data?.kind === "tracks"}
      {@const tracks = res.data.tracks}
      <DetailHeader
        kind={route.name === "localAlbum" ? t.kinds.album : t.kinds.artist}
        {title}
        subtitle={route.name === "localAlbum" ? artistName(route.artist) : null}
        stats={t.status.items(tracks.length)}
        thumbs={cover}
        round={route.name === "localArtist"}
      >
        {#snippet actions()}
          <button class="xp-btn" disabled={!tracks.length} onclick={() => playTracks(tracks, 0, source)}>{t.menu.playAll}</button>
          {#if route.name === "localAlbum" && route.artist}
            {@const artist = route.artist}
            <button class="xp-btn" onclick={() => router.go({ name: "localArtist", artist })}>{t.menu.goArtist}</button>
          {/if}
        {/snippet}
      </DetailHeader>
    {/if}

    <Loading {res}>
      {#if res.data?.kind === "tracks"}
        {#if res.data.tracks.length || local.scan.scanning}
          <TrackTable tracks={res.data.tracks} {source} showAlbum={route.name !== "localAlbum"} />
        {:else}
          <p class="hint">{t.local.noSongs}</p>
        {/if}
      {:else if res.data}
        <div class="scroll"><TileGrid items={tiles} onopen={openTile} /></div>
      {/if}
    </Loading>
  {/if}
</div>

<style>
  .local {
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
  .empty {
    margin: auto;
    max-width: 360px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty p {
    margin: 8px 0 12px;
  }
  .hint {
    color: var(--text-muted);
    padding: 20px;
    text-align: center;
  }
</style>
