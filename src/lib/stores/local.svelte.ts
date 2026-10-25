// "My music": the folders on disk and the state of their scan (kept up to date by
// `local://scan`). `version` goes up each time a scan finishes, so the views reload.

import { commands, events, type LocalScanStatus } from "../bindings";
import { settings } from "./settings.svelte";
import { ui } from "./ui.svelte";

class LocalStore {
  scan = $state<LocalScanStatus>({ scanning: false, done: 0, total: 0 });
  version = $state(0);

  /// The folders come with the settings (`settings://changed` keeps them current).
  get folders(): string[] {
    return settings.current?.localFolders ?? [];
  }

  async init(): Promise<void> {
    await events.localScan.listen((e) => {
      const finished = this.scan.scanning && !e.payload.scanning;
      this.scan = e.payload;
      if (finished) this.version++;
    });
    this.scan = await commands.localScanStatus();
  }

  async addFolder(): Promise<void> {
    const r = await commands.localAddFolder();
    if (r.status === "error") ui.error(r.error.message);
  }

  async removeFolder(path: string): Promise<void> {
    const r = await commands.localRemoveFolder(path);
    if (r.status === "error") ui.error(r.error.message);
  }

  rescan(): void {
    void commands.localRescan();
  }
}

export const local = new LocalStore();
