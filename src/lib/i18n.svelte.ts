// Interface language. `t` is the single entry point for UI text: it reads the
// active dictionary on every access, so templates and `$derived` that use it
// update live when the language changes.

import type { Language } from "./bindings";
import { en, type Dict } from "./locales/en";
import { es } from "./locales/es";

const dicts: Record<Language, Dict> = { en, es };

/** `es`, `es-ES`, `es_MX.UTF-8`… are Spanish; anything else is English. */
export function languageFromTag(tag: string | undefined | null): Language {
  return /^es([-_.@:]|$)/i.test((tag ?? "").trim()) ? "es" : "en";
}

const system = languageFromTag(typeof navigator === "undefined" ? "" : navigator.language);

/** The user's choice; `null` = follow the system language. */
let choice = $state<Language | null>(null);

export function language(): Language {
  return choice ?? system;
}

export function setLanguage(next: Language | null): void {
  choice = next;
  document.documentElement.lang = language();
}

export const t: Dict = new Proxy({} as Dict, {
  get: (_, key: string) => dicts[language()][key as keyof Dict],
});
