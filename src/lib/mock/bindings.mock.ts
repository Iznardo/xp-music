// Stand-in for `bindings.ts` to preview the UI in a regular browser
// (`npm run dev:mock`). Made-up data and a simulated player; it doesn't talk to Rust.

import type {
  AlbumDetail,
  AlbumSummary,
  AppError,
  ArtistDetail,
  ArtistSummary,
  AudioDevice,
  AuthStatus,
  BrowserProfile,
  InsertPosition,
  LikeStatus,
  MediaItem,
  Page,
  PlayerState,
  PlaylistDetail,
  PlaylistSummary,
  QueueSnapshot,
  QueueSource,
  RepeatMode,
  SearchFilter,
  Section,
  SessionAccount,
  Settings,
  Thumbnail,
  Track,
} from "../bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };
const ok = <T>(data: T): Promise<Result<T>> => new Promise((r) => setTimeout(() => r({ status: "ok", data }), 120));

// ------------------------------------------------------------------ events

type Cb<T> = (e: { event: string; id: number; payload: T }) => void;
function makeEvent<T>(name: string) {
  const subs = new Set<Cb<T>>();
  return {
    listen: (cb: Cb<T>) => {
      subs.add(cb);
      return Promise.resolve(() => subs.delete(cb));
    },
    once: (cb: Cb<T>) => {
      const w: Cb<T> = (e) => {
        subs.delete(w);
        cb(e);
      };
      subs.add(w);
      return Promise.resolve(() => subs.delete(w));
    },
    emit: (payload: T) => {
      subs.forEach((cb) => cb({ event: name, id: 0, payload }));
      return Promise.resolve();
    },
  };
}

export const events = {
  authChanged: makeEvent<AuthStatus>("auth://changed"),
  playerError: makeEvent<AppError>("player://error"),
  playerPosition: makeEvent<{ positionMs: number; bufferedMs: number }>("player://position"),
  playerState: makeEvent<PlayerState>("player://state"),
  queueChanged: makeEvent<QueueSnapshot>("queue://changed"),
  settingsChanged: makeEvent<Settings>("settings://changed"),
  ytCacheUpdated: makeEvent<{ key: string }>("yt://cache-updated"),
  vizFrame: makeEvent<{ bands: number[]; peaks: number[]; wave: number[]; level: number; beat: number; bass: number }>("viz://frame"),
  updateAvailable: makeEvent<{ version: string; notes: string | null; date: string | null }>("update://available"),
};

// ------------------------------------------------------------------ data

