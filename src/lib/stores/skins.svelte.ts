// WMP skins (.wmz): the folder's list, the chosen one and whether the app is in
// skin mode. The choice is remembered on this machine.

import { convertFileSrc } from "@tauri-apps/api/core";
import { commands, type SkinInfo } from "../bindings";
import { isTauri } from "../platform";
import { ui } from "./ui.svelte";

/// Where the backend serves the engine and the .wmz files (in the preview, Vite), without trailing `/`.
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
    /* no storage: not remembered */
  }
}

/// Preferences each skin saves (theme.savePreference).
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
  // A cap in case a skin saves nonstop.
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

  /// Switches to skin mode (with `id`, changes skin first).
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
