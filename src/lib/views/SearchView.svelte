<script lang="ts">
  // Search: tabs per filter; songs as a list and the rest as icons (FR-004-06).
  import { onMount } from "svelte";
  import { commands, type MediaItem, type Page, type SearchFilter, type Track } from "../bindings";
  import { Resource } from "../resource.svelte";
  import { router } from "../router.svelte";
  import { t } from "../i18n.svelte";
  import TileGrid from "../components/wmp/TileGrid.svelte";
  import TrackTable from "../components/wmp/TrackTable.svelte";
  import Loading from "../components/xp/Loading.svelte";

  let { query, filter }: { query: string; filter: SearchFilter } = $props();
  const filters: SearchFilter[] = ["All", "Songs", "Videos", "Albums", "Artists", "Playlists"];

  const res = new Resource<Page<MediaItem>>(() => commands.ytSearch(query, filter), () => `search:${filter}:${query}`);
  onMount(() => {
    if (query) void res.load();
  });

  const tracks = $derived((res.data?.items ?? []).flatMap((i) => (i.Track ? [i.Track] : [])) as Track[]);
  const others = $derived((res.data?.items ?? []).filter((i) => !i.Track));

  let loadingMore = $state(false);
  async function more() {
    const d = res.data;
    if (!d?.continuation) return;
    loadingMore = true;
    const r = await commands.ytSearchMore(d.continuation, filter);
    loadingMore = false;
    if (r.status === "ok") res.data = { items: [...d.items, ...r.data.items], continuation: r.data.continuation };
  }
</script>

<div class="search">
  <div class="tabs" role="tablist">
    {#each filters as f}
      <button class="tab" class:active={f === filter} role="tab" aria-selected={f === filter} onclick={() => router.go({ name: "search", filter: f, query })}>
        {t.views.filters[f]}
      </button>
    {/each}
  </div>
  <div class="panel">
    {#if !query}
      <p class="hint">{t.views.searchHint}</p>
    {:else}
      <Loading {res}>
        {#if res.data && !res.data.items.length}
          <p class="hint">{t.views.noResults(query)}</p>
        {/if}
        {#if others.length}
          <div class="tiles" class:only={!tracks.length}><TileGrid items={others} row={tracks.length > 0} /></div>
        {/if}
        {#if tracks.length}
          <TrackTable {tracks} source={{ Search: query }} numbered={false} hasMore={!!res.data?.continuation && filter !== "All"} {loadingMore} onmore={more} />
        {:else if res.data?.continuation && filter !== "All"}
          <div class="more"><button class="xp-btn" disabled={loadingMore} onclick={more}>{t.views.more}</button></div>
        {/if}
      </Loading>
    {/if}
  </div>
</div>

<style>
  .search {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 8px;
    background: var(--view-alt);
  }
  /* XP tabs: the active one merges with the panel and has the orange border on top. */
  .tabs {
    display: flex;
    gap: 1px;
    padding-left: 4px;
  }
  .tab {
    position: relative;
    top: 1px;
    padding: 3px 10px;
    border: 1px solid var(--tab-border);
    border-bottom: 0;
    border-radius: 3px 3px 0 0;
    background: var(--ctl-face);
    font: inherit;
    cursor: default;
  }
  .tab:hover {
    box-shadow: inset 0 2px #f9b63c;
  }
  .tab.active {
    z-index: 1;
    padding-bottom: 5px;
    margin-top: -2px;
    background: var(--view-bg);
    box-shadow: inset 0 2px #f9b63c;
  }
  .panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 8px;
    border: 1px solid var(--tab-border);
    background: var(--view-bg);
  }
  .tiles {
    flex: none;
  }
  .tiles.only {
    flex: 1;
    overflow: auto;
  }
  .hint {
    color: var(--text-muted);
    text-align: center;
    margin-top: 30px;
  }
  .more {
    text-align: center;
    padding: 6px;
  }
</style>
