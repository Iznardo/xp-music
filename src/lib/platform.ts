// Environment detection: inside Tauri or in a browser (`dev:mock` preview).

import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/// Current Tauri window, or `null` in the browser preview.
export function appWindow(): Window | null {
  return isTauri ? getCurrentWindow() : null;
}
