// Reflejo del reproductor del backend (principio II). Se hidrata al montar
// (la WebView puede recrearse) y se mantiene al día con los eventos.
//
// Los eventos traen el estado completo; para no recalcular (ni repintar) lo que
// no ha cambiado, se reconcilian con lo que ya hay en vez de sustituirlo: la cola,
// si tiene las mismas canciones, conserva su lista (y con ella la selección y el
// orden de la tabla), y del estado solo se asignan los campos distintos.

import { commands, events, type AppError, type PlayerState, type QueueSnapshot, type Track } from "../bindings";

/// Mismas canciones en el mismo orden (comparando ids: con 3000 filas es rápido).
function sameTracks(a: Track[], b: Track[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i]!.id !== b[i]!.id) return false;
  return true;
}

/// Asigna en `target` solo lo que difiere de `source` (claves de primer nivel;
/// los objetos anidados se comparan por contenido).
function patch<T extends object>(target: T, source: T) {
  for (const key of Object.keys(source) as (keyof T)[]) {
    const next = source[key];
    const prev = target[key];
    if (prev === next) continue;
    if (prev && next && typeof prev === "object" && JSON.stringify(prev) === JSON.stringify(next)) continue;
    target[key] = next;
  }
}

class PlayerStore {
  state = $state<PlayerState | null>(null);
  queue = $state<QueueSnapshot | null>(null);
  bufferedMs = $state(0);
  lastError = $state<AppError | null>(null);

  setState(next: PlayerState) {
    if (this.state && this.state.track?.id === next.track?.id) patch(this.state, next);
    else this.state = next;
  }

  setQueue(next: QueueSnapshot) {
    const cur = this.queue;
    if (!cur || !sameTracks(cur.tracks, next.tracks)) {
      this.queue = next;
      return;
    }
    // Mismas canciones: solo cambian el índice actual y los modos.
    const { tracks: _same, ...rest } = next;
    patch(cur, { ...cur, ...rest });
  }

  async init(): Promise<void> {
    await Promise.all([
      events.playerState.listen((e) => this.setState(e.payload)),
      events.playerPosition.listen((e) => {
        if (this.state) this.state.positionMs = e.payload.positionMs;
        this.bufferedMs = e.payload.bufferedMs;
      }),
      events.queueChanged.listen((e) => this.setQueue(e.payload)),
      events.playerError.listen((e) => {
        this.lastError = e.payload;
      }),
    ]);
    const [state, queue] = await Promise.all([commands.playerGetState(), commands.queueGet()]);
    this.setState(state);
    this.setQueue(queue);
  }
}

export const player = new PlayerStore();
