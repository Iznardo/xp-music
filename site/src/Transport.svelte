<script lang="ts">
  // Controls pill and bottom bar (visual copy of the app's TransportBar).
  // Controls the home page music (player.svelte.ts).
  import Slider from "../../src/lib/components/xp/Slider.svelte";
  import { place, radii, type Piece, type Rect } from "../../src/lib/skin";
  import { REPO, fmt } from "./content";
  import { t } from "./i18n.svelte";
  import { current, next, player, seek, setVolume, stop, toggle, toggleMute } from "./player.svelte";

  let { pod, bar }: { pod: Piece; bar: Rect } = $props();

  const track = $derived(current());
  const pct = $derived(Math.min(100, (player.pos / track.dur) * 100));
  const lcd = $derived(player.playing ? `${track.artist}: ${track.title}` : player.pos ? t.player.paused : player.started ? t.player.stopped : t.player.ready);

  let seekEl: HTMLDivElement;
  function seekAt(e: PointerEvent) {
    const r = seekEl.getBoundingClientRect();
    seek(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * track.dur);
  }
</script>

<div class="pod" class:joined={pod.r[0] === 0} style="{place(pod)};border-radius:{radii(pod)}">
  <button class="play" class:on={player.playing} title={player.playing ? t.player.pause : t.player.play} aria-label={player.playing ? t.player.pause : t.player.play} onclick={toggle}>
    {#if player.playing}
      <svg viewBox="0 0 16 16"><rect x="4" y="3" width="3" height="10" rx=".5" /><rect x="9" y="3" width="3" height="10" rx=".5" /></svg>
    {:else}
      <svg viewBox="0 0 16 16"><path d="M5 3v10l8-5z" /></svg>
    {/if}
  </button>
  <button class="round stop" title={t.player.stop} aria-label={t.player.stop} onclick={stop}>
    <svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg>
  </button>

  <div class="right">
    <div class="seekrow">
      <svg class="tick" viewBox="0 0 10 8" aria-hidden="true"><path d="M5 0v8L0 4zM10 0v8L5 4z" /></svg>
      <div class="seek" bind:this={seekEl} role="slider" aria-label={t.player.position} aria-valuemin={0} aria-valuemax={track.dur} aria-valuenow={Math.round(player.pos)} tabindex="-1" onpointerdown={seekAt}>
        <div class="played" style:width="{pct}%"></div>
        <div class="thumb" style:left="{pct}%"></div>
      </div>
      <svg class="tick" viewBox="0 0 10 8" aria-hidden="true"><path d="M0 0v8l5-4zM5 0v8l5-4z" /></svg>
    </div>
    <div class="row">
      <div class="pill">
        <button title={t.player.previous} aria-label={t.player.previous} onclick={() => next(-1)}>
          <svg viewBox="0 0 16 16"><rect x="3" y="4" width="2" height="8" /><path d="M13 4v8L6 8z" /></svg>
        </button>
        <button title={t.player.next} aria-label={t.player.next} onclick={() => next(1)}>
          <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
        </button>
      </div>
      <button class="round mute" title={player.muted ? t.player.unmute : t.player.mute} aria-label={player.muted ? t.player.unmute : t.player.mute} onclick={toggleMute}>
        <svg viewBox="0 0 16 16">
          <path d="M2 6h3l4-3v10l-4-3H2z" />
          {#if player.muted}
            <path d="M11 6l4 4M15 6l-4 4" stroke="currentColor" stroke-width="1.5" fill="none" />
          {:else}
            <path d="M11 5.5c1.2 1.4 1.2 3.6 0 5M12.8 4c2 2.3 2 5.7 0 8" stroke="currentColor" stroke-width="1.2" fill="none" />
          {/if}
        </svg>
      </button>
      <div class="volume">
        <Slider value={player.volume} min={0} max={100} label={t.player.volume} onchange={setVolume} />
      </div>
    </div>
  </div>
</div>

<div class="bar" style={place(bar)}>
  <a class="lcd" href="#/" title={t.player.nowPlaying}>
    <span class="lcd-text">{lcd}</span>
    <span class="lcd-time">{fmt(player.pos)} / {fmt(track.dur)}</span>
  </a>
  <a class="flat heart" href={REPO} title={t.player.star} aria-label={t.player.starShort}>
    <svg viewBox="0 0 16 16"><path d="M8 13.5S2 9.8 2 5.8C2 4 3.4 2.7 5 2.7c1.3 0 2.4.8 3 1.9.6-1.1 1.7-1.9 3-1.9 1.6 0 3 1.3 3 3.1 0 4-6 7.7-6 7.7z" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
  </a>
  <button
    class="flat"
    class:on={player.shuffle}
    title={player.shuffle ? t.player.shuffleOn : t.player.shuffleOff}
    aria-pressed={player.shuffle}
    onclick={() => (player.shuffle = !player.shuffle)}
  >
    <svg viewBox="0 0 16 16"><path d="M1 4h3l7 8h3M1 12h3l2-2.3M9 6.3 11 4h3M12 2l2 2-2 2M12 10l2 2-2 2" stroke="currentColor" stroke-width="1.3" fill="none" /></svg>
  </button>
</div>

<style>
  .pod {
    position: absolute;
    z-index: 30;
    background: linear-gradient(to bottom, var(--shell-1), var(--shell-2) 35%, var(--shell-3) 70%, var(--shell-4));
    border: 1px solid var(--skin-edge);
    box-shadow: inset 0 2px 1px rgb(255 255 255 / 0.9), inset 0 -3px 6px rgb(30 50 120 / 0.25), 0 2px 4px rgb(0 0 40 / 0.35);
    user-select: none;
  }
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
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 30%, var(--btn-1) 0%, var(--btn-2) 40%, var(--btn-3) 80%, var(--btn-4) 100%);
    box-shadow: inset 0 -4px 6px rgb(20 40 110 / 0.35), 0 0 0 3px rgb(255 255 255 / 0.5), 0 2px 4px rgb(0 0 40 / 0.45);
    cursor: pointer;
  }
  .play.on {
    color: #fff;
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 28%, var(--accent-1) 0%, var(--accent-2) 35%, var(--accent-3) 100%);
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
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 30%, var(--btn-1) 0%, var(--btn-2) 40%, var(--btn-3) 80%, var(--btn-4) 100%);
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
    fill: var(--shell-ink);
    filter: drop-shadow(0 1px 0 rgb(255 255 255 / 0.6));
  }
  .seek {
    position: relative;
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: rgb(0 0 30 / 0.38);
    box-shadow: inset 0 1px 2px rgb(0 0 30 / 0.7), 0 1px rgb(255 255 255 / 0.7);
    cursor: pointer;
    touch-action: none;
  }
  .played {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 3px;
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
    text-decoration: none;
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
    width: 24px;
    height: 22px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--shell-ink);
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
    color: var(--toggle-on);
  }
  .flat.heart:hover {
    color: #d0213f;
  }
</style>
