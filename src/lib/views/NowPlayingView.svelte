<script lang="ts">
  // Now Playing, WMP 9 style (FR-004-08/10): artist and title on top,
  // visualization or cover on black, visualization bar with ◀ ▶ and the current
  // list on the right, also on black.
  import { commands, type Track, type VizPreset } from "../bindings";
  import { settings } from "../stores/settings.svelte";
  import { appWindow } from "../platform";
  import Visualization from "../viz/Visualization.svelte";
  import { VIZ_ORDER } from "../viz/presets";
  import { artistNames, fmtTime, goAlbum as goAlbumOf, goArtist as goArtistOf, trackMenu } from "../actions";
  import { player } from "../stores/player.svelte";
  import { ui } from "../stores/ui.svelte";
  import { thumbSrc } from "../thumb";
  import { t } from "../i18n.svelte";
  import ListView from "../components/xp/ListView.svelte";
  import type { Column } from "../components/xp/types";

  const st = $derived(player.state);
  const q = $derived(player.queue);
  const cover = $derived(thumbSrc(st?.track?.thumbnails, 544));
  // Shown at 22 px: 48 is enough even on high-density screens.
  const small = $derived(thumbSrc(st?.track?.thumbnails, 48));

  // Visualization (005): "Cover" = cover only.
  const preset = $derived<VizPreset>(settings.current?.visualization ?? "Cover");
  const showViz = $derived(preset !== "Cover" && !!st?.track);

  function step(dir: 1 | -1) {
    const i = VIZ_ORDER.indexOf(preset);
    const next = VIZ_ORDER[(i + dir + VIZ_ORDER.length) % VIZ_ORDER.length]!;
    void settings.update({ visualization: next });
  }

  let clickTimer: ReturnType<typeof setTimeout> | undefined;
  function stageClick() {
    // A click changes the visualization; a double click (fullscreen) cancels it.
    clearTimeout(clickTimer);
    clickTimer = setTimeout(() => step(1), 250);
  }

  let fullscreen = $state(false);
  function toggleFullscreen() {
    clearTimeout(clickTimer);
    fullscreen = !fullscreen;
    void appWindow()?.setFullscreen(fullscreen);
  }

  const totalMs = $derived((q?.tracks ?? []).reduce((a, x) => a + (x.durationMs ?? 0), 0));

  const columns: Column<Track>[] = [
    { key: "title", label: t.columns.title, width: "3fr", text: (r) => r.title },
    { key: "artist", label: t.columns.artist, width: "2fr", text: (r) => artistNames(r) },
    { key: "dur", label: t.columns.duration, width: "44px", align: "right", text: (r) => fmtTime(r.durationMs), sort: (r) => r.durationMs ?? 0 },
  ];

  function goArtist() {
    if (st?.track) goArtistOf(st.track);
  }
  function goAlbum() {
    if (st?.track) goAlbumOf(st.track);
  }
</script>

