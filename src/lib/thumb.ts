// Portadas servidas por el protocolo `thumb` (caché en disco del backend).

import type { Thumbnail } from "./bindings";

/// URL local para una imagen de Google/YouTube.
export function thumbUrl(url: string): string {
  return `http://thumb.localhost/${encodeURIComponent(url)}`;
}

/// La menor miniatura de al menos `px` de ancho. En las de googleusercontent se
/// pide el tamaño exacto reescribiendo el sufijo `=wN-hN`.
export function thumbSrc(thumbs: Thumbnail[] | null | undefined, px: number): string | null {
  if (!thumbs?.length) return null;
  const sorted = [...thumbs].sort((a, b) => a.width - b.width);
  const pick = sorted.find((t) => t.width >= px) ?? sorted[sorted.length - 1]!;
  let url = pick.url;
  if (/googleusercontent\.com|ggpht\.com/.test(url) && /=w\d+-h\d+/.test(url)) {
    url = url.replace(/=w\d+-h\d+/, `=w${px}-h${px}`);
  }
  return thumbUrl(url);
}
