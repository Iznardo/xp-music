// Sustituto de `bindings.ts` para previsualizar la UI en un navegador normal
// (`npm run dev:mock`). Datos inventados y un reproductor simulado; no habla con Rust.

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

// ------------------------------------------------------------------ eventos

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
};

// ------------------------------------------------------------------ datos

const palette = ["#3a6ea5", "#a54a3a", "#3aa56b", "#8a3aa5", "#a5893a", "#3a9aa5", "#a53a7c", "#5b5b5b"];
function cover(seed: number, label: string): Thumbnail[] {
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
    author: "chema iznardo",
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

// ------------------------------------------------------------------ reproductor simulado

let settings: Settings = {
  theme: "Blue",
  volume: 0.8,
  normalizeVolume: true,
  audioQuality: "Aac128",
  outputDevice: null,
  closeToTray: true,
  startMinimized: false,
  launchAtLogin: false,
  reportHistory: true,
  visualization: "Bars",
  xpSounds: true,
  notifications: true,
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

let auth: AuthStatus = { SignedIn: { name: "chema iznardo", handle: "@chemaiz4", photo: null } };

const v = <T>(x: T) => Promise.resolve(x);

// ------------------------------------------------------------------ comandos

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
  authDetectBrowsers: () => v<BrowserProfile[]>([{ browser: "Zen", name: "Default (release)", path: "C:\\zen" }]),
  authScanBrowser: (_p: string) =>
    ok<SessionAccount[]>([
      { source: { Browser: { profile: "C:\\zen", container: "", containerName: "Sin contenedor" } }, index: 0, name: "Cuenta principal", handle: "@principal", photo: null },
      { source: { Browser: { profile: "C:\\zen", container: "^userContextId=1", containerName: "Personal" } }, index: 1, name: "chema iznardo", handle: "@chemaiz4", photo: null },
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
  windowIsCompact: () => v(false),
  windowSetCompact: (_e: boolean) => ok(null),
};

void seq;
