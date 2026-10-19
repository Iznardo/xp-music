// Carga asíncrona con estado (cargando / error / datos) para las vistas.

import type { AppError } from "./bindings";

type Res<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

export class Resource<T> {
  data = $state<T | null>(null);
  error = $state<AppError | null>(null);
  loading = $state(false);
  #seq = 0;

  constructor(private fetcher: () => Promise<Res<T>>) {}

  async load(): Promise<void> {
    const seq = ++this.#seq;
    this.loading = true;
    this.error = null;
    try {
      const r = await this.fetcher();
      if (seq !== this.#seq) return; // Llegó tarde: ya se pidió otra cosa.
      if (r.status === "ok") this.data = r.data;
      else this.error = r.error;
    } catch (e) {
      if (seq === this.#seq) this.error = { code: "Internal", message: String(e), recoverable: true };
    } finally {
      if (seq === this.#seq) this.loading = false;
    }
  }
}
