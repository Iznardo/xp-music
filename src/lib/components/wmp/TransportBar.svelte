<script lang="ts">
  // WMP 9 skin controls (FR-004-11/12): a silver pill that sticks out at the
  // bottom left with a large ▶, ■, time, ⏮⏭, mute and volume; and the body's
  // bottom bar with the status display, shuffle and repeat.
  import { commands, type RepeatMode } from "../../bindings";
  import { player } from "../../stores/player.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { router } from "../../router.svelte";
  import { artistNames, fmtTime, isLocal, playlistChoices, setLike } from "../../actions";
  import { t } from "../../i18n.svelte";
  import Slider from "../xp/Slider.svelte";
  import { place, radii, type Piece, type Rect } from "../../skin";

  let { pod, bar }: { pod: Piece; bar: Rect } = $props();

  const st = $derived(player.state);
  const playing = $derived(st?.status === "Playing" || st?.status === "Buffering" || st?.status === "Loading");
  const duration = $derived(st?.durationMs ?? 0);
  let dragMs = $state<number | null>(null);
  const shown = $derived(dragMs ?? st?.positionMs ?? 0);
  const bufferedPct = $derived(duration ? Math.min(100, ((st?.positionMs ?? 0) + player.bufferedMs) / duration * 100) : 0);
  const playedPct = $derived(duration ? Math.min(100, (shown / duration) * 100) : 0);

  const nextRepeat: Record<RepeatMode, RepeatMode> = { Off: "All", All: "One", One: "Off" };

  const track = $derived(st?.track ?? null);
  const liked = $derived(track?.likeStatus === "Like");
  // Likes and playlists are YouTube's: not for songs on disk.
  const ytOnly = $derived(!track || isLocal(track));

  function addMenu(e: MouseEvent) {
    if (!track) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // Upwards: the bar is at the bottom edge of the window.
    ui.openMenu({ x: r.left, y: r.top - 4 - 22 * (playlistChoices([track]).length + 1) }, playlistChoices([track]));
  }

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

  // LCD display: notices and states; while playing, the album (or the artist).
  const lcdText = $derived.by(() => {
    if (ui.status || !st?.track || st.status !== "Playing") return statusText;
    const album = st.track.album?.name;
    return album ? `${t.transport.album}: ${album}` : artistNames(st.track);
  });

  // Status bar notices disappear on their own.
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

<div class="pod" class:joined={pod.r[0] === 0} style="{place(pod)};border-radius:{radii(pod)}">
  <button
    class="play"
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
  <button class="round stop" title={t.transport.stop} aria-label={t.transport.stop} onclick={() => commands.playerStop()}>
    <svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg>
  </button>

  <div class="right">
    <div class="seekrow">
      <svg class="tick" viewBox="0 0 10 8" aria-hidden="true"><path d="M5 0v8L0 4zM10 0v8L5 4z" /></svg>
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
      <svg class="tick" viewBox="0 0 10 8" aria-hidden="true"><path d="M0 0v8l5-4zM5 0v8l5-4z" /></svg>
    </div>
    <div class="row">
      <div class="pill">
        <button title={t.transport.previous} aria-label={t.transport.previous} onclick={() => commands.playerPrevious()}>
          <svg viewBox="0 0 16 16"><rect x="3" y="4" width="2" height="8" /><path d="M13 4v8L6 8z" /></svg>
        </button>
        <button title={t.transport.next} aria-label={t.transport.next} onclick={() => commands.playerNext()}>
          <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
        </button>
      </div>
      <button
        class="round mute"
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
          onchange={(v) => commands.playerSetVolume(v / 100)}
        />
      </div>
    </div>
  </div>
</div>

