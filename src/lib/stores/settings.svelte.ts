// Frontend mirror of the backend settings (principle II: Rust is the source of truth).
// Hydrated on mount and kept up to date with `settings://changed`.

import { commands, events, type AppError, type Settings } from "../bindings";
import { setLanguage } from "../i18n.svelte";

class SettingsStore {
  current = $state<Settings | null>(null);
  error = $state<AppError | null>(null);

  /** Single place that adopts new settings, so the UI language follows them. */
  private adopt(s: Settings): void {
    this.current = s;
    setLanguage(s.language);
  }

  async init(): Promise<void> {
    await events.settingsChanged.listen((e) => this.adopt(e.payload));
    const res = await commands.settingsGet();
    if (res.status === "ok") this.adopt(res.data);
    else this.error = res.error;
  }

  async update(patch: Partial<Settings>): Promise<void> {
    if (!this.current) return;
    const res = await commands.settingsSet({ ...this.current, ...patch });
    if (res.status === "ok") this.adopt(res.data);
    else this.error = res.error;
  }
}

export const settings = new SettingsStore();
