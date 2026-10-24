<script lang="ts">
  import { commands, type PlaylistDetail } from "../bindings";
  import { Resource } from "../resource.svelte";
  import { playTracks } from "../actions";
  import { t } from "../i18n.svelte";
  import DetailHeader from "../components/wmp/DetailHeader.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import Loading from "../components/xp/Loading.svelte";

  let { id }: { id: string } = $props();
  const res = new Resource<PlaylistDetail>(() => commands.ytPlaylist(id), () => `playlist:${id}`);
  void res.load();

  let loadingMore = $state(false);
  async function more() {
    const d = res.data;
    if (!d?.tracks.continuation) return;
    loadingMore = true;
    const r = await commands.ytMoreTracks(d.tracks.continuation);
    loadingMore = false;
    if (r.status === "ok") res.data = { ...d, tracks: { items: [...d.tracks.items, ...r.data.items], continuation: r.data.continuation } };
  }

  function shuffle() {
    const d = res.data;
    if (!d) return;
    const items = [...d.tracks.items];
    for (let i = items.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [items[i], items[j]] = [items[j]!, items[i]!];
    }
    void playTracks(items, 0, { Playlist: id });
  }
</script>

<Loading {res}>
  {#if res.data}
    {@const d = res.data}
    <DetailHeader kind={t.kinds.playlist} title={d.summary.title} subtitle={d.summary.author} stats={d.stats} thumbs={d.summary.thumbnails}>
      {#snippet actions()}
        <button class="xp-btn" onclick={() => playTracks(d.tracks.items, 0, { Playlist: id })}>{t.menu.playAll}</button>
        <button class="xp-btn" onclick={shuffle}>{t.menu.shuffleAll}</button>
      {/snippet}
    </DetailHeader>
    <TrackTable tracks={d.tracks.items} source={{ Playlist: id }} hasMore={!!d.tracks.continuation} {loadingMore} onmore={more} />
  {/if}
</Loading>
