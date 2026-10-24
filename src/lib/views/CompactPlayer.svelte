<script lang="ts">
  // Modo compacto (FR-004-13): un reproductor MP3 plateado de los 2000 con la
  // silueta asimétrica de la ventana principal: lóbulo redondo para el disco que
  // sobresale arriba y abajo, empalmes curvos y una pestaña que sube en S arriba a
  // la derecha. Disco con la portada girando, LCD azul y botones "físicos".
  import { onMount } from "svelte";
  import { commands } from "../bindings";
  import { player } from "../stores/player.svelte";
  import { ui } from "../stores/ui.svelte";
  import { thumbSrc } from "../thumb";
  import { artistNames, fmtTime } from "../actions";
  import { appWindow } from "../platform";
  import { t } from "../i18n.svelte";

  const st = $derived(player.state);
  const playing = $derived(st?.status === "Playing" || st?.status === "Buffering");
  const duration = $derived(st?.durationMs ?? 0);
  const position = $derived(st?.positionMs ?? 0);
  const pct = $derived(duration ? Math.min(100, (position / duration) * 100) : 0);
  const cover = $derived(thumbSrc(st?.track?.thumbnails, 120));
  const title = $derived(st?.track?.title ?? t.views.nothingPlaying);
  const artist = $derived(st?.track ? artistNames(st.track) : "XP Music");
  const volume = $derived(Math.round((st?.volume ?? 0.8) * 100));

  // El título se desplaza si no cabe, como en las pantallas de la época.
  let titleBox = $state(0);
  let titleText = $state(0);
  const scroll = $derived(titleText > titleBox + 2);

  function lcdTime(ms: number) {
    const s = Math.floor(ms / 1000);
    return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  }

  let seekEl: HTMLDivElement;
  function seek(e: MouseEvent) {
    if (!duration) return;
    const r = seekEl.getBoundingClientRect();
    void commands.playerSeek(Math.round(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * duration));
  }

  // --- Silueta ---
  let w = $state(460);
  let h = $state(150);
  const R = 68; // lóbulo del disco
  const CX = 72;
  const TOP = 24; // banda del cuerpo (arriba)
  const BOTTOM_GAP = 20; // y su borde inferior, a esta distancia del final
  const F = 14; // empalmes
  const TAB_Y = 6; // pestaña de arriba a la derecha
  function fillet(line: number, below: boolean, cy: number) {
    const fcY = line + (below ? F : -F);
    const dy = fcY - cy;
    // Al montarse, la ventana aún puede tener el tamaño grande: sin raíces negativas.
    const fx = CX + Math.sqrt(Math.max(0, (R + F) ** 2 - dy ** 2));
    const k = R / (R + F);
    return { fx, tx: CX + (fx - CX) * k, ty: cy + dy * k };
  }
  const outline = $derived.by(() => {
    const cy = h / 2;
    const bot = h - BOTTOM_GAP;
    const t = fillet(TOP, false, cy);
    const b = fillet(bot, true, cy);
    const n = (v: number) => v.toFixed(1);
    return [
      `M ${n(t.tx)} ${n(t.ty)}`,
      `A ${F} ${F} 0 0 0 ${n(t.fx)} ${TOP}`,
      `L ${w - 170} ${TOP}`,
      // Sube en S hasta la pestaña de los tornillos, como en la ventana grande.
      `C ${w - 140} ${TOP} ${w - 130} ${TAB_Y} ${w - 100} ${TAB_Y}`,
      `L ${w - 20} ${TAB_Y}`,
      `A 16 16 0 0 1 ${w - 4} ${TAB_Y + 16}`,
      `L ${w - 4} ${bot - 14}`,
      `A 14 14 0 0 1 ${w - 18} ${bot}`,
      `L ${n(b.fx)} ${bot}`,
      `A ${F} ${F} 0 0 0 ${n(b.tx)} ${n(b.ty)}`,
      `A ${R} ${R} 0 1 1 ${n(t.tx)} ${n(t.ty)}`,
      "Z",
    ].join(" ");
  });

  // La silueta es también la forma para los clics: en lo transparente pasan a lo
  // de detrás (como en la ventana grande).
  let pathEl = $state<SVGPathElement>();
  $effect(() => {
    void outline;
    const path = pathEl;
    if (!path) return;
    const id = setTimeout(() => {
      const dpr = devicePixelRatio || 1;
      const len = path.getTotalLength();
      const pts: [number, number][] = [];
      const steps = Math.round(len / 3);
      for (let i = 0; i < steps; i++) {
        const q = path.getPointAtLength((i / steps) * len);
        pts.push([Math.round(q.x * dpr), Math.round(q.y * dpr)]);
      }
      void commands.windowSetShape([pts]);
    }, 60);
    return () => clearTimeout(id);
  });
  onMount(() => () => void commands.windowSetShape(null));

  function vol(delta: number) {
    void commands.playerSetVolume(Math.min(1, Math.max(0, ((st?.volume ?? 0.8) * 100 + delta) / 100)));
  }
