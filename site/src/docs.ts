// Páginas de documentación: los .md de ../docs (los convierte el plugin de vite.config.ts).

// Sin las páginas solo para el mantenedor (MAINTAINER_ONLY en vite.config.ts).
const files = import.meta.glob<{ title: string; html: string }>(["../../docs/*.md", "!../../docs/publicar.md"], { eager: true });

/** Orden en el árbol; lo que no esté aquí va al final, por nombre. README es el índice. */
const ORDER = ["README", "instalar", "uso", "mascaras", "preguntas-frecuentes", "compilar"];
const rank = (s: string) => (ORDER.includes(s) ? ORDER.indexOf(s) : ORDER.length);

export const docs = Object.entries(files)
  .map(([p, m]) => ({ file: p.slice(p.lastIndexOf("/") + 1), title: m.title, html: m.html }))
  .map((d) => ({ ...d, slug: d.file === "README.md" ? "" : d.file.slice(0, -3) }))
  .sort((a, b) => rank(a.file.slice(0, -3)) - rank(b.file.slice(0, -3)) || a.file.localeCompare(b.file));