<div class="bar" style={place(bar)}>
  <button class="lcd" onclick={() => router.go({ name: "now" })} title={t.tabs.now.replace("\n", " ")}>
    <span class="lcd-text">{lcdText}</span>
    <span class="lcd-time">{fmtTime(shown) || "00:00"}{duration ? ` / ${fmtTime(duration)}` : ""}</span>
  </button>
  <button
    class="flat heart"
    class:on={liked}
    disabled={ytOnly}
    title={liked ? t.menu.unlike : t.menu.like}
    aria-pressed={liked}
    onclick={() => track && setLike(track, !liked)}
  >
    <svg viewBox="0 0 16 16"><path d="M8 13.5S2 9.8 2 5.8C2 4 3.4 2.7 5 2.7c1.3 0 2.4.8 3 1.9.6-1.1 1.7-1.9 3-1.9 1.6 0 3 1.3 3 3.1 0 4-6 7.7-6 7.7z" stroke="currentColor" stroke-width="1.3" style:fill={liked ? "currentColor" : "none"} /></svg>
  </button>
  <button class="flat" disabled={ytOnly} title={t.menu.addToPlaylist} aria-label={t.menu.addToPlaylist} aria-haspopup="menu" onclick={addMenu}>
    <svg viewBox="0 0 16 16"><path d="M2 4h8M2 8h8M2 12h5M12 9v6M9 12h6" stroke="currentColor" stroke-width="1.4" style:fill="none" /></svg>
  </button>
  <button
    class="flat"
    class:on={st?.shuffle}
    title={st?.shuffle ? t.transport.shuffleOff : t.transport.shuffleOn}
    aria-pressed={st?.shuffle ?? false}
    onclick={() => commands.playerSetShuffle(!st?.shuffle)}
  >
    <svg viewBox="0 0 16 16"><path d="M1 4h3l7 8h3M1 12h3l2-2.3M9 6.3 11 4h3M12 2l2 2-2 2M12 10l2 2-2 2" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
  </button>
  <button
    class="flat"
    class:on={st?.repeat !== "Off"}
    title={t.transport.repeat[st?.repeat ?? "Off"]}
    aria-label={t.transport.repeat[st?.repeat ?? "Off"]}
    onclick={() => commands.playerSetRepeat(nextRepeat[st?.repeat ?? "Off"])}
  >
    <svg viewBox="0 0 16 16"><path d="M3 7V5h9M10 3l2 2-2 2M13 9v2H4M6 13l-2-2 2-2" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
    {#if st?.repeat === "One"}<span class="one">1</span>{/if}
  </button>
</div>

<style>
  /* Silver pill that sticks out below the body. */
  .pod {
    position: absolute;
    z-index: 30;
    background: linear-gradient(to bottom, #f4f7ff 0%, #d3dcf2 35%, #aebde4 70%, #8ea2d3 100%);
    border: 1px solid var(--skin-edge);
    box-shadow: inset 0 2px 1px rgb(255 255 255 / 0.9), inset 0 -3px 6px rgb(30 50 120 / 0.25), 0 2px 4px rgb(0 0 40 / 0.35);
    user-select: none;
  }
  /* Joined to the side panel: no border or background of its own; the shell paints it
     with the same color as the side panel, so they read as a single piece. */
  .pod.joined {
    border: 0;
    background: none;
    box-shadow: none;
  }
  .play {
    position: absolute;
    left: 15px;
    top: 14px;
    width: 58px;
    height: 58px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--btn-border);
    color: var(--btn-ink);
    background: radial-gradient(circle at 50% 30%, var(--btn-1) 0%, var(--btn-2) 40%, var(--btn-3) 80%, var(--btn-4) 100%);
    box-shadow: inset 0 -4px 6px rgb(20 40 110 / 0.35), 0 0 0 3px rgb(255 255 255 / 0.5), 0 2px 4px rgb(0 0 40 / 0.45);
    cursor: pointer;
  }
  .play.on {
    color: #fff;
    background: radial-gradient(circle at 50% 28%, var(--accent-1) 0%, var(--accent-2) 35%, var(--accent-3) 100%);
  }
  .play svg {
    width: 26px;
    height: 26px;
    fill: currentColor;
  }
  .play:hover,
  .round:hover,
  .pill button:hover {
    filter: brightness(1.08);
  }
  .play:active,
  .round:active,
  .pill button:active {
    filter: brightness(0.85);
  }
  .round,
  .pill button {
    display: grid;
    place-items: center;
    padding: 0;
    color: var(--btn-ink);
    background: radial-gradient(circle at 50% 30%, var(--btn-1) 0%, var(--btn-2) 40%, var(--btn-3) 80%, var(--btn-4) 100%);
    border: 1px solid var(--btn-border);
    box-shadow: inset 0 -2px 3px rgb(20 40 110 / 0.25), 0 1px 1px rgb(0 0 40 / 0.35);
    cursor: pointer;
  }
  .round {
    border-radius: 50%;
  }
  .stop {
    position: absolute;
    left: 80px;
    top: 28px;
    width: 30px;
    height: 30px;
  }
  .round svg,
  .pill svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
  .right {
    position: absolute;
    left: 120px;
    right: 26px;
    top: 18px;
    bottom: 14px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .seekrow {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .tick {
    width: 10px;
    height: 8px;
    fill: var(--skin-deep);
    filter: drop-shadow(0 1px 0 rgb(255 255 255 / 0.6));
  }
  .seek {
    position: relative;
    flex: 1;
    height: 6px;
    border-radius: 3px;
    /* Darkens whatever is underneath: reads the same over blue, olive or silver. */
    background: rgb(0 0 30 / 0.38);
    box-shadow: inset 0 1px 2px rgb(0 0 30 / 0.7), 0 1px rgb(255 255 255 / 0.7);
    cursor: pointer;
    touch-action: none;
  }
  .buffered,
  .played {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 3px;
  }
  .buffered {
    background: rgb(255 255 255 / 0.3);
  }
  .played {
    background: linear-gradient(to bottom, #c4ffc4, #3fca3f 55%, #1a8a1a);
  }
  .thumb {
    position: absolute;
    top: -5px;
    width: 12px;
    height: 16px;
    margin-left: -6px;
    border-radius: 4px;
    background: linear-gradient(to right, var(--btn-1), var(--btn-3));
    border: 1px solid var(--btn-border);
    box-shadow: 0 1px 2px rgb(0 0 40 / 0.5);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pill {
    display: flex;
  }
  .pill button {
    width: 30px;
    height: 20px;
  }
  .pill button:first-child {
    border-radius: 10px 0 0 10px;
  }
  .pill button:last-child {
    border-radius: 0 10px 10px 0;
    border-left: 0;
  }
  .mute {
    width: 22px;
    height: 22px;
  }
  .volume {
    flex: 1;
    min-width: 50px;
  }
  /* Body's bottom bar, to the right of the pill. */
  .bar {
    position: absolute;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 26px 0 10px;
  }
  .lcd {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--lcd-bg);
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.8), 0 1px rgb(255 255 255 / 0.6);
    color: var(--lcd-text);
    font: bold 11px var(--font-ui);
    text-align: left;
    cursor: pointer;
  }
  .lcd-text {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lcd-time {
    font-variant-numeric: tabular-nums;
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
    color: var(--skin-deep);
    cursor: pointer;
  }
  .flat svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }
  .flat:hover {
    border-color: var(--btn-soft-border);
    background: rgb(255 255 255 / 0.35);
  }
  .flat.on {
    color: #148a14;
  }
  .flat.heart.on {
    color: #d0213f;
  }
  .flat:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .flat:disabled:hover {
    border-color: transparent;
    background: none;
  }
  .one {
    position: absolute;
    right: 1px;
    bottom: 0;
    font-size: 8px;
    font-weight: bold;
  }
</style>
