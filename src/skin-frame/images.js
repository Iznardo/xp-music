// Skin images: decoded in the frame (Chromium reads BMP, PNG, JPEG and GIF) and
// the key color (transparencyColor / clippingColor) becomes transparent.
// The pixels are kept for the window shape, the button maps and the clicks
// that must ignore transparent areas.

import { parseColor } from "./util.js";

const MIME = /** @type {Record<string, string>} */ ({
  bmp: "image/bmp", dib: "image/bmp", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif",
});

/** @typedef {{ url: string, w: number, h: number, data: ImageData, canvas: OffscreenCanvas }} SkinImage */

export class Images {
  /** @param {{ read(name: string): Promise<Uint8Array> | null }} zip */
  constructor(zip) {
    this.zip = zip;
    /** @type {Map<string, Promise<SkinImage | null>>} */
    this.cache = new Map();
    /** Names that were not found or could not be read. */
    this.missing = new Set();
  }

  /** Image with the key color applied (or null). @param {string} name @param {unknown} [key] */
  get(name, key) {
    if (!name || typeof name !== "string") return Promise.resolve(null);
    const color = parseColor(key);
    const id = `${name.toLowerCase()}|${color ? color.join(",") : ""}`;
    let p = this.cache.get(id);
    if (!p) {
      p = this.#load(name.trim(), color);
      this.cache.set(id, p);
    }
    return p;
  }

  /** Pixels as they are (button and position maps). @param {string} name */
  raw(name) {
    return this.get(name, null);
  }

  /** @param {string} name @param {number[] | null} color */
  async #load(name, color) {
    // WMP internal resources (res://…, WMPImage_…): they do not exist outside WMP.
    if (/^res:|^wmpimage_/i.test(name)) return null;
    const bytes = this.zip.read(name);
    if (!bytes) {
      this.missing.add(name);
      return null;
    }
    try {
      const raw = await bytes;
      const ext = name.split(".").pop()?.toLowerCase() ?? "";
      const bitmap = await createImageBitmap(new Blob([/** @type {BlobPart} */ (raw)], { type: MIME[ext] ?? "" }));
      const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
      const ctx = /** @type {OffscreenCanvasRenderingContext2D} */ (canvas.getContext("2d", { willReadFrequently: true }));
      ctx.drawImage(bitmap, 0, 0);
      bitmap.close();
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
      let changed = false;
      if (color) {
        const [r, g, b] = color;
        const px = data.data;
        // JPEG never gives back the exact key color (Professional, Xbox use .jpg buttons with #ff00ff).
        const tol = ext === "jpg" || ext === "jpeg" ? 48 : 0;
        for (let i = 0; i < px.length; i += 4) {
          if (Math.abs(px[i] - r) <= tol && Math.abs(px[i + 1] - g) <= tol && Math.abs(px[i + 2] - b) <= tol) {
            px[i + 3] = 0;
            changed = true;
          }
        }
        if (changed) {
          feather(data);
          ctx.putImageData(data, 0, 0);
        }
      }
      // A GIF with no key color goes out as it came: re-encoding it through the canvas would freeze
      // it on the first frame (the Xbox intro starts black and ends transparent).
      // Known limit: an animated GIF that also has key-colored pixels still comes out frozen (it would need per-frame keying).
      const blob = ext === "gif" && !changed ? new Blob([/** @type {BlobPart} */ (raw)], { type: MIME.gif }) : await canvas.convertToBlob();
      const url = URL.createObjectURL(blob);
      return { url, w: canvas.width, h: canvas.height, data, canvas };
    } catch {
      this.missing.add(name);
      return null;
    }
  }
}

/** Is the pixel at (x, y) opaque? @param {SkinImage} img @param {number} x @param {number} y */
export function opaqueAt(img, x, y) {
  x = Math.floor(x);
  y = Math.floor(y);
  if (x < 0 || y < 0 || x >= img.w || y >= img.h) return false;
  return img.data.data[(y * img.w + x) * 4 + 3] > 0;
}

/** Color [r, g, b] at (x, y), or null if outside. @param {SkinImage} img @param {number} x @param {number} y */
export function colorAt(img, x, y) {
  x = Math.floor(x);
  y = Math.floor(y);
  if (x < 0 || y < 0 || x >= img.w || y >= img.h) return null;
  const i = (y * img.w + x) * 4;
  const d = img.data.data;
  return [d[i], d[i + 1], d[i + 2]];
}

/**
 * Smooths the edge of what was cut out by color: a key-color cutout leaves
 * jagged steps (in WMP too). The mask is blurred (5×5 gaussian) and each
 * opaque pixel on the edge takes an opacity based on how much of it remains
 * inside: the corners of the steps get rounded and the interior does not change.
 * Transparent areas stay transparent (their color is the key color).
 * @param {ImageData} img
 */
function feather(img) {
  const { width: w, height: h, data: d } = img;
  const K = [1, 4, 6, 4, 1];
  const solid = new Float32Array(w * h);
  for (let i = 0; i < w * h; i++) solid[i] = d[i * 4 + 3] > 0 ? 1 : 0;
  const tmp = new Float32Array(w * h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      let s = 0;
      for (let k = -2; k <= 2; k++) s += K[k + 2] * solid[y * w + Math.min(w - 1, Math.max(0, x + k))];
      tmp[y * w + x] = s / 16;
    }
  }
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (!solid[i]) continue;
      let s = 0;
      for (let k = -2; k <= 2; k++) s += K[k + 2] * tmp[Math.min(h - 1, Math.max(0, y + k)) * w + x];
      const b = s / 16;
      if (b >= 0.85) continue;
      const a = Math.round(255 * Math.min(1, Math.max(0.15, (b - 0.35) / 0.5)));
      if (a < d[i * 4 + 3]) d[i * 4 + 3] = a;
    }
  }
}
