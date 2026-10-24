import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { marked } from "marked";
import { fileURLToPath } from "node:url";
import path from "node:path";

/** Donde viven los .md en GitHub: los enlaces relativos que no son otra página de docs/ van ahí. */
const DOCS_ON_GITHUB = "https://github.com/Iznardo/xp-music/blob/main/docs/";
/** Páginas de docs/ solo para el mantenedor: no salen en la web y sus enlaces van a GitHub.
 *  También hay que excluirlas en el glob de src/docs.ts. */
const MAINTAINER_ONLY = ["publicar"];

/**
 * `docs/*.md` → `{ title, html }`. Los enlaces entre páginas de docs/ pasan a rutas
 * de la web (`#/docs/…`), el resto de enlaces relativos a GitHub, y las imágenes
 * se empaquetan como cualquier otro recurso.
 */
function markdown(): Plugin {
  return {
    name: "xp-docs-md",
    transform(src, id) {
      if (!id.endsWith(".md")) return;
      const title = /^#\s+(.+)$/m.exec(src)?.[1]?.trim() ?? path.basename(id, ".md");
      const images: string[] = [];
      const html = marked
        .parse(src, { async: false })
        // El título lo pinta la vista, no el documento.
        .replace(/^<h1>.*?<\/h1>\n?/, "")
        .replace(/ href="([^"#:]+)(#[^"]*)?"/g, (_, href: string, hash = "") => {
          const url = new URL(href, DOCS_ON_GITHUB).href;
          const doc = url.startsWith(DOCS_ON_GITHUB) ? /^([\w-]+)\.md$/.exec(url.slice(DOCS_ON_GITHUB.length)) : null;
          if (!doc || MAINTAINER_ONLY.includes(doc[1]!)) return ` href="${url}${hash}"`;
          return ` href="#/docs${doc[1] === "README" ? "" : `/${doc[1]}`}"`;
        })
        .replace(/ src="([^":]+)"/g, (_, src: string) => ` src="__IMG${images.push(src) - 1}__"`);
      const imports = images.map((p, i) => `import i${i} from ${JSON.stringify(p.startsWith(".") ? p : `./${p}`)};`);
      const html_ = JSON.stringify(html) + images.map((_, i) => `.replace("__IMG${i}__", i${i})`).join("");
      return `${imports.join("\n")}\nexport const title = ${JSON.stringify(title)};\nexport const html = ${html_};\n`;
    },
  };
}

// Web del proyecto (GitHub Pages). Reutiliza estilos, geometría del skin, iconos y
// visualizaciones de la app (../src) y la documentación de ../docs.
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  base: "./",
  plugins: [svelte({ configFile: "../svelte.config.js" }), markdown()],
  server: { port: 1440, host: "127.0.0.1" },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
