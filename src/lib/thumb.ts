// Covers served through the `thumb` protocol (the backend's on-disk cache).

import { convertFileSrc } from "@tauri-apps/api/core";
import type { Thumbnail } from "./bindings";
import { isTauri } from "./platform";

/// Local URL for a Google/YouTube image (in the preview, the original one).
/// `cover:<key>` is the cover of a song on disk (an opaque key), served by the `cover` protocol.
export function thumbUrl(url: string): string {
  if (url.startsWith("cover:")) return isTauri ? convertFileSrc(url.slice(6), "cover") : "";
  if (!isTauri || url.startsWith("data:")) return url;
  return convertFileSrc(url, "thumb");
}

/// The smallest thumbnail at least `px` wide. For googleusercontent ones the
/// exact size is requested by rewriting the `=wN-hN` suffix.
export function thumbSrc(thumbs: Thumbnail[] | null | undefined, px: number): string | null {
  if (!thumbs?.length) return null;
  const sorted = [...thumbs].sort((a, b) => a.width - b.width);
  const pick = sorted.find((t) => t.width >= px) ?? sorted[sorted.length - 1]!;
  let url = pick.url;
  if (url.startsWith("cover:")) return thumbUrl(url) || null;
  if (/googleusercontent\.com|ggpht\.com/.test(url) && /=w\d+-h\d+/.test(url)) {
    url = url.replace(/=w\d+-h\d+/, `=w${px}-h${px}`);
  }
  return thumbUrl(url);
}
