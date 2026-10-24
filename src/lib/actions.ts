// Acciones sobre canciones y colecciones, compartidas por todas las vistas y menús.

import { commands, type MediaItem, type QueueSource, type Track } from "./bindings";
import { router } from "./router.svelte";
import { library } from "./stores/library.svelte";
import { ui, type MenuItem } from "./stores/ui.svelte";
import { t } from "./i18n.svelte";

async function check<T>(p: Promise<{ status: "ok"; data: T } | { status: "error"; error: { message: string } }>): Promise<T | null> {
  const r = await p;
  if (r.status === "error") {
    ui.error(r.error.message);
    return null;
  }
  return r.data;
}

/// Para órdenes sin resultado: ¿ha ido bien? (Su `data` es `null`, igual que un fallo en `check`.)
async function succeeded(p: Promise<{ status: "ok"; data: unknown } | { status: "error"; error: { message: string } }>): Promise<boolean> {
  const r = await p;
  if (r.status === "error") ui.error(r.error.message);
  return r.status === "ok";
}

export async function playTracks(tracks: Track[], index = 0, source: QueueSource = "Manual") {
  if (!tracks.length) return;
  await check(commands.queueSet(tracks, index, source));
}

/// Una sola canción suelta: suena ella y después su radio (como YouTube Music).
/// Empieza a sonar ya; la radio se pide a la vez y se añade detrás al llegar.
export async function playWithRadio(track: Track) {
  await playTracks([track], 0, { Radio: track.id });
  const radio = (await check(commands.ytRadio(track.id))) ?? [];
  // Si mientras tanto se eligió otra cosa, no se toca la cola nueva.
  const q = await commands.queueGet();
  if (q.tracks.length !== 1 || q.tracks[0]?.id !== track.id) return;
  const rest = radio.filter((r) => r.id !== track.id);
  if (rest.length) await commands.queueAdd(rest, "End");
}

/// Carga una lista de reproducción entera en la cola y empieza a sonar.
export async function playPlaylist(id: string) {
  ui.status = t.views.loading;
  const d = await check(commands.ytPlaylist(id));
  ui.status = "";
  if (d?.tracks.items.length) await playTracks(d.tracks.items, 0, { Playlist: id });
}

let lastHint: string | null = null;
/// Avisa al reproductor de que esta canción probablemente se va a pedir.
export function hintTrack(id: string | undefined) {
  if (!id || id === lastHint) return;
  lastHint = id;
  void commands.playerHint([id]);
}

export async function playNext(tracks: Track[]) {
  await commands.queueAdd(tracks, "Next");
  ui.status = t.status.addedNext(tracks.length);
}

export async function addToQueue(tracks: Track[]) {
  await commands.queueAdd(tracks, "End");
  ui.status = t.status.addedQueue(tracks.length);
}

export async function setLike(track: Track, like: boolean) {
  if (await succeeded(commands.ytRate(track.id, like ? "Like" : "Indifferent"))) {
    track.likeStatus = like ? "Like" : "Indifferent";
    ui.status = like ? t.status.liked : t.status.unliked;
  }
}

export async function addToPlaylist(tracks: Track[], playlistId: string, title: string) {
  if (await succeeded(commands.ytAddToPlaylist(playlistId, tracks.map((x) => x.id)))) {
    ui.status = t.status.addedToPlaylist(tracks.length, title);
  }
}

export async function addToNewPlaylist(tracks: Track[]) {
  const title = await ui.input(t.dialogs.newPlaylistTitle, t.dialogs.newPlaylistLabel);
  if (!title?.trim()) return;
  const id = await check(commands.ytCreatePlaylist(title.trim(), null, "Private"));
  if (!id) return;
  if (tracks.length) await addToPlaylist(tracks, id, title.trim());
  await library.refresh();
}

/// "Nueva lista…" y tus listas, para añadirles canciones.
export function playlistChoices(tracks: Track[]): MenuItem[] {
  return [
    { kind: "item", label: t.menu.newPlaylist, action: () => addToNewPlaylist(tracks) },
    ...(library.editable.length ? [{ kind: "sep" } as MenuItem] : []),
    ...library.editable.map((p) => ({ kind: "item", label: p.title, action: () => addToPlaylist(tracks, p.id, p.title) }) as MenuItem),
  ];
}

/// Menú contextual de una o varias canciones (FR-004-09).
export function trackMenu(tracks: Track[], opts: { onPlay?: () => void; queueIndex?: number } = {}): MenuItem[] {
  const one = tracks.length === 1 ? tracks[0]! : null;
  const items: MenuItem[] = [
    { kind: "item", label: t.menu.play, bold: true, action: () => (opts.onPlay ? opts.onPlay() : playTracks(tracks)) },
    { kind: "item", label: t.menu.playNext, action: () => playNext(tracks) },
    { kind: "item", label: t.menu.addToQueue, action: () => addToQueue(tracks) },
  ];
  if (opts.queueIndex !== undefined) {
    const i = opts.queueIndex;
    items.push({ kind: "item", label: t.menu.removeFromQueue, action: () => void commands.queueRemove(i) });
  }
  items.push({ kind: "sep" });
  if (one) {
    items.push({ kind: "item", label: t.menu.startRadio, action: () => playWithRadio(one) });
    const liked = one.likeStatus === "Like";
    items.push({ kind: "item", label: liked ? t.menu.unlike : t.menu.like, action: () => setLike(one, !liked) });
  }
  items.push({ kind: "sub", label: t.menu.addToPlaylist, items: playlistChoices(tracks) });
  if (one) {
    items.push({ kind: "sep" });
    const album = one.album?.id;
    const artist = one.artists.find((a) => a.id)?.id;
    items.push({ kind: "item", label: t.menu.goAlbum, disabled: !album, action: () => album && router.go({ name: "album", id: album }) });
    items.push({ kind: "item", label: t.menu.goArtist, disabled: !artist, action: () => artist && router.go({ name: "artist", id: artist }) });
  }
  return items;
}

/// Abrir un elemento de Inicio/búsqueda/artista.
export async function openItem(item: MediaItem) {
  if (item.Track) return playWithRadio(item.Track);
  if (item.Playlist) return router.go({ name: "playlist", id: item.Playlist.id });
  if (item.Album) return router.go({ name: "album", id: item.Album.id });
  if (item.Artist) return router.go({ name: "artist", id: item.Artist.id });
}

export function itemTitle(item: MediaItem): string {
  return item.Track?.title ?? item.Playlist?.title ?? item.Album?.title ?? item.Artist?.name ?? "";
}

export function itemSubtitle(item: MediaItem): string {
  if (item.Track) return item.Track.artists.map((a) => a.name).join(", ");
  if (item.Playlist) return item.Playlist.author ?? t.kinds.playlist;
  if (item.Album) return item.Album.subtitle ?? item.Album.artists.map((a) => a.name).join(", ");
  if (item.Artist) return item.Artist.subtitle ?? t.kinds.artist;
  return "";
}

export function itemThumbs(item: MediaItem) {
  return item.Track?.thumbnails ?? item.Playlist?.thumbnails ?? item.Album?.thumbnails ?? item.Artist?.thumbnails ?? [];
}

export function artistNames(track: Track): string {
  return track.artists.map((a) => a.name).join(", ");
}

/// "3:07" a partir de milisegundos.
export function fmtTime(ms: number | null | undefined): string {
  if (ms == null || ms < 0) return "";
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}
