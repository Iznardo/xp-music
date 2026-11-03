<script lang="ts">
  import { commands, type ArtistDetail } from "../bindings";
  import { Resource } from "../resource.svelte";
  import { playTracks, playWithRadio } from "../actions";
  import { t } from "../i18n.svelte";
  import DetailHeader from "../components/wmp/DetailHeader.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import TileGrid from "../components/wmp/TileGrid.svelte";
  import Loading from "../components/xp/Loading.svelte";

  let { id }: { id: string } = $props();
  const res = new Resource<ArtistDetail>(() => commands.ytArtist(id), () => `artist:${id}`);
  void res.load();

  async function radio(d: ArtistDetail) {
    const seed = d.topSongs.find((s) => s.id === d.radioSeed) ?? d.topSongs[0];
    if (seed) await playWithRadio(seed);
  }
</script>

<Loading {res}>
  {#if res.data}
    {@const d = res.data}
    <div class="artist">
      <DetailHeader kind={t.kinds.artist} title={d.summary.name} subtitle={d.summary.subtitle} thumbs={d.summary.thumbnails} round>
        {#snippet actions()}
          <button class="xp-btn" onclick={() => playTracks(d.topSongs, 0, "Library")}>{t.menu.playAll}</button>
          <button class="xp-btn" onclick={() => radio(d)}>{t.menu.startRadio}</button>
        {/snippet}
      </DetailHeader>
      {#if d.topSongs.length}
        <h3>{t.views.topSongs}</h3>
        <div class="top" style:height="{d.topSongs.length * 18 + 26}px">
          <TrackTable tracks={d.topSongs} source="Library" />
        </div>
      {/if}
      {#each d.sections as s}
        <h3>{s.title}</h3>
        <TileGrid items={s.items} row />
      {/each}
      {#if d.description}<p class="desc">{d.description}</p>{/if}
    </div>
  {/if}
</Loading>

<style>
  .artist {
    flex: 1;
    overflow: auto;
  }
  h3 {
    margin: 10px 0 4px;
    font-size: 12px;
    color: var(--heading);
    border-bottom: 1px solid var(--rule);
  }
  .top {
    display: flex;
  }
  .desc {
    color: var(--text-muted);
    line-height: 1.45;
    max-width: 70ch;
  }
</style>
