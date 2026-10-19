// Reflejo en el frontend de los ajustes del backend (principio II: Rust es la fuente de verdad).
// Se hidrata al montar y se mantiene al día con `settings://changed`.

import { commands, events, type AppError, type Settings } from "../bindings";

class SettingsStore {
  current = $state<Settings | null>(null);
  error = $state<AppError | null>(null);

  async init(): Promise<void> {
    await events.settingsChanged.listen((e) => {
      this.current = e.payload;
    });
    const res = await commands.settingsGet();
    if (res.status === "ok") this.current = res.data;
    else this.error = res.error;
  }

  async update(patch: Partial<Settings>): Promise<void> {
    if (!this.current) return;
    const res = await commands.settingsSet({ ...this.current, ...patch });
    if (res.status === "ok") this.current = res.data;
    else this.error = res.error;
  }
}

export const settings = new SettingsStore();
