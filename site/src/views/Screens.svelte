<script lang="ts">
  // Capturas de la app, cada una en una ventana de XP.
  import biblioteca from "../../../docs/biblioteca.jpg";
  import reproduciendo from "../../../docs/reproduciendo.jpg";

  const shots = [
    { src: reproduciendo, title: "Reproduciendo", text: "La visualización «Ambiente: Agua» y la lista de reproducción actual." },
    { src: biblioteca, title: "Biblioteca", text: "Tus listas de YouTube Music en el árbol de la izquierda y las canciones a la derecha." },
  ];
</script>

<div class="view">
  <header class="vhead">
    <div>
      <h1>Capturas</h1>
      <span class="muted">{shots.length} elementos</span>
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
