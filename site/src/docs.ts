// Documentation pages: the .md files in ../../docs (English) and ../../docs/es (Spanish),
// converted by the plugin in vite.config.ts. Both languages use the same file names, so a
// page has the same slug (and route) in each.

import type { Language } from "../../src/lib/locale";

type Page = { title: string; html: string };

// Without the maintainer-only pages (MAINTAINER_ONLY in vite.config.ts).
const en = import.meta.glob<Page>(["../../docs/*.md", "!../../docs/releasing.md"], { eager: true });
const es = import.meta.glob<Page>(["../../docs/es/*.md", "!../../docs/es/releasing.md"], { eager: true });

/** Order in the tree; anything not listed goes last, by name. README is the index. */
const ORDER = ["README", "install", "usage", "skins", "faq", "building"];
const rank = (s: string) => (ORDER.includes(s) ? ORDER.indexOf(s) : ORDER.length);

const pages = (files: Record<string, Page>) =>
  Object.entries(files)
    .map(([p, m]) => ({ file: p.slice(p.lastIndexOf("/") + 1), title: m.title, html: m.html }))
    .map((d) => ({ ...d, slug: d.file === "README.md" ? "" : d.file.slice(0, -3) }))
    .sort((a, b) => rank(a.file.slice(0, -3)) - rank(b.file.slice(0, -3)) || a.file.localeCompare(b.file));

export const docs: Record<Language, ReturnType<typeof pages>> = { en: pages(en), es: pages(es) };

/** Path of a page's source inside docs/, for the "Edit on GitHub" link. */
export const sourcePath = (lang: Language, file: string) => (lang === "es" ? `es/${file}` : file);
