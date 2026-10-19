<script lang="ts">
  // Barra de transporte inferior al estilo de WMP 10 (FR-004-11/12).
  import { commands, type RepeatMode } from "../../bindings";
  import { player } from "../../stores/player.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { router } from "../../router.svelte";
  import { thumbSrc } from "../../thumb";
  import { artistNames, fmtTime } from "../../actions";
  import { t } from "../../strings";
  import Slider from "../xp/Slider.svelte";

  const st = $derived(player.state);
  const playing = $derived(st?.status === "Playing" || st?.status === "Buffering" || st?.status === "Loading");
  const duration = $derived(st?.durationMs ?? 0);
  let dragMs = $state<number | null>(null);
  const shown = $derived(dragMs ?? st?.positionMs ?? 0);
  const bufferedPct = $derived(duration ? Math.min(100, ((st?.positionMs ?? 0) + player.bufferedMs) / duration * 100) : 0);
  const playedPct = $derived(duration ? Math.min(100, (shown / duration) * 100) : 0);

  const nextRepeat: Record<RepeatMode, RepeatMode> = { Off: "All", All: "One", One: "Off" };

  const statusText = $derived.by(() => {
    if (ui.status) return ui.status;
    switch (st?.status) {
      case "Loading":
        return t.status.loading;
      case "Buffering":
        return t.status.buffering(Math.round(bufferedPct));
      case "Playing":
        return t.status.playing;
      case "Paused":
        return t.status.paused;
      case "Error":
        return st.error?.message ?? "";
      default:
        return t.status.ready;
    }
  });

  // Los avisos de la barra de estado desaparecen solos.
  $effect(() => {
    if (!ui.status) return;
    const id = setTimeout(() => (ui.status = ""), 4000);
    return () => clearTimeout(id);
  });

  let seekEl: HTMLDivElement;
  function seekFrom(e: PointerEvent) {
    const r = seekEl.getBoundingClientRect();
    return Math.round(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * duration);
  }
  function seekDown(e: PointerEvent) {
    if (!duration) return;
    seekEl.setPointerCapture(e.pointerId);
    dragMs = seekFrom(e);
  }
  function seekMove(e: PointerEvent) {
    if (dragMs !== null) dragMs = seekFrom(e);
  }
  function seekUp() {
    if (dragMs !== null) {
      void commands.playerSeek(dragMs);
      dragMs = null;
    }
  }
</script>

