<script lang="ts">
  // Transporte inferior del skin WMP 9 (FR-004-11/12): pantalla LCD verde, botón de
  // reproducir grande en un "pod" que sobresale y el resto de controles en una pista.
  import { commands, type RepeatMode } from "../../bindings";
  import { player } from "../../stores/player.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { router } from "../../router.svelte";
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

  // Pantalla LCD: avisos y estados; mientras suena, el álbum (o el artista).
  const lcdText = $derived.by(() => {
    if (ui.status || !st?.track || st.status !== "Playing") return statusText;
    const album = st.track.album?.name;
    return album ? `${t.transport.album}: ${album}` : artistNames(st.track);
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
  <button class="lcd" onclick={() => router.go({ name: "now" })} title={t.tabs.now.replace("\n", " ")}>
    <span class="lcd-text">{lcdText}</span>
    <span class="lcd-time">{fmtTime(shown) || "00:00"}{duration ? ` / ${fmtTime(duration)}` : ""}</span>
  </button>

  <div class="deck">
    <div class="pod">
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
    </div>

    <div class="rail">
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

      <div class="row">
        <button class="round" title={t.transport.stop} aria-label={t.transport.stop} onclick={() => commands.playerStop()}>
          <svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg>
        </button>
        <div class="pill">
          <button title={t.transport.previous} aria-label={t.transport.previous} onclick={() => commands.playerPrevious()}>
            <svg viewBox="0 0 16 16"><rect x="2" y="4" width="2" height="8" /><path d="M9 4v8L4 8zM14 4v8L9 8z" /></svg>
          </button>
          <button title={t.transport.next} aria-label={t.transport.next} onclick={() => commands.playerNext()}>
            <svg viewBox="0 0 16 16"><path d="M2 4v8l5-4zM7 4v8l5-4z" /><rect x="12" y="4" width="2" height="8" /></svg>
          </button>
        </div>

        <button
          class="round"
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

        <span class="spacer"></span>
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
        <button class="flat" title={t.transport.compact} aria-label={t.transport.compact} onclick={() => ui.setCompact(true)}>
          <svg viewBox="0 0 16 16"><rect x="1.5" y="4.5" width="13" height="7" rx="3.5" fill="none" stroke="currentColor" stroke-width="1.3" /><circle cx="5" cy="8" r="1.8" /></svg>
        </button>
      </div>
    </div>
  </div>
</footer>

<style>
  .transport {
    flex: none;
    position: relative;
    margin-bottom: var(--shape-bottom, 0px);
    color: #fff;
    user-select: none;
  }
  /* Pantalla LCD sobre el transporte. */
  .lcd {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 18px;
    padding: 0 34px 0 106px;
    border: 0;
    border-top: 1px solid var(--skin-edge);
    border-bottom: 1px solid #1a1a1a;
    background: var(--lcd-bg);
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
  .deck {
    display: flex;
    align-items: stretch;
    height: 50px;
    background: linear-gradient(to bottom, var(--skin-hi) 0%, var(--skin-top) 10%, var(--skin-mid) 50%, var(--skin-low) 100%);
  }
  /* Disco del ▶: coincide con el círculo de la silueta (lib/shape.ts: centro a 50 px
     del borde izquierdo y 48 px del inferior, radio 48) y sobresale por arriba. */
  .pod {
    flex: none;
    width: 100px;
  }
  .pod::before {
    content: "";
    position: absolute;
    z-index: 940;
    left: 4px;
    bottom: calc(-1 * var(--shape-bottom, 0px) + 2px);
    width: 92px;
    height: 92px;
    border-radius: 50%;
    background: radial-gradient(circle at 50% 30%, var(--skin-hi), var(--skin-mid) 45%, var(--skin-low) 75%, var(--skin-deep));
    box-shadow: inset 0 2px 3px rgb(255 255 255 / 0.5), inset 0 -3px 6px rgb(0 0 0 / 0.35);
  }
  .play {
    position: absolute;
    z-index: 941;
    left: 19px;
    bottom: calc(-1 * var(--shape-bottom, 0px) + 17px);
    width: 62px;
    height: 62px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid #06183f;
    color: #fff;
    background: radial-gradient(circle at 50% 28%, #d8efff 0%, #5cb0ff 28%, #1765dc 62%, #0a2f86 100%);
    box-shadow: inset 0 -4px 6px rgb(0 0 40 / 0.5), 0 0 8px rgb(80 160 255 / 0.55), 0 2px 4px #000;
    cursor: pointer;
  }
  .play.on {
    background: radial-gradient(circle at 50% 28%, #e4fff0 0%, #6fe08f 28%, #1f9a45 62%, #0b4d20 100%);
    box-shadow: inset 0 -4px 6px rgb(0 40 0 / 0.5), 0 0 8px rgb(90 230 120 / 0.55), 0 2px 4px #000;
  }
  .play svg {
    width: 26px;
    height: 26px;
    fill: currentColor;
    filter: drop-shadow(0 1px 1px rgb(0 0 0 / 0.5));
  }
  .play:hover {
    filter: brightness(1.12);
  }
  .play:active {
    filter: brightness(0.88);
  }
  .rail {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    padding: 0 64px 0 4px;
  }
  .seek {
    position: relative;
    height: 6px;
    border-radius: 3px;
    background: var(--skin-deep);
    box-shadow: inset 0 1px 2px #000, 0 1px rgb(255 255 255 / 0.3);
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
    background: rgb(255 255 255 / 0.18);
  }
  .played {
    background: linear-gradient(to bottom, #b6ffb6, #36c436 55%, #178417);
  }
  .thumb {
    position: absolute;
    top: -4px;
    width: 14px;
    height: 14px;
    margin-left: -7px;
    border-radius: 50%;
    background: radial-gradient(circle at 40% 35%, #fff, #c9d6f2 50%, #6c85bd);
    border: 1px solid var(--skin-edge);
    box-shadow: 0 1px 2px #000;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .round,
  .pill button {
    display: grid;
    place-items: center;
    padding: 0;
    color: var(--skin-deep);
    background: radial-gradient(circle at 50% 30%, #ffffff 0%, #dfe8fa 35%, #9fb4dd 75%, #6f88bf 100%);
    border: 1px solid var(--skin-edge);
    box-shadow: inset 0 -2px 3px rgb(0 0 30 / 0.3), 0 1px 1px rgb(0 0 0 / 0.5);
    cursor: pointer;
  }
  .round {
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }
  .pill {
    display: flex;
  }
  .pill button {
    width: 28px;
    height: 20px;
  }
  .pill button:first-child {
    border-radius: 10px 0 0 10px;
  }
  .pill button:last-child {
    border-radius: 0 10px 10px 0;
    border-left: 0;
  }
  .round svg,
  .pill svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
  .round:hover,
  .pill button:hover {
    filter: brightness(1.1);
  }
  .round:active,
  .pill button:active {
    filter: brightness(0.85);
  }
  .volume {
    width: 84px;
  }
  .spacer {
    flex: 1;
  }
  .flat {
    display: grid;
    place-items: center;
    position: relative;
    width: 24px;
    height: 20px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: #fff;
    opacity: 0.8;
    cursor: pointer;
  }
  .flat svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }
  .flat:hover {
    opacity: 1;
    border-color: rgb(255 255 255 / 0.4);
    background: rgb(255 255 255 / 0.1);
  }
  .flat.on {
    opacity: 1;
    color: #8dff8d;
  }
  .one {
    position: absolute;
    right: 1px;
    bottom: 0;
    font-size: 8px;
    font-weight: bold;
  }
</style>
