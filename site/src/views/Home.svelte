<script lang="ts">
  // Portada con la forma de «Reproduciendo»: visualización a la izquierda y, en la
  // lista de reproducción, música CC0 que suena de verdad.
  import Viz from "../Viz.svelte";
  import { PRESETS } from "../viz";
  import { TRACKS, fmt } from "../content";
  import { release, site } from "../state.svelte";
  import { current, play, player } from "../player.svelte";

  const total = TRACKS.reduce((s, f) => s + f.dur, 0);
  const track = $derived(current());
  const step = (d: number) => (site.preset = (site.preset + d + PRESETS.length) % PRESETS.length);
</script>

<div class="now">
  <div class="center">
    <div class="head">
      <span class="artist">YouTube Music · Windows Media Player 9 · Windows XP</span>
      <h1 class="title">XP Music</h1>
    </div>
    <div class="stage">
      {#key site.preset}<Viz make={PRESETS[site.preset]!.make} />{/key}
      <div class="pitch">
        <p>Reproductor de escritorio para <b>YouTube Music</b> con el aspecto de <b>Windows Media Player 9</b> en Windows XP.</p>
        <p class="sub">
          Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta; visualizaciones, modo compacto y máscaras de WMP.
          Para Windows 10/11, y en pruebas para Linux y macOS. Gratis y de código abierto.
        </p>
        <div class="cta">
          <a class="xp-btn" href="#/descargar">Descargar{release.data ? ` ${release.data.tag}` : ""}</a>
          <a class="xp-btn" href="#/capturas">Ver capturas</a>
        </div>
      </div>
    </div>
    <div class="vizbar">
      <div class="steps">
        <button title="Visualización anterior" aria-label="Visualización anterior" onclick={() => step(-1)}>
          <svg viewBox="0 0 10 10"><path d="M7 1v8L2 5z" /></svg>
        </button>
        <button title="Visualización siguiente" aria-label="Visualización siguiente" onclick={() => step(1)}>
          <svg viewBox="0 0 10 10"><path d="M3 1v8l5-4z" /></svg>
        </button>
      </div>
      <span class="preset">{PRESETS[site.preset]!.name}</span>
      <span class="grow"></span>
      <a class="album" href="#/docs">Documentación</a>
    </div>
  </div>

  <aside class="list">
    <div class="list-head"><span>Lista de reproducción actual</span><span class="n">{TRACKS.length}</span></div>
    <div class="cols"><span>Título</span><span>Artista</span><span>Dur.</span></div>
    <ol>
      {#each TRACKS as f, i (f.file)}
        <li>
          <button class:current={i === player.track} aria-current={i === player.track ? "true" : undefined} title="Reproducir «{f.title}»" onclick={() => play(i)}>
            <span class="t">{f.title}</span><span class="c">{f.artist}</span><span class="d">{fmt(f.dur)}</span>
          </button>
        </li>
      {/each}
    </ol>
    <p class="desc">«{track.title}» de {track.artist}. Dominio público (CC0): <a href={track.src}>origen</a>.</p>
    <div class="list-foot"><span>Música libre (CC0)</span><span>Tiempo total: {fmt(total)}</span></div>
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
    display: flex;
    flex-direction: column;
    padding: 6px 10px;
  }
  .artist {
    color: #c8c8c8;
    font-size: 12px;
  }
  .title {
    margin: 0;
    font: bold 17px var(--font-title);
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 240px;
    margin: 0 6px;
    overflow: hidden;
    background: #000;
    border: 1px solid #1d1d1d;
  }
  .pitch {
    position: absolute;
    left: 14px;
    bottom: 14px;
    max-width: min(420px, calc(100% - 28px));
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.15);
    border-radius: 4px;
    background: rgb(0 0 0 / 0.6);
    font-size: 13px;
    line-height: 1.4;
  }
  .pitch p {
    margin: 0 0 6px;
  }
  .pitch .sub {
    color: #bbb;
    font-size: 11px;
  }
  .cta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .cta .xp-btn {
    display: inline-grid;
    place-items: center;
    color: #000;
    text-decoration: none;
  }
  .vizbar {
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
    height: 24px;
    padding: 0 10px;
    border: 1px solid #2a2a2a;
    background: linear-gradient(to right, #1a1a1a, #000);
    color: #f0c040;
    font-weight: bold;
    text-decoration: none;
  }
  .album:hover {
    text-decoration: underline;
  }

  /* Lista de reproducción oscura de WMP 9. */
  .list {
    width: 280px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: #000;
    color: var(--dark-list-text);
  }
  .list-head {
    display: flex;
    justify-content: space-between;
    margin: 6px;
    padding: 4px 8px;
    border: 1px solid var(--btn-soft-border);
    border-radius: 3px;
    background: linear-gradient(to bottom, var(--btn-1), var(--btn-3));
    color: #000;
    font-weight: bold;
  }
  .n {
    font-weight: normal;
  }
  .cols,
  .list button {
    display: grid;
    grid-template-columns: 1fr 86px 34px;
    gap: 6px;
    padding: 2px 8px;
  }
  .cols {
    color: #9fb4e0;
    border-bottom: 1px solid #222;
  }
  ol {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow: auto;
  }
  .list button {
    width: 100%;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: default;
  }
  .list li:nth-child(even) button {
    background: #0b0b0b;
  }
  .list button:hover {
    background: var(--dark-list-sel);
  }
  .list button.current {
    color: var(--dark-list-current);
    font-weight: bold;
  }
  .t,
  .c {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .d {
    text-align: right;
  }
  .desc {
    margin: 0;
    padding: 6px 8px;
    min-height: 3.2em;
    border-top: 1px solid #222;
    color: #ddd;
  }
  .desc a {
    color: #f0c040;
  }
  .list-foot {
    display: flex;
    justify-content: space-between;
    padding: 6px 8px;
    border-top: 1px solid #222;
  }

  @media (max-width: 759px) {
    .now {
      flex-direction: column;
      overflow: auto;
    }
    .center {
      border-right: 0;
    }
    .stage {
      flex: none;
      height: 340px;
    }
    .list {
      width: auto;
    }
    ol {
      overflow: visible;
    }
  }
</style>