const palette = ["#3a6ea5", "#a54a3a", "#3aa56b", "#8a3aa5", "#a5893a", "#3a9aa5", "#a53a7c", "#5b5b5b"];
/** A real cover for "Ruta 2004" (the song in the screenshots and the promo video). */
const ROUTE_2004_COVER = `<svg xmlns='http://www.w3.org/2000/svg' width='600' height='600' viewBox='0 0 600 600'>
<defs>
<linearGradient id='sky' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#1d1b5e'/><stop offset='.45' stop-color='#b2338f'/><stop offset='.62' stop-color='#ff7a3d'/></linearGradient>
<linearGradient id='sun' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#ffe66b'/><stop offset='1' stop-color='#ff4f6d'/></linearGradient>
<linearGradient id='chrome' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#ffffff'/><stop offset='.48' stop-color='#b8c4d6'/><stop offset='.52' stop-color='#4a5670'/><stop offset='1' stop-color='#e8eef8'/></linearGradient>
<clipPath id='half'><rect x='0' y='0' width='600' height='372'/></clipPath>
</defs>
<rect width='600' height='372' fill='url(#sky)'/>
<g clip-path='url(#half)'><circle cx='300' cy='372' r='150' fill='url(#sun)'/>
<g fill='#c43a8c'><rect x='140' y='300' width='320' height='7'/><rect x='140' y='322' width='320' height='9'/><rect x='140' y='344' width='320' height='12'/><rect x='140' y='364' width='320' height='8'/></g></g>
<rect y='372' width='600' height='228' fill='#14102e'/>
<g stroke='#ff4fd8' stroke-width='3' opacity='.85'>
<line x1='300' y1='372' x2='-300' y2='600'/><line x1='300' y1='372' x2='0' y2='600'/><line x1='300' y1='372' x2='150' y2='600'/><line x1='300' y1='372' x2='450' y2='600'/><line x1='300' y1='372' x2='600' y2='600'/><line x1='300' y1='372' x2='900' y2='600'/>
<line x1='0' y1='392' x2='600' y2='392'/><line x1='0' y1='420' x2='600' y2='420'/><line x1='0' y1='462' x2='600' y2='462'/><line x1='0' y1='524' x2='600' y2='524'/></g>
<polygon points='292,372 308,372 360,600 240,600' fill='#2a2350'/>
<g fill='#ffe66b'><rect x='297' y='400' width='6' height='18'/><rect x='296' y='452' width='8' height='30'/><rect x='294' y='530' width='12' height='50'/></g>
<text x='300' y='150' text-anchor='middle' font-family='Arial Black, Tahoma, sans-serif' font-weight='900' font-size='104' fill='url(#chrome)' stroke='#1d1b5e' stroke-width='4' letter-spacing='4'>RUTA</text>
<text x='300' y='250' text-anchor='middle' font-family='Arial Black, Tahoma, sans-serif' font-weight='900' font-size='104' fill='url(#chrome)' stroke='#1d1b5e' stroke-width='4' letter-spacing='4'>2004</text>
<text x='300' y='300' text-anchor='middle' font-family='Tahoma, sans-serif' font-weight='bold' font-size='26' fill='#ffffff' letter-spacing='10'>DUO SATÉLITE</text>
</svg>`;

