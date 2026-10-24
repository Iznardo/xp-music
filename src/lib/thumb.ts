// Covers served through the `thumb` protocol (the backend's on-disk cache).

import { convertFileSrc } from "@tauri-apps/api/core";
import type { Thumbnail } from "./bindings";
import { isTauri } from "./platform";

/// Local URL for a Google/YouTube image (in the preview, the original one).
export function thumbUrl(url: string): string {
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
  if (/googleusercontent\.com|ggpht\.com/.test(url) && /=w\d+-h\d+/.test(url)) {
    url = url.replace(/=w\d+-h\d+/, `=w${px}-h${px}`);
  }
  return thumbUrl(url);
}
