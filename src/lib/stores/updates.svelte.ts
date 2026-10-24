// Actualizaciones de la propia app (007): aviso al detectar versión nueva y
// estado para Opciones ▸ Acerca de.

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
    // Encontrada mientras la ventana estaba cerrada (en la bandeja).
    if (this.status?.pending) void this.offer(this.status.pending);
  }

  async refresh(): Promise<void> {
    this.status = await commands.updateStatus();
  }

  /// Comprobación pedida por el usuario.
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
    // Si va bien la app se reinicia y no se llega aquí.
    this.busy = false;
    ui.status = "";
    if (r.status === "error") ui.error(r.error);
  }
}

export const updates = new UpdatesStore();
