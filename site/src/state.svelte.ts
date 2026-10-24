// Estado de la web: ruta (hash), tema y última versión. El reproductor está en player.svelte.ts.

import { REPO } from "./content";
import { classify, type Build } from "./builds";

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

/** Un instalador de la release. `sha256`: el que calcula GitHub al subirlo. */
export type Asset = Build & { name: string; size: number; url: string; sha256: string | null };
export type Release = { tag: string; date: string; url: string; assets: Asset[] };

/** Última versión publicada (API de GitHub, sin autenticar: 60 peticiones/hora por IP). */
export const release = $state<{ data: Release | null; failed: boolean }>({ data: null, failed: false });

// VITE_RELEASE_API: otra URL con el mismo formato, para probar la página con una
// release de ejemplo (`VITE_RELEASE_API=/ejemplo.json npm run site:dev`).
fetch(import.meta.env.VITE_RELEASE_API || `${REPO.replace("github.com", "api.github.com/repos")}/releases/latest`)
  .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
  .then((j) => {
    release.data = {
      tag: j.tag_name,
      date: new Date(j.published_at).toLocaleDateString("es-ES", { day: "numeric", month: "long", year: "numeric" }),
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
