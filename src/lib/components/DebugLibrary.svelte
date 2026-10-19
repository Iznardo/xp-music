<script lang="ts">
  // Panel provisional de cuenta y biblioteca (003). Lo sustituye la UI de 004.
  import { onMount } from "svelte";
  import {
    commands,
    type AppError,
    type BrowserProfile,
    type MediaItem,
    type PlaylistSummary,
    type SessionAccount,
    type Track,
  } from "../bindings";
  import { auth } from "../stores/auth.svelte";
  import { thumbSrc } from "../thumb";

  let browsers = $state<BrowserProfile[]>([]);
  let accounts = $state<SessionAccount[]>([]);
  let pasted = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  let playlists = $state<PlaylistSummary[]>([]);
  let query = $state("");
  let results = $state<MediaItem[]>([]);

  onMount(async () => {
    await auth.init();
    browsers = await commands.authDetectBrowsers();
    if (auth.signedIn) loadLibrary();
  });

  async function guard<T>(p: Promise<{ status: "ok"; data: T } | { status: "error"; error: AppError }>): Promise<T | null> {
    busy = true;
    error = null;
    try {
      const r = await p;
      if (r.status === "error") {
        error = r.error.message;
        return null;
      }
      return r.data;
    } finally {
      busy = false;
    }
  }

  async function scan(profile: string) {
    accounts = (await guard(commands.authScanBrowser(profile))) ?? [];
  }

  async function scanPasted() {
    accounts = (await guard(commands.authScanCookie(pasted))) ?? [];
    pasted = "";
  }

  async function choose(a: SessionAccount) {
    if (await guard(commands.authSelectAccount(a))) {
      accounts = [];
      loadLibrary();
    }
  }

  async function loadLibrary() {
    playlists = (await guard(commands.ytLibraryPlaylists())) ?? [];
  }

  async function playPlaylist(p: PlaylistSummary) {
    const d = await guard(commands.ytPlaylist(p.id));
    if (d?.tracks.items.length) await commands.queueSet(d.tracks.items, 0, { Playlist: p.id });
  }

  async function search() {
    if (!query.trim()) return;
    results = (await guard(commands.ytSearch(query, "All")))?.items ?? [];
  }

  async function playTrack(t: Track) {
    // Como en YT Music: la canción y después su radio.
    const radio = (await guard(commands.ytRadio(t.id))) ?? [];
    const rest = radio.filter((r) => r.id !== t.id);
    await commands.queueSet([t, ...rest], 0, { Radio: t.id });
  }

  function label(i: MediaItem): string {
    if (i.Track) return `♪ ${i.Track.title} — ${i.Track.artists.map((a) => a.name).join(", ")}`;
    if (i.Album) return `💿 ${i.Album.title}`;
    if (i.Artist) return `👤 ${i.Artist.name}`;
    return `📃 ${i.Playlist?.title ?? ""}`;
  }

  async function open(i: MediaItem) {
    if (i.Track) return playTrack(i.Track);
    if (i.Playlist) return playPlaylist(i.Playlist);
    if (i.Album) {
      const id = i.Album.id;
      const a = await guard(commands.ytAlbum(id));
      if (a?.tracks.length) await commands.queueSet(a.tracks, 0, { Album: id });
    }
    if (i.Artist) {
      const a = await guard(commands.ytArtist(i.Artist.id));
      if (a?.topSongs.length) await commands.queueSet(a.topSongs, 0, "Library");
    }
  }
</script>

<fieldset class="lib">
  <legend>Cuenta y biblioteca (depuración)</legend>

  {#if auth.signedIn}
    <div class="row">
      <span>Sesión: <b>{auth.accountName}</b></span>
      <button onclick={() => commands.authLogout()}>Cerrar sesión</button>
    </div>
  {:else}
    {#if auth.accountName}<p class="err">La sesión de {auth.accountName} caducó.</p>{/if}
    <div class="row">
      <span>Importar sesión de:</span>
      {#each browsers as b}
        <button disabled={busy} onclick={() => scan(b.path)}>{b.browser} ({b.name})</button>
      {:else}
        <i>no se encontró ningún navegador compatible</i>
      {/each}
    </div>
    <div class="row">
      <input class="grow" type="password" placeholder="…o pega aquí la cabecera Cookie" bind:value={pasted} />
      <button disabled={busy || !pasted} onclick={scanPasted}>Usar</button>
    </div>
    {#each accounts as a}
      <button class="account" onclick={() => choose(a)}>
        {#if a.photo}<img src={thumbSrc([{ url: a.photo, width: 108, height: 108 }], 108)} alt="" />{/if}
        <b>{a.name}</b>
        {a.handle ?? ""}
        <small>{typeof a.source === "object" && a.source.Browser ? a.source.Browser.containerName : "cookie pegada"}</small>
      </button>
    {/each}
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
  {#if busy}<p>Cargando…</p>{/if}

  {#if auth.signedIn}
    <div class="row">
      <input class="grow" placeholder="Buscar en YouTube Music" bind:value={query} onkeydown={(e) => e.key === "Enter" && search()} />
      <button onclick={search}>Buscar</button>
    </div>
    <ul class="list">
      {#each results as r}
        <li><button class="link" ondblclick={() => open(r)}>{label(r)}</button></li>
      {/each}
    </ul>
    <p>Tus playlists (doble clic para reproducir):</p>
    <ul class="list">
      {#each playlists as p}
        <li>
          <button class="link" ondblclick={() => playPlaylist(p)}>
            {#if thumbSrc(p.thumbnails, 32)}<img src={thumbSrc(p.thumbnails, 32)} alt="" />{/if}
            {p.title} <small>{p.trackCount ?? ""}</small>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</fieldset>

<style>
  .lib {
    width: 100%;
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
  }
  .err {
    color: #b00;
    margin: 0;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 6px;
    text-align: left;
  }
  .account img {
    width: 24px;
    height: 24px;
    border-radius: 50%;
  }
  .list {
    margin: 0;
    padding: 2px;
    list-style: none;
    max-height: 150px;
    overflow: auto;
    background: #fff;
    border: 1px solid #7f9db9;
  }
  .link {
    border: 0;
    background: none;
    padding: 1px 2px;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    text-align: left;
    cursor: default;
  }
  .link:hover {
    background: var(--selection);
    color: #fff;
  }
  .link img {
    width: 16px;
    height: 16px;
  }
</style>