</script>

<div class="frame" bind:clientWidth={w} bind:clientHeight={h}>
<div class="device" style="clip-path: path('{outline}')" data-tauri-drag-region>
  <!-- Disco: la portada gira mientras suena. -->
  <div class="disc" class:spin={playing} data-tauri-drag-region>
    {#if cover}
      <img src={cover} alt="" data-tauri-drag-region />
    {:else}
      <span class="noart" data-tauri-drag-region>♪</span>
    {/if}
    <span class="hole" data-tauri-drag-region></span>
  </div>
  <span class="shine" aria-hidden="true"></span>

  <div class="main" data-tauri-drag-region>
    <div class="lcd" data-tauri-drag-region>
      <div class="row1" data-tauri-drag-region>
        <span class="state" aria-hidden="true">{playing ? "▶" : st?.track ? "❚❚" : "■"}</span>
        <div class="marquee" bind:clientWidth={titleBox} data-tauri-drag-region>
          <span class="title" class:scroll bind:clientWidth={titleText} data-tauri-drag-region>
            {title}{#if scroll}<span class="gap">•</span>{title}{/if}
          </span>
        </div>
      </div>
      <div class="artist" data-tauri-drag-region>{artist}</div>
      <div class="row3">
        <span class="time">{lcdTime(position)}</span>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="seek" bind:this={seekEl} role="slider" tabindex="-1" aria-label={t.transport.seek} aria-valuenow={position} onclick={seek}>
          <div class="fill" style:width="{pct}%"></div>
        </div>
        <span class="time">{duration ? lcdTime(duration) : "--:--"}</span>
        <span class="vol" title={t.transport.volume}>
          <svg viewBox="0 0 12 10" aria-hidden="true"><path d="M0 3h2.5L6 0v10L2.5 7H0z" /></svg>{st?.muted ? "--" : String(volume).padStart(2, "0")}
        </span>
      </div>
    </div>

    <div class="keys">
      <button class="key" title={t.transport.previous} aria-label={t.transport.previous} onclick={() => commands.playerPrevious()}>
        <svg viewBox="0 0 16 16"><rect x="3" y="4" width="2" height="8" /><path d="M13 4v8L6 8z" /></svg>
      </button>
      <button class="key big" title={playing ? t.transport.pause : t.transport.play} aria-label={playing ? t.transport.pause : t.transport.play} onclick={() => commands.playerToggle()}>
        <svg viewBox="0 0 24 16"><path d="M3 3v10l7-5z" /><rect x="13" y="3" width="3" height="10" rx=".5" /><rect x="18" y="3" width="3" height="10" rx=".5" /></svg>
      </button>
      <button class="key" title={t.transport.next} aria-label={t.transport.next} onclick={() => commands.playerNext()}>
        <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
      </button>
      <button class="key" title={t.transport.stop} aria-label={t.transport.stop} onclick={() => commands.playerStop()}>
        <svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg>
      </button>
      <span class="spacer" data-tauri-drag-region></span>
      <div class="rocker" role="group" aria-label={t.transport.volume}>
        <button title="{t.transport.volume} −" aria-label="{t.transport.volume} −" onclick={() => vol(-5)}>−</button>
        <button title="{t.transport.volume} +" aria-label="{t.transport.volume} +" onclick={() => vol(5)}>+</button>
      </div>
    </div>
  </div>

  <div class="screws">
    <button title={t.window.minimize} aria-label={t.window.minimize} onclick={() => appWindow()?.minimize()}>
      <svg viewBox="0 0 8 8"><path d="M1.5 4h5" /></svg>
    </button>
    <button title={t.transport.full} aria-label={t.transport.full} onclick={() => ui.setCompact(false)}>
      <svg viewBox="0 0 8 8"><path d="M1.5 1.5h5v5h-5z" /></svg>
    </button>
    <button title={t.window.close} aria-label={t.window.close} onclick={() => appWindow()?.close()}>
      <svg viewBox="0 0 8 8"><path d="M2 2l4 4M6 2l-4 4" /></svg>
    </button>
  </div>
</div>
<!-- Bisel: brillo por dentro del borde, sin delineado oscuro. -->
<svg class="bevel" width={w} height={h} aria-hidden="true">
  <defs><clipPath id="compact-clip"><path d={outline} /></clipPath></defs>
  <path d={outline} bind:this={pathEl} clip-path="url(#compact-clip)" />
</svg>
</div>

<style>
  .frame {
    position: absolute;
    inset: 0;
    /* La sombra sigue la silueta (se aplica después del recorte). */
    filter: drop-shadow(0 2px 3px rgb(0 0 0 / 0.45));
  }
  .bevel {
    position: absolute;
    inset: 0;
    pointer-events: none;
    fill: none;
    stroke: rgb(255 255 255 / 0.85);
    stroke-width: 3;
  }
  /* Cuerpo de aluminio cepillado, recortado con la silueta. */
  .device {
    position: absolute;
    inset: 0;
    background:
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.06) 0 1px, rgb(0 0 0 / 0.025) 1px 2px),
      linear-gradient(to bottom, #f7f8fa 0%, #dfe2e7 30%, #c9cdd4 70%, #b3b8c1 100%);
    user-select: none;
  }
  /* Reflejo diagonal del metal. */
  .shine {
    position: absolute;
    inset: 0;
    background: linear-gradient(115deg, transparent 30%, rgb(255 255 255 / 0.35) 45%, transparent 60%);
    pointer-events: none;
  }

  /* --- Disco con la portada --- */
  /* El disco ocupa el lóbulo de la izquierda. */
  .disc {
    position: absolute;
    left: 18px;
    top: calc(50% - 54px);
    width: 108px;
    height: 108px;
    border-radius: 50%;
    padding: 3px;
    background: conic-gradient(from 0deg, #d9dde3, #9aa1ab, #eef0f3, #9aa1ab, #d9dde3);
    box-shadow: 0 2px 4px rgb(0 0 0 / 0.45), inset 0 0 0 1px rgb(0 0 0 / 0.25);
  }
  .disc img,
  .noart {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    background: radial-gradient(circle, #2a3a66, #0c1630);
    font-size: 34px;
    color: #9fc7ff;
  }
  .spin img,
  .spin .noart {
    animation: spin 6s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  /* Agujero central y brillo fijo encima (no gira). */
  .hole {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 18px;
    height: 18px;
    margin: -9px 0 0 -9px;
    border-radius: 50%;
    background: radial-gradient(circle, #6d737d 0 35%, #e6e9ed 40% 70%, #9aa1ab 75%);
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.3);
  }
  .disc::after {
    content: "";
    position: absolute;
    inset: 3px;
    border-radius: 50%;
    background: linear-gradient(140deg, rgb(255 255 255 / 0.35) 0%, transparent 40%, transparent 70%, rgb(0 0 0 / 0.2));
    pointer-events: none;
  }

  /* Pantalla y botones en la banda del cuerpo. */
  .main {
    position: absolute;
    left: 152px;
    right: 16px;
    top: 30px;
    /* Botones algo por encima del borde inferior del cuerpo. */
    bottom: 26px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  /* --- Pantalla LCD azul retroiluminada --- */
  .lcd {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px 6px;
    border-radius: 6px;
    border: 2px solid #6f7580;
    border-top-color: #5a606a;
    border-bottom-color: #e9ecf0;
    background:
      repeating-linear-gradient(0deg, rgb(0 0 30 / 0.12) 0 1px, transparent 1px 3px),
      radial-gradient(ellipse at 50% 30%, #4f8cf5 0%, #2a5fd8 55%, #1a3fa6 100%);
    box-shadow: inset 0 2px 6px rgb(0 0 40 / 0.6);
    color: #f4f8ff;
    font-family: Consolas, "Lucida Console", "Courier New", monospace;
    text-shadow: 0 0 4px rgb(190 225 255 / 0.85);
    overflow: hidden;
  }
  .row1 {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .state {
    flex: none;
    font-size: 10px;
  }
  .marquee {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .title {
    display: inline-block;
    /* Su ancho natural: así se sabe si cabe o hay que desplazarlo. */
    width: max-content;
    font-weight: bold;
    font-size: 14px;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }
  .title.scroll {
    animation: marquee 14s linear infinite;
  }
  .gap {
    padding: 0 18px;
  }
  @keyframes marquee {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(-50%);
    }
  }
  .artist {
    font-size: 12px;
    letter-spacing: 0.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.9;
  }
  .row3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
    font-size: 11px;
  }
  .time {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  /* Barra segmentada, como los LCD de la época. */
  .seek {
    position: relative;
    flex: 1;
    height: 7px;
    background: rgb(0 0 40 / 0.35);
    cursor: pointer;
  }
  .fill {
    height: 100%;
    background: repeating-linear-gradient(90deg, #f4f8ff 0 4px, transparent 4px 6px);
    box-shadow: 0 0 4px rgb(190 225 255 / 0.7);
  }
  .vol {
    flex: none;
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .vol svg {
    width: 10px;
    height: 9px;
    fill: currentColor;
  }

  /* --- Botones físicos --- */
  .keys {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .key,
  .rocker button {
    display: grid;
    place-items: center;
    height: 24px;
    padding: 0;
    border: 1px solid #7e848e;
    border-radius: 12px;
    background: linear-gradient(to bottom, #ffffff 0%, #e4e7ec 45%, #c3c8d0 100%);
    box-shadow: inset 0 1px 0 #fff, 0 2px 0 #8d939d, 0 3px 3px rgb(0 0 0 / 0.3);
    color: #3a3f48;
    cursor: pointer;
    transition: transform 0.05s, box-shadow 0.05s;
  }
  .key {
    width: 36px;
  }
  .key.big {
    width: 54px;
  }
  .key svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }
  .key.big svg {
    width: 22px;
    height: 15px;
  }
  /* Al pulsar, el botón se hunde. */
  .key:active,
  .rocker button:active {
    transform: translateY(2px);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.25), 0 0 0 #8d939d;
    background: linear-gradient(to bottom, #d9dde3, #eef0f3);
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  .rocker {
    display: flex;
  }
  .rocker button {
    width: 26px;
    font: bold 14px/1 var(--font-ui);
  }
  .rocker button:first-child {
    border-radius: 12px 0 0 12px;
  }
  .rocker button:last-child {
    border-radius: 0 12px 12px 0;
    border-left: 0;
  }

  /* --- Botoncitos de ventana, como tornillos --- */
  /* En la pestaña que sube arriba a la derecha. */
  .screws {
    position: absolute;
    /* Algo separados del borde superior de la pestaña. */
    top: 11px;
    right: 22px;
    display: flex;
    gap: 4px;
  }
  .screws button {
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid #80868f;
    background: radial-gradient(circle at 40% 35%, #ffffff, #c8ccd3 60%, #9aa1ab);
    box-shadow: 0 1px 1px rgb(0 0 0 / 0.3);
    cursor: pointer;
  }
  .screws svg {
    width: 8px;
    height: 8px;
    fill: none;
    stroke: #3a3f48;
    stroke-width: 1.4;
  }
  .screws button:hover {
    filter: brightness(1.08);
  }
  .screws button:last-child:hover {
    background: radial-gradient(circle at 40% 35%, #ffd5c8, #e06a45 60%, #a8360f);
  }
</style>