<footer class="transport">
  <div
    class="seek"
    bind:this={seekEl}
    title={t.transport.seek}
    role="slider"
    aria-label={t.transport.seek}
    aria-valuemin={0}
    aria-valuemax={duration}
    aria-valuenow={shown}
    tabindex="-1"
    onpointerdown={seekDown}
    onpointermove={seekMove}
    onpointerup={seekUp}
  >
    <div class="buffered" style:width="{bufferedPct}%"></div>
    <div class="played" style:width="{playedPct}%"></div>
    {#if duration}<div class="thumb" style:left="{playedPct}%"></div>{/if}
  </div>

  <div class="controls">
    <button class="np" onclick={() => router.go({ name: "now" })} title={t.tabs.now.replace("\n", " ")}>
      {#if thumbSrc(st?.track?.thumbnails, 60)}
        <img src={thumbSrc(st?.track?.thumbnails, 60)} alt="" />
      {:else}
        <span class="noart"></span>
      {/if}
      <span class="np-text">
        <span class="np-title">{st?.track?.title ?? t.views.nothingPlaying}</span>
        <span class="np-artist">{st?.track ? artistNames(st.track) : statusText}</span>
      </span>
    </button>

    <div class="buttons">
      <button class="round small" title={t.transport.stop} aria-label={t.transport.stop} onclick={() => commands.playerStop()}>
        <svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg>
      </button>
      <button class="round small" title={t.transport.previous} aria-label={t.transport.previous} onclick={() => commands.playerPrevious()}>
        <svg viewBox="0 0 16 16"><rect x="3" y="4" width="2" height="8" /><path d="M13 4v8L6 8z" /></svg>
      </button>
      <button
        class="round big"
        class:on={playing}
        title={playing ? t.transport.pause : t.transport.play}
        aria-label={playing ? t.transport.pause : t.transport.play}
        onclick={() => commands.playerToggle()}
      >
        {#if playing}
          <svg viewBox="0 0 16 16"><rect x="4" y="3" width="3" height="10" rx=".5" /><rect x="9" y="3" width="3" height="10" rx=".5" /></svg>
        {:else}
          <svg viewBox="0 0 16 16"><path d="M5 3v10l8-5z" /></svg>
        {/if}
      </button>
      <button class="round small" title={t.transport.next} aria-label={t.transport.next} onclick={() => commands.playerNext()}>
        <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
      </button>

      <button
        class="flat"
        title={st?.muted ? t.transport.unmute : t.transport.mute}
        aria-label={st?.muted ? t.transport.unmute : t.transport.mute}
        onclick={() => commands.playerSetMuted(!st?.muted)}
      >
        <svg viewBox="0 0 16 16">
          <path d="M2 6h3l4-3v10l-4-3H2z" />
          {#if st?.muted}
            <path d="M11 6l4 4M15 6l-4 4" stroke="currentColor" stroke-width="1.5" fill="none" />
          {:else}
            <path d="M11 5.5c1.2 1.4 1.2 3.6 0 5M12.8 4c2 2.3 2 5.7 0 8" stroke="currentColor" stroke-width="1.2" fill="none" />
          {/if}
        </svg>
      </button>
      <div class="volume">
        <Slider
          value={Math.round((st?.volume ?? 0.8) * 100)}
          min={0}
          max={100}
          label={t.transport.volume}
          dark
          onchange={(v) => commands.playerSetVolume(v / 100)}
        />
      </div>
    </div>

    <div class="right">
      <span class="time">{fmtTime(shown) || "0:00"}{duration ? ` / ${fmtTime(duration)}` : ""}</span>
      <button
        class="flat toggle"
        class:on={st?.shuffle}
        title={st?.shuffle ? t.transport.shuffleOff : t.transport.shuffleOn}
        aria-pressed={st?.shuffle ?? false}
        onclick={() => commands.playerSetShuffle(!st?.shuffle)}
      >
        <svg viewBox="0 0 16 16"><path d="M1 4h3l7 8h3M1 12h3l2-2.3M9 6.3 11 4h3M12 2l2 2-2 2M12 10l2 2-2 2" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
      </button>
      <button
        class="flat toggle"
        class:on={st?.repeat !== "Off"}
        title={t.transport.repeat[st?.repeat ?? "Off"]}
        aria-label={t.transport.repeat[st?.repeat ?? "Off"]}
        onclick={() => commands.playerSetRepeat(nextRepeat[st?.repeat ?? "Off"])}
      >
        <svg viewBox="0 0 16 16"><path d="M3 7V5h9M10 3l2 2-2 2M13 9v2H4M6 13l-2-2 2-2" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
        {#if st?.repeat === "One"}<span class="one">1</span>{/if}
      </button>
      <button class="flat" title={t.transport.compact} aria-label={t.transport.compact} onclick={() => ui.setCompact(true)}>
        <svg viewBox="0 0 16 16"><rect x="1.5" y="4.5" width="13" height="7" rx="3.5" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="5" cy="8" r="1.8" /></svg>
      </button>
    </div>
  </div>
  <div class="statusline">{statusText}</div>
</footer>

<style>
  .transport {
    flex: none;
    background: linear-gradient(to bottom, var(--wmp-top) 0%, var(--wmp-mid) 45%, var(--wmp-deep) 100%);
    border-top: 1px solid var(--wmp-line);
    color: var(--wmp-text);
    padding: 0 10px 2px;
    user-select: none;
  }
  .seek {
    position: relative;
    height: 7px;
    margin: 6px 0 4px;
    border-radius: 4px;
    background: #000;
    box-shadow: inset 0 1px 2px #000, 0 1px var(--wmp-top);
    cursor: pointer;
    touch-action: none;
  }
  .buffered,
  .played {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 4px;
  }
  .buffered {
    background: var(--wmp-mid);
  }
  .played {
    background: linear-gradient(to bottom, #9cf09c, #2fb62f 55%, #178417);
  }
  .thumb {
    position: absolute;
    top: -4px;
    width: 9px;
    height: 15px;
    margin-left: -4px;
    border-radius: 3px;
    background: linear-gradient(to right, #f4f7ff, #a7b6d6);
    border: 1px solid #2b3c63;
    box-shadow: 0 1px 2px #000;
  }
  .controls {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    height: 48px;
  }
  .np {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .np img,
  .noart {
    width: 40px;
    height: 40px;
    flex: none;
    border: 1px solid var(--wmp-line);
    background: var(--wmp-deep);
    object-fit: cover;
  }
  .np-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .np-title,
  .np-artist {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .np-title {
    font-weight: bold;
    color: #fff;
  }
  .np-artist {
    color: var(--wmp-dim);
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .round {
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 50%;
    border: 1px solid #0a1224;
    color: #20345e;
    background: radial-gradient(circle at 50% 30%, #ffffff 0%, #d9e3f6 35%, #8ea5d3 70%, #5d78b2 100%);
    box-shadow: inset 0 -2px 3px rgb(0 0 30 / 0.35), 0 1px 2px #000;
    cursor: pointer;
  }
  .round svg {
    fill: currentColor;
  }
  .round.small {
    width: 26px;
    height: 26px;
  }
  .round.small svg {
    width: 13px;
    height: 13px;
  }
  .round.big {
    width: 44px;
    height: 44px;
    margin: 0 4px;
    color: #fff;
    background: radial-gradient(circle at 50% 28%, #c7e6ff 0%, #4ea6ff 30%, #1460d8 65%, #0a2f86 100%);
    border-color: #06183f;
    box-shadow: inset 0 -3px 5px rgb(0 0 40 / 0.45), 0 0 6px rgb(80 160 255 / 0.5), 0 1px 3px #000;
  }
  .round.big svg {
    width: 20px;
    height: 20px;
    filter: drop-shadow(0 1px 1px rgb(0 0 0 / 0.5));
  }
  .round:hover {
    filter: brightness(1.12);
  }
  .round:active {
    filter: brightness(0.88);
  }
  .flat {
    display: grid;
    place-items: center;
    position: relative;
    width: 24px;
    height: 22px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--wmp-dim);
    cursor: pointer;
  }
  .flat svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }
  .flat:hover {
    border-color: var(--wmp-line);
    background: rgb(255 255 255 / 0.06);
    color: #fff;
  }
  .flat.on {
    color: #7dff7d;
  }
  .one {
    position: absolute;
    right: 1px;
    bottom: 0;
    font-size: 8px;
    font-weight: bold;
  }
  .volume {
    width: 80px;
    margin-left: 2px;
  }
  .right {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
  }
  .time {
    margin-right: 6px;
    font-variant-numeric: tabular-nums;
    color: var(--wmp-text);
  }
  .statusline {
    height: 14px;
    font-size: 10px;
    color: var(--wmp-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
