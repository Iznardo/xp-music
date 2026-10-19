// Detección del entorno: dentro de Tauri o en un navegador (vista previa `dev:mock`).

import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/// Ventana actual de Tauri, o `null` en la vista previa del navegador.
export function appWindow(): Window | null {
  return isTauri ? getCurrentWindow() : null;
}
