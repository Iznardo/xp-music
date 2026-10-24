// Estado de la web: ruta (hash), tema y última versión. El reproductor está en player.svelte.ts.

import { REPO } from "./content";

export type Theme = "blue" | "olive" | "silver";

export const site = $state({
  hash: location.hash,
  theme: (document.documentElement.dataset.theme ?? "blue") as Theme,
  preset: 0,
  sideCollapsed: false,
  optionsOpen: false,
});

addEventListener("hashchange", () => (site.hash = location.hash));

export function route(hash: string) {
  const [section = "", doc = ""] = hash.replace(/^#\/?/, "").split("/");
  return { section, doc: decodeURIComponent(doc) };
}

export function setTheme(t: Theme) {
  site.theme = t;
  document.documentElement.dataset.theme = t;
  localStorage.setItem("xp-theme", t);
}

export type Asset = { name: string; size: number; url: string; downloads: number };
export type Release = { tag: string; date: string; url: string; assets: Asset[] };

/** Última versión publicada (API de GitHub, sin autenticar: 60 peticiones/hora por IP). */
export const release = $state<{ data: Release | null; failed: boolean }>({ data: null, failed: false });

fetch(`${REPO.replace("github.com", "api.github.com/repos")}/releases/latest`)
  .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
  .then((j) => {
    release.data = {
      tag: j.tag_name,
      date: new Date(j.published_at).toLocaleDateString("es-ES", { day: "numeric", month: "long", year: "numeric" }),
      url: j.html_url,
      assets: j.assets.map((a: { name: string; size: number; browser_download_url: string; download_count: number }) => ({
        name: a.name,
        size: a.size,
        url: a.browser_download_url,
        downloads: a.download_count,
      })),
    };
  })
  .catch(() => (release.failed = true));