function cover(seed: number, label: string): Thumbnail[] {
  if (label === "Ruta") return [{ url: `data:image/svg+xml;utf8,${encodeURIComponent(ROUTE_2004_COVER)}`, width: 600, height: 600 }];
  const c = palette[seed % palette.length];
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='226' height='226'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='${c}'/><stop offset='1' stop-color='#111'/></linearGradient></defs><rect width='226' height='226' fill='url(#g)'/><text x='16' y='206' font-family='Tahoma' font-size='26' fill='#fff' opacity='.85'>${label}</text></svg>`;
  return [{ url: `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`, width: 226, height: 226 }];
}

const artists = ["Los Planetas Azules", "Marea Norte", "Duo Satélite", "Clara Vidal", "Orquesta Luna", "Nube Roja"];
const albums = ["Cielo de Cristal", "Noches de Radio", "Ruta 2004", "Ecos del Sur", "Siempre Verano", "Bahía"];
const titles = [
  "Luz de Neón", "Carretera Infinita", "Tu Voz en la Radio", "Bailando en el Andén", "Mar de Fondo", "Domingo Lento",
  "La Última Estación", "Calor de Agosto", "Telegrama", "Faro", "Ventanas Abiertas", "Polaroid",
  "Viento del Este", "Hora Dorada", "Cassette", "Sin Cobertura", "Madrugada", "Postales",
];

let seq = 0;
function track(i: number): Track {
  const a = i % artists.length;
  const al = i % albums.length;
  seq++;
  return {
    id: `mock${String(i).padStart(7, "0")}`,
    title: titles[i % titles.length]!,
    artists: [{ id: `UCmock${a}`, name: artists[a]! }],
    album: { id: `MPREbmock${al}`, name: albums[al]! },
    durationMs: 150_000 + ((i * 37_000) % 120_000),
    thumbnails: cover(al, albums[al]!.split(" ")[0]!),
    explicit: i % 7 === 3,
    likeStatus: i % 5 === 0 ? "Like" : "Indifferent",
    setVideoId: null,
  };
}
const allTracks = Array.from({ length: 60 }, (_, i) => track(i));

const playlists: PlaylistSummary[] = [
  { id: "VLLM", title: "Música que me gusta", author: null, trackCount: 39, thumbnails: cover(0, "♥") },
  ...["Verano 2004", "Para conducir", "Bachata clásica", "Estudiar", "Fiesta", "Relax", "Lista enorme"].map((t, i) => ({
    id: `VLPLmock${i}`,
    title: t,
    author: "Alex Rivera",
    trackCount: t === "Lista enorme" ? 3000 : 10 + i * 7,
    thumbnails: cover(i + 2, t.split(" ")[0]!),
  })),
];

const albumSummaries: AlbumSummary[] = albums.map((t, i) => ({
  id: `MPREbmock${i}`,
  title: t,
  artists: [{ id: `UCmock${i}`, name: artists[i]! }],
  subtitle: `Álbum • ${2001 + i}`,
  thumbnails: cover(i, t.split(" ")[0]!),
}));

const artistSummaries: ArtistSummary[] = artists.map((n, i) => ({
  id: `UCmock${i}`,
  name: n,
  subtitle: `${(i + 1) * 1.3}M oyentes mensuales`,
  thumbnails: cover(i + 3, n.split(" ")[0]!),
}));

/** Data from another library for screenshots (the promo video): JSON in localStorage "xpm.mock.data". */
try {
  const raw = localStorage.getItem("xpm.mock.data");
  if (raw) {
    const d = JSON.parse(raw) as { tracks: Track[]; albums: AlbumSummary[]; artists: ArtistSummary[] };
    allTracks.splice(0, allTracks.length, ...d.tracks);
    albumSummaries.splice(0, albumSummaries.length, ...d.albums);
    artistSummaries.splice(0, artistSummaries.length, ...d.artists);
  }
} catch {
  // No alternative data: the sample data stays.
}

// ------------------------------------------------------------------ simulated player

let settings: Settings = {
  theme: "Blue",
  language: null,
  volume: 0.8,
  normalizeVolume: true,
  audioQuality: "Aac128",
  outputDevice: null,
  closeToTray: true,
  startMinimized: false,
  launchAtLogin: false,
  reportHistory: true,
  visualization: "Water",
  xpSounds: true,
  notifications: true,
  pauseOnLock: false,
  discordPresence: false,
  hotkeys: { playPause: null, next: null, previous: null, show: null },
};

let queue: QueueSnapshot = { tracks: allTracks.slice(0, 12), currentIndex: 2, source: "Manual", autoplay: true, shuffle: false, repeat: "Off" };
let state: PlayerState = {
  status: "Paused",
  track: queue.tracks[2] ?? null,
  positionMs: 48_000,
  durationMs: queue.tracks[2]?.durationMs ?? null,
  volume: 0.8,
  muted: false,
  shuffle: false,
  repeat: "Off",
  error: null,
};

function emitState() {
  state = { ...state, shuffle: queue.shuffle, repeat: queue.repeat };
  events.playerState.emit(state);
}
function emitQueue() {
  events.queueChanged.emit(queue);
}
function playAt(i: number) {
  queue = { ...queue, currentIndex: i };
  const t = queue.tracks[i] ?? null;
  state = { ...state, track: t, positionMs: 0, durationMs: t?.durationMs ?? null, status: "Loading" };
  emitQueue();
  emitState();
  setTimeout(() => {
    state = { ...state, status: "Playing" };
    emitState();
  }, 400);
}
setInterval(() => {
  if (state.status !== "Playing") return;
  state.positionMs += 250;
  if (state.durationMs && state.positionMs >= state.durationMs) {
    const n = (queue.currentIndex ?? 0) + 1;
    if (n < queue.tracks.length) playAt(n);
    else {
      state = { ...state, status: "Idle", positionMs: 0 };
      emitState();
    }
    return;
  }
  events.playerPosition.emit({ positionMs: state.positionMs, bufferedMs: 30_000 });
}, 250);

// Made-up visualization frames: kick drum at 120 BPM, vocals in the mids and hi-hats.
let vizTimer: ReturnType<typeof setInterval> | undefined;
const peaks = new Array(64).fill(0);
function fakeViz() {
  const t = performance.now() / 1000;
  const beat = Math.exp(-((t * 2) % 1) * 6);
  const bands = Array.from({ length: 64 }, (_, i) => {
    const low = i < 10 ? beat * (1 - i / 14) : 0;
    const mid = 0.45 * Math.exp(-(((i - 26) / 9) ** 2)) * (0.7 + 0.3 * Math.sin(t * 3 + i));
    const hat = i > 44 ? 0.35 * Math.exp(-((t * 4) % 1) * 9) : 0;
    return Math.min(1, low + mid + hat + 0.05 * Math.random());
  });
  bands.forEach((b, i) => (peaks[i] = Math.max(b, peaks[i] - 0.01)));
  const wave = Array.from({ length: 256 }, (_, i) => 0.5 * beat * Math.sin(i * 0.15) + 0.2 * Math.sin(i * 0.9 + t * 20));
  const u = (x: number) => Math.round(Math.min(1, Math.max(0, x)) * 255);
  events.vizFrame.emit({
    bands: bands.map(u),
    peaks: peaks.map(u),
    wave: wave.map((x) => Math.round(Math.max(-1, Math.min(1, x)) * 127)),
    level: 0.3 + beat * 0.5,
    beat: beat > 0.9 ? 1 : beat * 0.8,
    bass: beat,
  });
}

let auth: AuthStatus = { SignedIn: { name: "Alex Rivera", handle: "@alexrivera", photo: null } };

const v = <T>(x: T) => Promise.resolve(x);

// ------------------------------------------------------------------ commands

export const commands = {
  settingsGet: () => ok(settings),
  settingsSet: (s: Settings) => {
    settings = s;
    events.settingsChanged.emit(s);
    return ok(s);
  },
  playerGetState: () => v(state),
  playerPlay: () => {
    state = { ...state, status: "Playing" };
    emitState();
    return v(undefined);
  },
  playerPause: () => {
    state = { ...state, status: "Paused" };
    emitState();
    return v(undefined);
  },
  playerToggle: () => (state.status === "Playing" ? commands.playerPause() : commands.playerPlay()),
  playerStop: () => {
    state = { ...state, status: "Idle", positionMs: 0 };
    emitState();
    return v(undefined);
  },
  playerNext: () => {
    const n = (queue.currentIndex ?? -1) + 1;
    if (n < queue.tracks.length) playAt(n);
    return v(undefined);
  },
  playerPrevious: () => {
    if (state.positionMs > 3000) state = { ...state, positionMs: 0 };
    else playAt(Math.max(0, (queue.currentIndex ?? 0) - 1));
    emitState();
    return v(undefined);
  },
  playerSeek: (ms: number) => {
    state = { ...state, positionMs: ms };
    emitState();
    return v(undefined);
  },
  playerSetVolume: (vol: number | null) => {
    state = { ...state, volume: vol ?? 0.8 };
    emitState();
    return v(undefined);
  },
  playerSetMuted: (m: boolean) => {
    state = { ...state, muted: m };
    emitState();
    return v(undefined);
  },
  playerSetShuffle: (e: boolean) => {
    queue = { ...queue, shuffle: e };
    emitQueue();
    emitState();
    return v(undefined);
  },
  playerSetRepeat: (m: RepeatMode) => {
    queue = { ...queue, repeat: m };
    emitQueue();
    emitState();
    return v(undefined);
  },
  playerListDevices: () => v<AudioDevice[]>([{ id: "dev0", name: "Altavoces (Realtek)", isDefault: true }]),
  playerSetDevice: (_id: string | null) => ok(null),
  queueGet: () => v(queue),
  queueSet: (tracks: Track[], start: number, source: QueueSource) => {
    queue = { ...queue, tracks, source };
    playAt(start);
    return ok(null);
  },
  queuePlayIndex: (i: number) => {
    playAt(i);
    return ok(null);
  },
  queueAdd: (tracks: Track[], position: InsertPosition) => {
    const t = [...queue.tracks];
    const at = position === "Next" ? (queue.currentIndex ?? -1) + 1 : t.length;
    t.splice(at, 0, ...tracks);
    queue = { ...queue, tracks: t };
    emitQueue();
    return v(undefined);
  },
  queueRemove: (i: number) => {
    const t = queue.tracks.filter((_, k) => k !== i);
    const cur = queue.currentIndex ?? 0;
    queue = { ...queue, tracks: t, currentIndex: i < cur ? cur - 1 : cur };
    emitQueue();
    return v(undefined);
  },
  queueMove: (from: number, to: number) => {
    const t = [...queue.tracks];
    const [x] = t.splice(from, 1);
    t.splice(to, 0, x!);
    const cur = queue.currentIndex ?? 0;
    const nc = cur === from ? to : from < cur && to >= cur ? cur - 1 : from > cur && to <= cur ? cur + 1 : cur;
    queue = { ...queue, tracks: t, currentIndex: nc };
    emitQueue();
    return v(undefined);
  },
  queueClear: () => {
    queue = { ...queue, tracks: [], currentIndex: null };
    emitQueue();
    return commands.playerStop();
  },
  queueSetAutoplay: (e: boolean) => {
    queue = { ...queue, autoplay: e };
    emitQueue();
    return v(undefined);
  },
  authStatus: () => v(auth),
  authSignIn: () => ok([{ source: "App", index: 0, name: "Usuario de prueba", handle: "@prueba", photo: null }]),
  authDetectBrowsers: () => v<BrowserProfile[]>([{ browser: "Zen", name: "Default (release)", path: "C:\\zen" }]),
  authScanBrowser: (_p: string) =>
    ok<SessionAccount[]>([
      { source: { Browser: { profile: "C:\\zen", container: "", containerName: "Sin contenedor" } }, index: 0, name: "Cuenta principal", handle: "@principal", photo: null },
      { source: { Browser: { profile: "C:\\zen", container: "^userContextId=1", containerName: "Personal" } }, index: 1, name: "Alex Rivera", handle: "@alexrivera", photo: null },
    ]),
  authScanCookie: (_c: string) => ok<SessionAccount[]>([]),
  authSelectAccount: (a: SessionAccount) => {
    auth = { SignedIn: { name: a.name, handle: a.handle, photo: a.photo } };
    events.authChanged.emit(auth);
    return ok(auth);
  },
  authLogout: () => {
    auth = "SignedOut";
    events.authChanged.emit(auth);
    return ok(null);
  },
  ytHome: (_c: string | null) =>
    ok<Page<Section>>({
      items: [
        { title: "Selección rápida", items: allTracks.slice(10, 22).map((t) => ({ Track: t }) as MediaItem) },
        { title: "Mezclas para ti", items: playlists.slice(1).map((p) => ({ Playlist: p }) as MediaItem) },
        { title: "Álbumes recomendados", items: albumSummaries.map((a) => ({ Album: a }) as MediaItem) },
        { title: "Artistas parecidos", items: artistSummaries.map((a) => ({ Artist: a }) as MediaItem) },
      ],
      continuation: null,
    }),
  ytLibraryPlaylists: () => ok(playlists),
  ytLibraryAlbums: () => ok(albumSummaries),
  ytLibraryArtists: () => ok(artistSummaries),
  ytLibrarySongs: () => ok<Page<Track>>({ items: allTracks.slice(0, 15), continuation: null }),
  ytLikedSongs: () => ok<Page<Track>>({ items: allTracks.filter((t) => t.likeStatus === "Like"), continuation: null }),
  ytHistory: () =>
    ok<Section[]>([
      { title: "Hoy", items: allTracks.slice(3, 9).map((t) => ({ Track: t }) as MediaItem) },
      { title: "Ayer", items: allTracks.slice(20, 30).map((t) => ({ Track: t }) as MediaItem) },
    ]),
  ytMoreTracks: (_c: string) => ok<Page<Track>>({ items: [], continuation: null }),
  ytPlaylist: (id: string) => {
    const p = playlists.find((x) => x.id === id) ?? playlists[1]!;
    const n = p.trackCount ?? 20;
    return ok<PlaylistDetail>({
      summary: p,
      description: "Playlist de prueba con datos inventados.",
      stats: `${n} canciones • 1 hora`,
      owned: true,
      tracks: { items: Array.from({ length: n }, (_, i) => allTracks[(i * 3 + id.length) % allTracks.length]!), continuation: null },
    });
  },
  ytAlbum: (id: string) => {
    const a = albumSummaries.find((x) => x.id === id) ?? albumSummaries[0]!;
    return ok<AlbumDetail>({ summary: a, playlistId: null, stats: "10 canciones • 41 minutos", description: null, tracks: allTracks.filter((t) => t.album?.id === id).slice(0, 10) });
  },
  ytArtist: (id: string) => {
    const a = artistSummaries.find((x) => x.id === id) ?? artistSummaries[0]!;
    return ok<ArtistDetail>({
      summary: a,
      description: "Artista inventado para la vista previa.",
      topSongs: allTracks.filter((t) => t.artists[0]?.id === id).slice(0, 5),
      sections: [{ title: "Álbumes", items: albumSummaries.slice(0, 4).map((x) => ({ Album: x }) as MediaItem) }],
      radioSeed: allTracks[0]!.id,
    });
  },
  ytSearch: (q: string, _f: SearchFilter) =>
    ok<Page<MediaItem>>({
      items: [
        ...artistSummaries.slice(0, 1).map((a) => ({ Artist: a }) as MediaItem),
        ...allTracks.filter((t) => (t.title + t.artists[0]!.name).toLowerCase().includes(q.toLowerCase().slice(0, 3))).slice(0, 12).map((t) => ({ Track: t }) as MediaItem),
        ...albumSummaries.slice(0, 3).map((a) => ({ Album: a }) as MediaItem),
      ],
      continuation: null,
    }),
  ytSearchMore: (_c: string, _f: SearchFilter) => ok<Page<MediaItem>>({ items: [], continuation: null }),
  ytSearchSuggestions: (q: string) => ok([q, `${q} en directo`, `${q} remix`, `${q} acústico`]),
  ytRadio: (seed: string) => ok([allTracks.find((t) => t.id === seed) ?? allTracks[0]!, ...allTracks.slice(30, 50)]),
  ytRate: (_id: string, _s: LikeStatus) => ok(null),
  ytAddToPlaylist: (_p: string, _t: string[]) => ok(null),
  ytCreatePlaylist: (_t: string, _d: string | null, _p: string) => ok("VLPLnueva"),
  uiPlaySound: (_s: string) => v(undefined),
  uiGetLastRoute: () => v<string | null>(null),
  uiSetLastRoute: (_r: string) => v(undefined),
  playerHint: (_ids: string[]) => v(undefined),
  windowIsCompact: () => v(false),
  windowSetCompact: (_e: boolean) => ok(null),
  windowSetShape: (_p: [number, number][][] | null) => ok(null),
  windowSetSkin: (_s: [number | null, number | null] | null) => ok(null),
  // With XP_SKINS_DIR, Vite serves the skins folder (vite.config.ts).
  skinsList: async () => ({ status: "ok" as const, data: (await (await fetch("/skin/list")).json()) as { id: string; name: string; bytes: number }[] }),
  skinsOpenDir: () => ok(null),
  vizSetActive: (on: boolean) => {
    clearInterval(vizTimer);
    if (on) vizTimer = setInterval(fakeViz, 33);
    return v(undefined);
  },
  toolsVersions: () => v({ app: "0.1.0", ytDlp: "2026.08.19", deno: "2.9.7", ytDlpUpdated: false }),
  toolsUpdateNow: () => ok<string | null>(null),
  toolsOpenLogsDir: () => ok(null),
  updateStatus: () => v({ enabled: true, pending: null }),
  updateCheck: () => ok(null),
  updateInstall: () => ok(null),
  uiLog: (level: string, message: string) => {
    console[level === "error" ? "error" : "log"](message);
    return v(undefined);
  },
};

void seq;

// Only in the preview: console access for measuring and debugging, and a log of
// the calls to the "backend" (to count repeated requests).
const calls: string[] = [];
for (const [name, fn] of Object.entries(commands)) {
  (commands as Record<string, unknown>)[name] = (...args: unknown[]) => {
    calls.push(name);
    return (fn as (...a: unknown[]) => unknown)(...args);
  };
}
(globalThis as { __mock?: unknown }).__mock = { commands, events, calls };
