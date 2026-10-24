// Website language: the visitor's choice (localStorage) or the browser's. `t` reads the
// active dictionary on every access, so templates update live when the language changes.

import { languageFromTag, type Language } from "../../src/lib/locale";
import { setLanguage as setAppLanguage } from "../../src/lib/i18n.svelte";
import { en, type Dict } from "./locales/en";
import { es } from "./locales/es";

const dicts: Record<Language, Dict> = { en, es };
const KEY = "xp-lang";
/** BCP 47 tags for dates and times. */
const LOCALE: Record<Language, string> = { en: "en-US", es: "es-ES" };

const system = languageFromTag(navigator.languages?.[0] ?? navigator.language);
const saved = ((): Language | null => {
  const v = localStorage.getItem(KEY);
  return v === "en" || v === "es" ? v : null;
})();

/** `choice`: null = follow the browser. */
export const i18n = $state<{ choice: Language | null }>({ choice: saved });

export const language = (): Language => i18n.choice ?? system;
export const locale = () => LOCALE[language()];

export function setLanguage(choice: Language | null) {
  i18n.choice = choice;
  if (choice) localStorage.setItem(KEY, choice);
  else localStorage.removeItem(KEY);
}

export const t: Dict = new Proxy({} as Dict, {
  get: (_, key: string) => dicts[language()][key as keyof Dict],
});

// <html lang> (set by setAppLanguage) and the description meta follow the language;
// App.svelte sets the title.
$effect.root(() => {
  $effect(() => {
    // The app components the site reuses (tree, dialogs) read their texts from the app's `t`.
    setAppLanguage(language());
    document.querySelector('meta[name="description"]')?.setAttribute("content", t.description);
  });
});
