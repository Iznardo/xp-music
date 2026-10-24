// Async loading with state (loading / error / data) for the views.
//
// With `key`, data is kept in memory: returning to a view shows it instantly; if it
// is younger than `FRESH_MS` it is not requested again and, if older, it is still
// shown and refreshed in the background. Identical requests that overlap in time
// are merged into one.

import type { AppError } from "./bindings";

type Res<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

const FRESH_MS = 60_000;
const cache = new Map<string, { data: unknown; at: number }>();
const inflight = new Map<string, Promise<Res<unknown>>>();

/// When switching account or signing out, whatever is stored is no longer valid.
export function clearResourceCache() {
  cache.clear();
}

/// Runs `fetcher` only once for all simultaneous requests for `key`.
export function shared<T>(key: string, fetcher: () => Promise<Res<T>>): Promise<Res<T>> {
  const running = inflight.get(key) as Promise<Res<T>> | undefined;
  if (running) return running;
  const p = fetcher().finally(() => inflight.delete(key));
  inflight.set(key, p);
  return p;
}

export class Resource<T> {
  data = $state<T | null>(null);
  error = $state<AppError | null>(null);
  loading = $state(false);
  #seq = 0;

  constructor(
    private fetcher: () => Promise<Res<T>>,
    /// A function (not a value) so it follows the view's props if they change.
    private keyOf?: () => string,
  ) {
    const hit = keyOf ? cache.get(keyOf()) : undefined;
    if (hit) this.data = hit.data as T;
  }

  async load(): Promise<void> {
    const key = this.keyOf?.();
    const hit = key ? cache.get(key) : undefined;
    if (hit) {
      this.data = hit.data as T;
      if (Date.now() - hit.at < FRESH_MS) return;
    }
    const seq = ++this.#seq;
    // With data already on screen, "Loading…" is not shown: it refreshes behind the scenes.
    this.loading = !hit;
    this.error = null;
    try {
      const r = key ? await shared(key, this.fetcher) : await this.fetcher();
      if (seq !== this.#seq) return; // Arrived late: something else was already requested.
      if (r.status === "ok") {
        this.data = r.data;
        if (key) cache.set(key, { data: r.data, at: Date.now() });
      } else if (!hit) this.error = r.error;
    } catch (e) {
      if (seq === this.#seq && !hit) this.error = { code: "Internal", message: String(e), recoverable: true };
    } finally {
      if (seq === this.#seq) this.loading = false;
    }
  }
}
