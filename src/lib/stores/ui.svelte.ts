// Estado puramente visual: cuadros de mensaje, menú contextual y diálogos.

import { commands, type AppError, type UiSound } from "../bindings";

export type MenuItem =
  | { kind: "item"; label: string; action: () => void; disabled?: boolean; bold?: boolean; shortcut?: string }
  | { kind: "check"; label: string; checked: boolean; action: () => void }
  | { kind: "sub"; label: string; items: MenuItem[]; disabled?: boolean }
  | { kind: "sep" };

export type MessageKind = "info" | "error" | "warning" | "question";

class UiStore {
  message = $state<{ title: string; text: string; kind: MessageKind; buttons: string[]; resolve: (b: string) => void } | null>(null);
  menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  optionsOpen = $state(false);
  aboutOpen = $state(false);
  loginOpen = $state(false);
  prompt = $state<{ title: string; label: string; value: string; resolve: (v: string | null) => void } | null>(null);
  /// Texto de la barra de estado (a la izquierda del transporte).
  status = $state("");
  /// Modo compacto (skin pequeño, siempre encima).
  compact = $state(false);

  sound(s: UiSound) {
    void commands.uiPlaySound(s);
  }

  async setCompact(enabled: boolean) {
    const r = await commands.windowSetCompact(enabled);
    if (r.status === "ok") this.compact = enabled;
  }

  /// Cuadro de mensaje XP. Devuelve el botón pulsado.
  ask(title: string, text: string, kind: MessageKind = "info", buttons = ["Aceptar"]): Promise<string> {
    this.sound(kind === "error" ? "Error" : kind === "info" ? "Notify" : "Exclamation");
    return new Promise((resolve) => {
      this.message = { title, text, kind, buttons, resolve };
    });
  }

  error(e: AppError | string) {
    void this.ask("XP Music", typeof e === "string" ? e : e.message, "error");
  }

  input(title: string, label: string, value = ""): Promise<string | null> {
    return new Promise((resolve) => {
      this.prompt = { title, label, value, resolve };
    });
  }

  openMenu(e: MouseEvent | { x: number; y: number }, items: MenuItem[]) {
    if ("preventDefault" in e) e.preventDefault();
    const x = "clientX" in e ? e.clientX : e.x;
    const y = "clientY" in e ? e.clientY : e.y;
    this.menu = { x, y, items };
  }
}

export const ui = new UiStore();
