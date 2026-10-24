// App self-updates (007): notice when a new version is detected and
// state for Options ▸ About.

import { commands, events, type UpdateInfo, type UpdaterStatus } from "../bindings";
import { ui } from "./ui.svelte";
import { t } from "../i18n.svelte";

class UpdatesStore {
  status = $state<UpdaterStatus | null>(null);
  busy = $state(false);
  message = $state("");

  async init(): Promise<void> {
    await events.updateAvailable.listen((e) => void this.offer(e.payload));
    await this.refresh();
    // Found while the window was closed (in the tray).
    if (this.status?.pending) void this.offer(this.status.pending);
  }

  async refresh(): Promise<void> {
    this.status = await commands.updateStatus();
  }

  /// Check requested by the user.
  async check(): Promise<void> {
    this.busy = true;
    this.message = t.updates.checking;
    const r = await commands.updateCheck();
    this.busy = false;
    if (r.status === "error") this.message = r.error.message;
    else if (r.data) {
      this.message = t.updates.available(r.data.version);
      void this.offer(r.data);
    } else this.message = t.updates.upToDate;
    await this.refresh();
  }

  async offer(info: UpdateInfo): Promise<void> {
    if (ui.message) return;
    const b = await ui.ask(t.updates.title, t.updates.question(info.version, info.notes), "question", [t.updates.install, t.updates.later]);
    if (b === t.updates.install) await this.install();
  }

  async install(): Promise<void> {
    this.busy = true;
    ui.status = t.updates.downloading;
    const r = await commands.updateInstall();
    // If it goes well the app restarts and we never get here.
    this.busy = false;
    ui.status = "";
    if (r.status === "error") ui.error(r.error);
  }
}

export const updates = new UpdatesStore();
