<script lang="ts">
  // Descargas de la última release (API de GitHub), con la forma de la Biblioteca.
  import logo from "../../../src/assets/logo.webp";
  import { RELEASES } from "../content";
  import { release, type Asset } from "../state.svelte";

  const assets = $derived(release.data?.assets.filter((a) => !/\.(sig|json)$/.test(a.name)) ?? []);
  const find = (re: RegExp) => assets.find((a) => re.test(a.name));
  const exe = $derived(find(/\.exe$/i));
  const deb = $derived(find(/\.deb$/i));
  const appimage = $derived(find(/\.AppImage$/i));
  const mb = (a: Asset) => `${(a.size / 1048576).toFixed(1)} MB`;
</script>

<div class="view">
  <header class="vhead">
    <img src={logo} alt="" width="40" height="40" />
    <div>
      <h1>Descargar XP Music</h1>
      <span class="muted">
        {#if release.data}Versión {release.data.tag} · {release.data.date}{:else if release.failed}No se pudo consultar la última versión.{:else}Buscando la última versión…{/if}
      </span>
    </div>
    <span class="grow"></span>
    <a class="xp-btn" href={release.data?.url ?? RELEASES}>Notas de la versión</a>
  </header>

  <div class="cards">
    <fieldset>
      <legend>Windows 10/11</legend>
      <p>Instalador para x64. Se actualiza solo: al arrancar te avisa si hay una versión nueva.</p>
      <a class="xp-btn big" href={exe?.url ?? RELEASES}>Descargar instalador{exe ? ` (${mb(exe)})` : ""}</a>
      <p class="hint">Si SmartScreen avisa: «Más información» → «Ejecutar de todas formas». <a href="#/docs/instalar">Más detalles</a></p>
    </fieldset>
    <fieldset>
      <legend>Linux x86_64 (experimental)</legend>
      <p>Compilado en Ubuntu 24.04. Necesita audio y un Secret Service (GNOME Keyring o KeePassXC).</p>
      <div class="row">
        <a class="xp-btn big" href={deb?.url ?? RELEASES}>.deb{deb ? ` (${mb(deb)})` : ""}</a>
        <a class="xp-btn big" href={appimage?.url ?? RELEASES}>AppImage{appimage ? ` (${mb(appimage)})` : ""}</a>
      </div>
      <p class="hint"><a href="#/docs/instalar">Instrucciones para Linux</a></p>
    </fieldset>
  </div>

  {#if assets.length}
    <h2>Todos los archivos</h2>
    <table class="lv">
      <thead><tr><th>Nombre</th><th class="num">Tamaño</th><th class="num">Descargas</th></tr></thead>
      <tbody>
        {#each assets as a (a.name)}
          <tr><td><a href={a.url}>{a.name}</a></td><td class="num">{mb(a)}</td><td class="num">{a.downloads.toLocaleString("es-ES")}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
  <p class="muted">Versiones anteriores y código fuente en <a href={RELEASES}>GitHub Releases</a>.</p>
</div>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }
  fieldset {
    background: var(--surface-light);
  }
  fieldset p {
    margin: 4px 0 10px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .big {
    display: inline-grid;
    place-items: center;
    height: 28px;
    font-weight: bold;
  }
  .hint {
    color: var(--text-muted);
  }
</style>
