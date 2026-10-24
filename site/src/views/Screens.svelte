<script lang="ts">
  // App screenshots, each in an XP window. Each language has its own, taken with that UI language.
  import nowPlaying from "../../../docs/now-playing.jpg";
  import library from "../../../docs/library.jpg";
  import nowPlayingEs from "../../../docs/now-playing.es.jpg";
  import libraryEs from "../../../docs/library.es.jpg";
  import { language, t } from "../i18n.svelte";

  const images = { en: [nowPlaying, library], es: [nowPlayingEs, libraryEs] };
  const shots = $derived(t.screenshots.shots.map((s, i) => ({ ...s, src: images[language()][i]! })));
</script>

<div class="view">
  <header class="vhead">
    <div>
      <h1>{t.sections.screenshots}</h1>
      <span class="muted">{t.screenshots.items(shots.length)}</span>
    </div>
  </header>
  {#each shots as s (s.title)}
    <figure class="xpwin">
      <figcaption>{s.title} - XP Music</figcaption>
      <a href={s.src} target="_blank" rel="noopener"><img src={s.src} alt={`${s.title}: ${s.text}`} width="1100" height="720" loading="lazy" /></a>
      <p>{s.text}</p>
    </figure>
  {/each}
</div>

<style>
  .xpwin {
    max-width: 900px;
    margin: 0 0 18px;
    background: var(--surface);
    border: 3px solid var(--frame);
    border-top: 0;
    border-radius: 8px 8px 0 0;
    box-shadow: 3px 3px 8px rgb(0 0 0 / 0.35);
  }
  figcaption {
    height: 26px;
    margin: 0 -3px;
    padding: 0 8px;
    line-height: 26px;
    border-radius: 7px 7px 0 0;
    background: var(--title-grad);
    color: var(--title-text);
    font: bold 12px/26px var(--font-title);
    text-shadow: var(--title-text-shadow);
  }
  img {
    display: block;
    width: 100%;
    height: auto;
  }
  p {
    margin: 0;
    padding: 6px 8px;
  }
</style>
