<script lang="ts">
  // Descargas de la última release (API de GitHub), con la forma de la Biblioteca.
  // Una tarjeta por sistema con lo que traiga la release; la del visitante, la primera.
  import logo from "../../../src/assets/logo.webp";
  import { RELEASES, REPO } from "../content";
  import { release, type Asset } from "../state.svelte";
  import { OS_NAME, archName, detectOs, type Os } from "../builds";

  const mine = detectOs(navigator.userAgent);
  const ALL: Os[] = ["windows", "macos", "linux"];
  const order = mine ? [mine, ...ALL.filter((o) => o !== mine)] : ALL;

  const INTRO: Record<Os, string> = {
    windows: "Instalador para Windows 10/11. Se actualiza solo: al arrancar te avisa si hay una versión nueva.",
    macos: "Experimental. Imagen de disco: ábrela y arrastra XP Music a Aplicaciones. No se actualiza sola: para cambiar de versión, descarga el .dmg nuevo.",
    linux: "Experimental. Necesita una sesión con audio y un Secret Service (GNOME Keyring o KeePassXC).",
  };

  const assets = $derived(release.data?.assets ?? []);
  const cards = $derived(
    order
      .map((os) => {
        const own = assets.filter((a) => a.os === os);
        // En macOS, Apple Silicon primero: es lo que llevan los Mac actuales.
        const arches = (os === "macos" ? (["arm64", "x64"] as const) : (["x64", "arm64"] as const))
          .map((arch) => ({ arch, files: own.filter((a) => a.arch === arch) }))
          .filter((r) => r.files.length);
        return { os, arches };
      })
      .filter((c) => c.arches.length),
  );
  const table = $derived(cards.flatMap((c) => c.arches.flatMap((r) => r.files)));
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

  {#if cards.length}
    <div class="cards">
      {#each cards as c (c.os)}
        <fieldset class:mine={c.os === mine}>
          <legend>{OS_NAME[c.os]}{c.os === mine ? " · tu sistema" : ""}</legend>
          <p>{INTRO[c.os]}</p>
          {#each c.arches as r (r.arch)}
            <div class="arch">
              <span class="arch-name">{archName(c.os, r.arch)}</span>
              <div class="row">
                {#each r.files as a (a.name)}
                  <a class="xp-btn big" href={a.url} title={a.name}>{a.kind} <span class="size">{mb(a)}</span></a>
                {/each}
              </div>
            </div>
          {/each}
          <p class="hint">
            {#if c.os === "windows"}Si SmartScreen avisa: «Más información» → «Ejecutar de todas formas».{:else if c.os === "macos"}La primera vez macOS la bloquea: Ajustes del Sistema → Privacidad y seguridad → «Abrir igualmente».{/if}
            <a href="#/docs/instalar">Instrucciones de instalación</a>
          </p>
        </fieldset>
      {/each}
    </div>
  {:else if release.data || release.failed}
    <fieldset>
      <legend>Descargas</legend>
      <p>No se pudo leer la lista de archivos. Están todos en la página de la última versión.</p>
      <a class="xp-btn big" href={RELEASES}>Abrir la última versión en GitHub</a>
    </fieldset>
  {/if}

  {#if table.length}
    <h2>Todos los archivos</h2>
    <div class="scroll">
      <table class="lv">
        <thead><tr><th>Sistema</th><th>Archivo</th><th class="num">Tamaño</th><th>SHA-256</th></tr></thead>
        <tbody>
          {#each table as a (a.name)}
            <tr>
              <td>{OS_NAME[a.os]} {archName(a.os, a.arch)}</td>
              <td><a href={a.url}>{a.name}</a></td>
              <td class="num">{mb(a)}</td>
              <td>{#if a.sha256}<code class="sha">{a.sha256}</code>{:else}<span class="muted">—</span>{/if}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  <p class="muted">Versiones anteriores y código fuente en <a href="{REPO}/releases">GitHub Releases</a>.</p>
</div>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(270px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }
  fieldset {
    background: var(--surface-light);
  }
  /* Borde más grueso, no box-shadow: la sombra no deja hueco para la leyenda y la tacha. */
  fieldset.mine {
    border: 2px solid var(--selection);
  }
  fieldset p {
    margin: 4px 0 10px;
  }
  .arch {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .arch-name {
    width: 86px;
    flex: none;
    font-weight: bold;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .big {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    font-weight: bold;
  }
  .size {
    color: var(--text-muted);
    font-weight: normal;
    font-size: 11px;
  }
  .hint {
    color: var(--text-muted);
  }
  .scroll {
    max-width: 100%;
    overflow-x: auto;
  }
  .lv {
    max-width: none;
  }
  /* Clic = seleccionar el hash entero, para copiarlo y compararlo. */
  .sha {
    font: 11px Consolas, "Lucida Console", monospace;
    user-select: all;
    -webkit-user-select: all;
  }
</style>
