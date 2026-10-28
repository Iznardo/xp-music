<script lang="ts">
  // Song list (WMP columns) with playback, context menu and "load more".
  import type { QueueSource, Track } from "../../bindings";
  import { artistNames, fmtTime, hintTrack, playTracks, trackMenu } from "../../actions";
  import { player } from "../../stores/player.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { t } from "../../i18n.svelte";
  import ListView from "../xp/ListView.svelte";
  import type { Column } from "../xp/types";

  let {
    tracks,
    source,
    showAlbum = true,
    numbered = true,
    hasMore = false,
    loadingMore = false,
    onmore,
  }: {
    tracks: Track[];
    source: QueueSource;
    showAlbum?: boolean;
    numbered?: boolean;
    hasMore?: boolean;
    loadingMore?: boolean;
    onmore?: () => void;
  } = $props();

  const columns = $derived<Column<Track>[]>([
    ...(numbered ? [{ key: "n", label: t.columns.index, width: "34px", align: "right" as const, text: (_: Track, i: number) => String(i + 1), sort: () => 0 }] : []),
    { key: "title", label: t.columns.title, width: "3fr", text: (r) => (r.explicit ? `${r.title} 🅴` : r.title) },
    { key: "artist", label: t.columns.artist, width: "2fr", text: (r) => artistNames(r) },
    ...(showAlbum ? [{ key: "album", label: t.columns.album, width: "2fr", text: (r: Track) => r.album?.name ?? "" }] : []),
    { key: "dur", label: t.columns.duration, width: "64px", align: "right", text: (r) => fmtTime(r.durationMs), sort: (r) => r.durationMs ?? 0 },
  ]);

  // Playing row: the first one with the same id.
  const current = $derived.by(() => {
    const id = player.state?.track?.id;
    if (!id) return null;
    const i = tracks.findIndex((x) => x.id === id);
    return i >= 0 ? i : null;
  });

  function playFrom(i: number) {
    void playTracks(tracks, i, source);
  }
</script>

<div class="table">
  <ListView
    rows={tracks}
    {columns}
    {current}
    rowKey={(r, i) => `${r.id}:${i}`}
    onactivate={playFrom}
    onhint={(i) => hintTrack(tracks[i]?.id)}
    onmenu={(idx, e) => {
      const sel = idx.map((i) => tracks[i]!);
      ui.openMenu(e, trackMenu(sel, { onPlay: () => (sel.length === 1 ? playFrom(idx[0]!) : playTracks(sel, 0, source)) }));
    }}
    empty={t.views.empty}
  />
  {#if hasMore}
    <div class="more">
      <button class="xp-btn" disabled={loadingMore} onclick={onmore}>{loadingMore ? t.views.loading : t.views.more}</button>
    </div>
  {/if}
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .more {
    flex: none;
    padding: 4px;
    text-align: center;
    background: var(--view-bg);
    border: 1px solid var(--input-border);
    border-top: 0;
  }
</style>
