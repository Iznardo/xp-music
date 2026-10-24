// Mirror of the backend player (principle II). Hydrated on mount (the WebView
// can be recreated) and kept up to date with events.
//
// Events carry the full state; to avoid recomputing (or repainting) what hasn't
// changed, they are reconciled with what is already there instead of replacing it:
// the queue, if it has the same songs, keeps its list (and with it the selection and
// the table order), and only the fields that differ are assigned to the state.

import { commands, events, type AppError, type PlayerState, type QueueSnapshot, type Track } from "../bindings";

/// Same songs in the same order (comparing ids: fast even with 3000 rows).
function sameTracks(a: Track[], b: Track[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i]!.id !== b[i]!.id) return false;
  return true;
}

/// Assigns to `target` only what differs from `source` (top-level keys;
/// nested objects are compared by content).
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
    // Same songs: only the current index and the modes change.
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