<div class="now">
  <section class="center">
    <header class="head">
      {#if st?.track}
        <button class="artist" onclick={goArtist}>{artistNames(st.track)}</button>
        <span class="title">{st.track.title}</span>
      {:else}
        <span class="title idle">{t.views.nothingPlaying}</span>
      {/if}
    </header>

    <!-- The stage is operated with the mouse as in WMP; the keyboard has Ctrl+… and the View menu. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="stage"
      class:fullscreen
      class:viz={showViz}
      onclick={stageClick}
      ondblclick={toggleFullscreen}
      onkeydown={(e) => e.key === "Escape" && fullscreen && toggleFullscreen()}
    >
      {#if showViz}
        {#key preset}<Visualization {preset} />{/key}
      {:else if st?.track}
        <div class="glow" style:background-image={cover ? `url("${cover}")` : undefined}></div>
        <div class="art">{#if cover}<img src={cover} alt="" />{/if}</div>
      {/if}
    </div>

    <div class="vizbar">
      <div class="steps">
        <button title={t.viz.previous} aria-label={t.viz.previous} onclick={() => step(-1)}>
          <svg viewBox="0 0 10 10"><path d="M7 1.5 2.5 5 7 8.5z" /></svg>
        </button>
        <button title={t.viz.next} aria-label={t.viz.next} onclick={() => step(1)}>
          <svg viewBox="0 0 10 10"><path d="M3 1.5 7.5 5 3 8.5z" /></svg>
        </button>
      </div>
      <span class="preset">{t.options.vizNames[preset]}</span>
      <span class="grow"></span>
      {#if st?.track}
        <button class="album" onclick={goAlbum} disabled={!st.track.album?.id}>
          {#if small}<img src={small} alt="" />{/if}
          <span>{st.track.album?.name ?? artistNames(st.track)}</span>
        </button>
      {/if}
    </div>
  </section>

  <aside class="queue">
    <header>
      <span class="q-title">{t.views.queue}</span>
      <span class="q-count">{q?.tracks.length ?? 0}</span>
    </header>
    <ListView
      dark
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
      <label title={t.views.autoplay}>
        <input type="checkbox" checked={q?.autoplay ?? true} onchange={(e) => commands.queueSetAutoplay(e.currentTarget.checked)} />
        {t.views.autoplayShort}
      </label>
      <button class="clear" disabled={!q?.tracks.length} onclick={() => commands.queueClear()}>{t.views.clearQueue}</button>
      <span class="total">{t.views.totalTime}: {fmtTime(totalMs) || "0:00"}</span>
    </footer>
  </aside>
</div>

<style>
  .now {
    flex: 1;
    display: flex;
    min-height: 0;
    background: var(--np-bg);
    color: #fff;
  }
  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    border-right: 2px solid var(--skin-low);
  }
  .head {
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 6px 10px 6px;
    min-width: 0;
  }
  .artist {
    align-self: flex-start;
    max-width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: #c8c8c8;
    font: 12px var(--font-ui);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }
  .artist:hover {
    color: #fff;
    text-decoration: underline;
  }
  .title {
    font: bold 17px var(--font-title);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title.idle {
    color: #8a8a8a;
    font-weight: normal;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    margin: 0 6px;
    overflow: hidden;
    background: #000;
    border: 1px solid #1d1d1d;
  }
  .stage.viz {
    cursor: pointer;
  }
  .stage.fullscreen {
    position: fixed;
    inset: 0;
    z-index: 700;
    margin: 0;
    border: 0;
  }
  .glow {
    position: absolute;
    inset: -40px;
    background-size: cover;
    background-position: center;
    filter: blur(40px) saturate(1.4) brightness(0.45);
  }
  /* Absolutely centered on the stage: with the grid, the % height depended on
     the content itself and the cover ended up shifted and cut off. */
  .art {
    position: absolute;
    inset: 0;
    margin: auto;
    height: min(78%, 420px);
    max-width: 90%;
    aspect-ratio: 1;
    border: 1px solid #333;
    background: #0a0a0a;
    box-shadow: 0 6px 24px rgb(0 0 0 / 0.8);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .vizbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
  }
  .steps {
    display: flex;
    gap: 3px;
  }
  .steps button {
    width: 17px;
    height: 17px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid #000;
    background: radial-gradient(circle at 50% 30%, #f0f0f0, #9a9a9a 70%, #555);
    color: #111;
    cursor: pointer;
  }
  .steps button:hover {
    filter: brightness(1.15);
  }
  .steps svg {
    width: 9px;
    height: 9px;
    fill: currentColor;
  }
  .preset {
    font-weight: bold;
    color: #e6e6e6;
    white-space: nowrap;
  }
  .grow {
    flex: 1;
  }
  .album {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: 55%;
    height: 24px;
    padding: 0 8px 0 0;
    border: 1px solid #2a2a2a;
    background: linear-gradient(to right, #1a1a1a, #000);
    color: #f0c040;
    font: bold 11px var(--font-ui);
    cursor: pointer;
  }
  .album:disabled {
    cursor: default;
  }
  .album img {
    height: 22px;
    aspect-ratio: 1;
    object-fit: cover;
  }
  .album span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .queue {
    flex: none;
    width: 270px;
    display: flex;
    flex-direction: column;
    background: #000;
  }
  .queue header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    margin: 4px 6px;
    padding: 0 8px;
    border: 1px solid var(--skin-edge);
    border-radius: 3px;
    background: linear-gradient(to bottom, #fff, #e3e8f3 60%, #c3cde2);
    color: #000;
  }
  .q-title {
    flex: 1;
    font-weight: bold;
  }
  .q-count {
    color: #555;
  }
  .queue footer {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 6px;
    color: #c8c8c8;
    border-top: 1px solid #1d1d1d;
  }
  .queue footer label {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .clear {
    padding: 0 6px;
    height: 17px;
    border: 1px solid #444;
    border-radius: 3px;
    background: #151515;
    color: #ddd;
    font: inherit;
    cursor: pointer;
  }
  .clear:hover:not(:disabled) {
    background: #252a36;
  }
  .clear:disabled {
    opacity: 0.5;
  }
  .total {
    margin-left: auto;
    white-space: nowrap;
  }
</style>
