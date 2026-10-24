// Máscaras de WMP (skins .wmz): la lista de la carpeta, la elegida y si la app
// está en modo máscara. Lo elegido se recuerda en este equipo.

import { convertFileSrc } from "@tauri-apps/api/core";
import { commands, type SkinInfo } from "../bindings";
import { isTauri } from "../platform";
import { ui } from "./ui.svelte";

/// Dónde sirve el backend el motor y los .wmz (en la vista previa, Vite), sin `/` final.
export const SKIN_BASE = isTauri ? convertFileSrc("", "skin").replace(/\/$/, "") : "/skin";

const KEY = "xpm.skin";
const MODE = "xpm.skinMode.v2";

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function write(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    /* sin almacenamiento: no se recuerda */
  }
}

/// Preferencias que guarda cada máscara (theme.savePreference).
export function skinPrefs(id: string): Record<string, string> {
  try {
    return JSON.parse(read(`xpm.skinPrefs.${id}`) ?? "{}") as Record<string, string>;
  } catch {
    return {};
  }
}
export function saveSkinPref(id: string, key: string, value: string) {
  const prefs = skinPrefs(id);
  prefs[key.slice(0, 100)] = value.slice(0, 2000);
  // Un tope por si una máscara guarda sin parar.
  if (Object.keys(prefs).length <= 200) write(`xpm.skinPrefs.${id}`, JSON.stringify(prefs));
}

class SkinStore {
  list = $state<SkinInfo[]>([]);
  current = $state<string | null>(read(KEY));
  active = $state(read(MODE) === "1");
  pickerOpen = $state(false);

  async refresh() {
    try {
      const r = await commands.skinsList();
      if (r.status === "ok") this.list = r.data;
      else ui.error(r.error);
    } catch (e) {
      void commands.uiLog("error", `skins_list: ${String(e)}`);
      ui.error(String(e));
    }
  }

  /// Pasa a modo máscara (con `id`, cambia antes de máscara).
  async enter(id?: string) {
    if (id) {
      this.current = id;
      write(KEY, id);
    }
    if (!this.current) {
      this.pickerOpen = true;
      return;
    }
    if (ui.compact) await ui.setCompact(false);
    this.active = true;
    write(MODE, "1");
  }

  async exit() {
    this.active = false;
    write(MODE, "0");
    await commands.windowSetSkin(null);
  }
}

export const skins = new SkinStore();
