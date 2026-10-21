// Carga asíncrona con estado (cargando / error / datos) para las vistas.
//
// Con `key`, los datos se guardan en memoria: al volver a una vista se muestran al
// instante; si tienen menos de `FRESH_MS` no se vuelven a pedir y, si son más
// viejos, se muestran igualmente y se refrescan en segundo plano. Las peticiones
// iguales que coinciden en el tiempo se unifican en una sola.

import type { AppError } from "./bindings";

type Res<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

const FRESH_MS = 60_000;
const cache = new Map<string, { data: unknown; at: number }>();
const inflight = new Map<string, Promise<Res<unknown>>>();

/// Al cambiar de cuenta o cerrar sesión, lo guardado deja de valer.
export function clearResourceCache() {
  cache.clear();
}

/// Ejecuta `fetcher` una sola vez para todas las peticiones simultáneas de `key`.
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
    /// Función (no valor) para que siga a las props de la vista si cambian.
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
    // Con datos ya en pantalla no se muestra "Cargando…": se refresca por detrás.
    this.loading = !hit;
    this.error = null;
    try {
      const r = key ? await shared(key, this.fetcher) : await this.fetcher();
      if (seq !== this.#seq) return; // Llegó tarde: ya se pidió otra cosa.
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
