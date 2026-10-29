// Website state: route (hash), theme and latest release. The player is in player.svelte.ts;
// the language is in i18n.svelte.ts.

import { REPO } from "./content";
import { classify, type Build } from "./builds";

export type Theme = "blue" | "olive" | "silver";
/** Same values as the app's setting (and its texts). */
export type Appearance = "System" | "Light" | "Dark";
const APPEARANCES: Appearance[] = ["System", "Light", "Dark"];
const saved = localStorage.getItem("xp-appearance") as Appearance;

export const site = $state({
  hash: location.hash,
  theme: (document.documentElement.dataset.theme ?? "blue") as Theme,
  appearance: APPEARANCES.includes(saved) ? saved : "System",
  preset: 0,
  sideCollapsed: false,
  optionsOpen: false,
});

addEventListener("hashchange", () => (site.hash = location.hash));

/** Routes used before the site was translated, so published links keep working. */
const LEGACY: Record<string, string> = {
  descargar: "download",
  capturas: "screenshots",
  instalar: "install",
  uso: "usage",
  mascaras: "skins",
  "preguntas-frecuentes": "faq",
  compilar: "building",
};

export function route(hash: string) {
  const [section = "", doc = ""] = hash.replace(/^#\/?/, "").split("/");
  const slug = decodeURIComponent(doc);
  return { section: LEGACY[section] ?? section, doc: LEGACY[slug] ?? slug };
}

// Light or dark: the chosen one, or the system's (followed live).
const systemDark = matchMedia("(prefers-color-scheme: dark)");
function applyMode() {
  const dark = site.appearance === "Dark" || (site.appearance === "System" && systemDark.matches);
  document.documentElement.dataset.mode = dark ? "dark" : "light";
}
systemDark.addEventListener("change", applyMode);

export function setAppearance(a: Appearance) {
  site.appearance = a;
  localStorage.setItem("xp-appearance", a);
  applyMode();
}

export function setTheme(t: Theme) {
  site.theme = t;
  document.documentElement.dataset.theme = t;
  localStorage.setItem("xp-theme", t);
}

/** A release installer. `sha256`: the one GitHub computes on upload. */
export type Asset = Build & { name: string; size: number; url: string; sha256: string | null };
/** `published` is an ISO date; the view formats it in the active language. */
export type Release = { tag: string; published: string; url: string; assets: Asset[] };

/** Latest published release (GitHub API, unauthenticated: 60 requests/hour per IP). */
export const release = $state<{ data: Release | null; failed: boolean }>({ data: null, failed: false });

// VITE_RELEASE_API: another URL with the same format, to try the page with a sample
// release (`VITE_RELEASE_API=/sample.json npm run site:dev`).
fetch(import.meta.env.VITE_RELEASE_API || `${REPO.replace("github.com", "api.github.com/repos")}/releases/latest`)
  .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
  .then((j) => {
    release.data = {
      tag: j.tag_name,
      published: j.published_at,
      url: j.html_url,
      assets: j.assets.flatMap((a: { name: string; size: number; browser_download_url: string; digest?: string | null }) => {
        const build = classify(a.name);
        if (!build) return [];
        const sha256 = a.digest?.startsWith("sha256:") ? a.digest.slice(7) : null;
        return [{ ...build, name: a.name, size: a.size, url: a.browser_download_url, sha256 }];
      }),
    };
  })
  .catch(() => (release.failed = true));
