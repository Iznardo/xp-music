<script lang="ts">
  // Reproducción actual: portada / visualización + lista actual a la derecha (FR-004-08/10).
  import { commands, type Track } from "../bindings";
  import { artistNames, fmtTime, trackMenu } from "../actions";
  import { player } from "../stores/player.svelte";
  import { ui } from "../stores/ui.svelte";
  import { router } from "../router.svelte";
  import { thumbSrc } from "../thumb";
  import { t } from "../strings";
  import ListView from "../components/xp/ListView.svelte";
  import type { Column } from "../components/xp/types";

  const st = $derived(player.state);
  const q = $derived(player.queue);
  const cover = $derived(thumbSrc(st?.track?.thumbnails, 544));

  const columns: Column<Track>[] = [
    { key: "title", label: t.columns.title, width: "3fr", text: (r) => r.title },
    { key: "artist", label: t.columns.artist, width: "2fr", text: (r) => artistNames(r) },
    { key: "dur", label: t.columns.duration, width: "50px", align: "right", text: (r) => fmtTime(r.durationMs), sort: (r) => r.durationMs ?? 0 },
  ];
</script>

<div class="now">
  <div class="stage">
    <div class="glow" style:background-image={cover ? `url("${cover}")` : undefined}></div>
    {#if st?.track}
      <div class="art">{#if cover}<img src={cover} alt="" />{/if}</div>
      <div class="meta">
        <span class="title">{st.track.title}</span>
        <button class="link" onclick={() => { const a = st?.track?.artists.find((x) => x.id); if (a?.id) router.go({ name: "artist", id: a.id }); }}>{artistNames(st.track)}</button>
        {#if st.track.album}
          <button class="link album" onclick={() => { const id = st?.track?.album?.id; if (id) router.go({ name: "album", id }); }}>{st.track.album.name}</button>
        {/if}
      </div>
    {:else}
      <p class="idle">{t.views.nothingPlaying}</p>
    {/if}
  </div>

  <aside class="queue">
    <header>{t.views.queue}</header>
    <ListView
      rows={q?.tracks ?? []}
      {columns}
      current={q?.currentIndex ?? null}
      rowKey={(r, i) => `${r.id}:${i}`}
      onactivate={(i) => commands.queuePlayIndex(i)}
      ondelete={(idx) => [...idx].sort((a, b) => b - a).forEach((i) => commands.queueRemove(i))}
      onmove={(from, to) => commands.queueMove(from, to)}
      onmenu={(idx, e) => {
        const sel = idx.map((i) => q!.tracks[i]!);
        ui.openMenu(e, trackMenu(sel, { onPlay: () => commands.queuePlayIndex(idx[0]!), queueIndex: idx.length === 1 ? idx[0] : undefined }));
      }}
      empty={t.views.queueEmpty}
    />
    <footer>
      <label><input type="checkbox" checked={q?.autoplay ?? true} onchange={(e) => commands.queueSetAutoplay(e.currentTarget.checked)} /> {t.views.autoplay}</label>
      <button class="xp-btn" disabled={!q?.tracks.length} onclick={() => commands.queueClear()}>{t.views.clearQueue}</button>
    </footer>
  </aside>
</div>

<style>
  .now {
    flex: 1;
    display: flex;
    min-height: 0;
    background: #000;
  }
  .stage {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    overflow: hidden;
    background: radial-gradient(ellipse at 50% 40%, #1d3263 0%, #070d1d 70%);
    color: #fff;
  }
  /* Fondo: la portada desenfocada (hasta que llegue la visualización de 005). */
  .glow {
    position: absolute;
    inset: -40px;
    background-size: cover;
    background-position: center;
    filter: blur(40px) saturate(1.3) brightness(0.45);
    opacity: 0.8;
  }
  .art {
    position: relative;
    width: min(46vh, 60%);
    aspect-ratio: 1;
    border: 1px solid #5a74ad;
    background: #0c1630;
    box-shadow: 0 6px 24px rgb(0 0 0 / 0.7), 0 0 0 4px rgb(255 255 255 / 0.05);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .meta {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    max-width: 90%;
    text-shadow: 0 1px 3px #000;
  }
  .title {
    font: bold 18px var(--font-title);
  }
  .link {
    border: 0;
    background: none;
    color: #b8cdf7;
    font: 13px var(--font-ui);
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    color: #fff;
  }
  .album {
    font-size: 11px;
    color: #8ea6d6;
  }
  .idle {
    position: relative;
    color: #8ea3cf;
  }
  .queue {
    flex: none;
    width: 300px;
    display: flex;
    flex-direction: column;
    background: #eef2fa;
    border-left: 1px solid #071022;
  }
  .queue header {
    padding: 5px 8px;
    background: linear-gradient(to bottom, var(--wmp-top), var(--wmp-mid));
    color: #fff;
    font-weight: bold;
  }
  .queue footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 4px 6px;
    border-top: 1px solid #a6b9de;
  }
  .queue footer label {
    display: flex;
    align-items: center;
    gap: 3px;
  }
</style>
