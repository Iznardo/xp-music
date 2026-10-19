<script lang="ts">
  // Modo compacto con skin propio (FR-004-13): cápsula oscura y brillante, siempre encima.
  import { commands } from "../bindings";
  import { player } from "../stores/player.svelte";
  import { ui } from "../stores/ui.svelte";
  import { thumbSrc } from "../thumb";
  import { artistNames, fmtTime } from "../actions";
  import { appWindow } from "../platform";
  import { t } from "../strings";

  const st = $derived(player.state);
  const playing = $derived(st?.status === "Playing" || st?.status === "Buffering" || st?.status === "Loading");
  const duration = $derived(st?.durationMs ?? 0);
  const pct = $derived(duration ? Math.min(100, ((st?.positionMs ?? 0) / duration) * 100) : 0);
  const cover = $derived(thumbSrc(st?.track?.thumbnails, 120));

  let seekEl: HTMLDivElement;
  function seek(e: MouseEvent) {
    if (!duration) return;
    const r = seekEl.getBoundingClientRect();
    void commands.playerSeek(Math.round(((e.clientX - r.left) / r.width) * duration));
  }
</script>

<div class="skin" data-tauri-drag-region>
  <div class="orb" data-tauri-drag-region>
    {#if cover}<img src={cover} alt="" data-tauri-drag-region />{:else}<span class="noart" data-tauri-drag-region>♪</span>{/if}
  </div>

  <div class="body" data-tauri-drag-region>
    <div class="top" data-tauri-drag-region>
      <div class="text" data-tauri-drag-region>
        <span class="title" data-tauri-drag-region>{st?.track?.title ?? t.views.nothingPlaying}</span>
        <span class="artist" data-tauri-drag-region>{st?.track ? artistNames(st.track) : ""}</span>
      </div>
      <div class="win">
        <button title={t.window.minimize} aria-label={t.window.minimize} onclick={() => appWindow()?.minimize()}>–</button>
        <button title={t.transport.full} aria-label={t.transport.full} onclick={() => ui.setCompact(false)}>⤢</button>
        <button class="close" title={t.window.close} aria-label={t.window.close} onclick={() => appWindow()?.close()}>✕</button>
      </div>
    </div>

    <div class="controls">
      <button class="b" title={t.transport.previous} aria-label={t.transport.previous} onclick={() => commands.playerPrevious()}>
        <svg viewBox="0 0 16 16"><rect x="3" y="4" width="2" height="8" /><path d="M13 4v8L6 8z" /></svg>
      </button>
      <button class="b play" title={playing ? t.transport.pause : t.transport.play} aria-label={playing ? t.transport.pause : t.transport.play} onclick={() => commands.playerToggle()}>
        {#if playing}
          <svg viewBox="0 0 16 16"><rect x="4" y="3" width="3" height="10" rx=".5" /><rect x="9" y="3" width="3" height="10" rx=".5" /></svg>
        {:else}
          <svg viewBox="0 0 16 16"><path d="M5 3v10l8-5z" /></svg>
        {/if}
      </button>
      <button class="b" title={t.transport.next} aria-label={t.transport.next} onclick={() => commands.playerNext()}>
        <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
      </button>
      <div class="seek" bind:this={seekEl} role="slider" tabindex="-1" aria-label={t.transport.seek} aria-valuenow={st?.positionMs ?? 0} onclick={seek} onkeydown={() => {}}>
        <div class="fill" style:width="{pct}%"></div>
      </div>
      <span class="time">{fmtTime(st?.positionMs) || "0:00"}</span>
    </div>
  </div>
</div>

<style>
  .skin {
    position: absolute;
    inset: 4px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 0 8px;
    border-radius: 52px 18px 18px 52px;
    background: linear-gradient(to bottom, var(--wmp-top) 0%, var(--wmp-mid) 48%, var(--wmp-deep) 100%);
    border: 1px solid #000;
    box-shadow: inset 0 1px rgb(255 255 255 / 0.35), inset 0 -2px 6px rgb(0 0 0 / 0.5), 0 2px 6px rgb(0 0 0 / 0.6);
    color: var(--wmp-text);
    overflow: hidden;
  }
  /* Brillo de cristal en la mitad superior. */
  .skin::before {
    content: "";
    position: absolute;
    inset: 1px 1px 50% 40px;
    border-radius: 0 16px 0 0;
    background: linear-gradient(to bottom, rgb(255 255 255 / 0.16), rgb(255 255 255 / 0));
    pointer-events: none;
  }
  .orb {
    flex: none;
    width: 92px;
    height: 92px;
    border-radius: 50%;
    padding: 4px;
    background: radial-gradient(circle at 50% 30%, #c7e6ff, #3d8ff0 45%, #0a2f86);
    box-shadow: 0 0 10px rgb(80 160 255 / 0.55), inset 0 -3px 6px rgb(0 0 40 / 0.5);
  }
  .orb img,
  .noart {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    background: #0c1630;
    font-size: 30px;
    color: #9fc7ff;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .top {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .artist {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title {
    font: bold 13px var(--font-title);
    color: #fff;
  }
  .artist {
    color: var(--wmp-dim);
  }
  .win {
    display: flex;
    gap: 2px;
  }
  .win button {
    width: 18px;
    height: 16px;
    padding: 0;
    border: 1px solid var(--wmp-line);
    border-radius: 3px;
    background: rgb(255 255 255 / 0.08);
    color: var(--wmp-text);
    font: 10px/1 Tahoma;
    cursor: pointer;
  }
  .win button:hover {
    background: rgb(255 255 255 / 0.2);
  }
  .win .close:hover {
    background: #c42b1c;
    border-color: #ff9b8f;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .b {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid #0a1224;
    color: #20345e;
    background: radial-gradient(circle at 50% 30%, #fff 0%, #d9e3f6 35%, #8ea5d3 70%, #5d78b2 100%);
    cursor: pointer;
  }
  .b svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
  .b.play {
    width: 32px;
    height: 32px;
    color: #fff;
    background: radial-gradient(circle at 50% 28%, #c7e6ff 0%, #4ea6ff 30%, #1460d8 65%, #0a2f86 100%);
    box-shadow: 0 0 6px rgb(80 160 255 / 0.5);
  }
  .b.play svg {
    width: 15px;
    height: 15px;
  }
  .b:hover {
    filter: brightness(1.12);
  }
  .seek {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: #000;
    box-shadow: inset 0 1px 2px #000, 0 1px var(--wmp-top);
    cursor: pointer;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: linear-gradient(to bottom, #9cf09c, #2fb62f 55%, #178417);
  }
  .time {
    min-width: 34px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
