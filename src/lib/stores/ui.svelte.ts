// Purely visual state: message boxes, context menu and dialogs.

import { commands, type AppError, type UiSound } from "../bindings";

export type MenuItem =
  | { kind: "item"; label: string; action: () => void; disabled?: boolean; bold?: boolean; shortcut?: string }
  | { kind: "check"; label: string; checked: boolean; action: () => void }
  | { kind: "sub"; label: string; items: MenuItem[]; disabled?: boolean }
  | { kind: "sep" };

export type MessageKind = "info" | "error" | "warning" | "question";

function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}
function writeFlag(key: string, on: boolean) {
  try {
    localStorage.setItem(key, on ? "1" : "0");
  } catch {
    /* no storage: not remembered */
  }
}

class UiStore {
  message = $state<{ title: string; text: string; kind: MessageKind; buttons: string[]; resolve: (b: string) => void } | null>(null);
  menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  optionsOpen = $state(false);
  /// Open Options directly on the About tab (consumed by the dialog).
  optionsAbout = $state(false);
  loginOpen = $state(false);
  prompt = $state<{ title: string; label: string; value: string; resolve: (v: string | null) => void } | null>(null);
  /// Status bar text (left of the transport).
  status = $state("");
  /// Compact mode (small skin, always on top).
  compact = $state(false);
  /// Side task panel hidden (remembered on this machine).
  sideCollapsed = $state(readFlag("xpm.sideCollapsed"));

  toggleSide() {
    this.sideCollapsed = !this.sideCollapsed;
    writeFlag("xpm.sideCollapsed", this.sideCollapsed);
  }

  sound(s: UiSound) {
    void commands.uiPlaySound(s);
  }

  async setCompact(enabled: boolean) {
    const r = await commands.windowSetCompact(enabled);
    if (r.status === "ok") this.compact = enabled;
  }

  /// XP message box. Returns the button pressed.
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
