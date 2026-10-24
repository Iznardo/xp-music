import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { marked } from "marked";
import { fileURLToPath } from "node:url";
import path from "node:path";

/** Where the .md files live on GitHub: relative links that are not another docs/ page go there. */
const DOCS_ON_GITHUB = "https://github.com/Iznardo/xp-music/blob/main/docs/";
const DOCS_DIR = fileURLToPath(new URL("../docs", import.meta.url));
/** docs/ pages for the maintainer only: not on the website, and links to them go to GitHub.
 *  They must also be excluded in the globs of src/docs.ts. */
const MAINTAINER_ONLY = ["releasing"];

/**
 * `docs/*.md` and `docs/es/*.md` → `{ title, html }`. Links between docs pages become
 * website routes (`#/docs/…`, the language is not part of the route), other relative
 * links go to GitHub, and images are bundled like any other asset.
 */
function markdown(): Plugin {
  return {
    name: "xp-docs-md",
    transform(src, id) {
      if (!id.endsWith(".md")) return;
      const title = /^#\s+(.+)$/m.exec(src)?.[1]?.trim() ?? path.basename(id, ".md");
      // Relative links resolve against the page's own folder (docs/ or docs/es/).
      const base = new URL(path.relative(DOCS_DIR, path.dirname(id)).split(path.sep).join("/") + "/", DOCS_ON_GITHUB);
      const pageIsEs = base.href.startsWith(`${DOCS_ON_GITHUB}es/`);
      // The page's own language only: a link to the other language's copy stays on GitHub.
      const isDoc = (url: string) => {
        const rel = url.slice(DOCS_ON_GITHUB.length);
        return /^([\w-]+)\.md$/.exec(pageIsEs ? (rel.startsWith("es/") ? rel.slice(3) : "") : rel);
      };
      const images: string[] = [];
      const html = marked
        .parse(src, { async: false })
        // The view draws the title, not the document.
        .replace(/^<h1>.*?<\/h1>\n?/, "")
        .replace(/ href="([^"#:]+)(#[^"]*)?"/g, (_, href: string, hash = "") => {
          const url = new URL(href, base).href;
          const doc = url.startsWith(DOCS_ON_GITHUB) ? isDoc(url) : null;
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

// Project website (GitHub Pages). Reuses styles, skin geometry, icons and visualizations
// from the app (../src) and the documentation in ../docs.
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  base: "./",
  plugins: [svelte({ configFile: "../svelte.config.js" }), markdown()],
  server: { port: 1440, host: "127.0.0.1" },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
