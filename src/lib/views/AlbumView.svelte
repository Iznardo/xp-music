<script lang="ts">
  import { commands, type AlbumDetail } from "../bindings";
  import { Resource } from "../resource.svelte";
  import { playTracks } from "../actions";
  import { router } from "../router.svelte";
  import { t } from "../i18n.svelte";
  import DetailHeader from "../components/wmp/DetailHeader.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import Loading from "../components/xp/Loading.svelte";

  let { id }: { id: string } = $props();
  const res = new Resource<AlbumDetail>(() => commands.ytAlbum(id), () => `album:${id}`);
  void res.load();
</script>

<Loading {res}>
  {#if res.data}
    {@const d = res.data}
    {@const artist = d.summary.artists.find((a) => a.id)}
    <DetailHeader
      kind={d.summary.subtitle ?? t.kinds.album}
      title={d.summary.title}
      subtitle={d.summary.artists.map((a) => a.name).join(", ")}
      stats={d.stats}
      thumbs={d.summary.thumbnails}
    >
      {#snippet actions()}
        <button class="xp-btn" onclick={() => playTracks(d.tracks, 0, { Album: id })}>{t.menu.playAll}</button>
        {#if artist?.id}
          <button class="xp-btn" onclick={() => artist.id && router.go({ name: "artist", id: artist.id })}>{t.menu.goArtist}</button>
        {/if}
      {/snippet}
    </DetailHeader>
    <TrackTable tracks={d.tracks} source={{ Album: id }} showAlbum={false} />
  {/if}
</Loading>
