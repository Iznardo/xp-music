<script lang="ts">
  // Downloads of the latest release (GitHub API), shaped like the Library.
  // One card per system with whatever the release has; the visitor's first.
  import logo from "../../../src/assets/logo.webp";
  import { RELEASES, REPO } from "../content";
  import { release, type Asset } from "../state.svelte";
  import { OS_NAME, archName, detectOs, kindLabel, type Os } from "../builds";
  import { locale, t } from "../i18n.svelte";

  const mine = detectOs(navigator.userAgent);
  const ALL: Os[] = ["windows", "macos", "linux"];
  const order = mine ? [mine, ...ALL.filter((o) => o !== mine)] : ALL;

  const assets = $derived(release.data?.assets ?? []);
  const cards = $derived(
    order
      .map((os) => {
        const own = assets.filter((a) => a.os === os);
        // On macOS, Apple Silicon first: it is what current Macs have.
        const arches = (os === "macos" ? (["arm64", "x64"] as const) : (["x64", "arm64"] as const))
          .map((arch) => ({ arch, files: own.filter((a) => a.arch === arch) }))
          .filter((r) => r.files.length);
        return { os, arches };
      })
      .filter((c) => c.arches.length),
  );
  const table = $derived(cards.flatMap((c) => c.arches.flatMap((r) => r.files)));
  const mb = (a: Asset) => `${(a.size / 1048576).toFixed(1)} MB`;
  const date = (iso: string) => new Date(iso).toLocaleDateString(locale(), { day: "numeric", month: "long", year: "numeric" });
</script>

<div class="view">
  <header class="vhead">
    <img src={logo} alt="" width="40" height="40" />
    <div>
      <h1>{t.download.title}</h1>
      <span class="muted">
        {#if release.data}{t.download.version(release.data.tag, date(release.data.published))}{:else if release.failed}{t.download.failed}{:else}{t.download.loading}{/if}
      </span>
    </div>
    <span class="grow"></span>
    <a class="xp-btn" href={release.data?.url ?? RELEASES}>{t.download.notes}</a>
  </header>

  {#if cards.length}
    <div class="cards">
      {#each cards as c (c.os)}
        <fieldset class:mine={c.os === mine}>
          <legend>{OS_NAME[c.os]}{c.os === mine ? t.download.mine : ""}</legend>
          <p>{t.download.intro[c.os]}</p>
          {#each c.arches as r (r.arch)}
            <div class="arch">
              <span class="arch-name">{archName(c.os, r.arch)}</span>
              <div class="row">
                {#each r.files as a (a.name)}
                  <a class="xp-btn big" href={a.url} title={a.name}>{kindLabel(a.kind, t.download.installer)} <span class="size">{mb(a)}</span></a>
                {/each}
              </div>
            </div>
          {/each}
          <p class="hint">
            {#if c.os === "windows"}{t.download.smartScreen}{:else if c.os === "macos"}{t.download.gatekeeper}{/if}
            <a href="#/docs/install">{t.download.instructions}</a>
          </p>
        </fieldset>
      {/each}
    </div>
  {:else if release.data || release.failed}
    <fieldset>
      <legend>{t.download.downloads}</legend>
      <p>{t.download.listFailed}</p>
      <a class="xp-btn big" href={RELEASES}>{t.download.openRelease}</a>
    </fieldset>
  {/if}

  {#if table.length}
    <h2>{t.download.all}</h2>
    <div class="scroll">
      <table class="lv">
        <thead><tr><th>{t.download.colSystem}</th><th>{t.download.colFile}</th><th class="num">{t.download.colSize}</th><th>SHA-256</th></tr></thead>
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
  <p class="muted">{t.download.older} <a href="{REPO}/releases">GitHub Releases</a>.</p>
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
  /* Thicker border, not box-shadow: the shadow leaves no gap for the legend and strikes it through. */
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
  /* Click = select the whole hash, to copy and compare it. */
  .sha {
    font: 11px Consolas, "Lucida Console", monospace;
    user-select: all;
    -webkit-user-select: all;
  }
</style>
