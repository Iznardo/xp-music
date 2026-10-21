// Imágenes de un skin: se decodifican en el marco (Chromium lee BMP, PNG, JPEG y
// GIF) y el color clave (transparencyColor / clippingColor) pasa a transparente.
// Se guardan los píxeles para la forma de la ventana, los mapas de botones y
// los clics que deben ignorar lo transparente.

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
    /** Nombres que no se encontraron o no se pudieron leer. */
    this.missing = new Set();
  }

  /** Imagen con el color clave aplicado (o null). @param {string} name @param {unknown} [key] */
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

  /** Píxeles tal cual (mapas de botones y de posición). @param {string} name */
  raw(name) {
    return this.get(name, null);
  }

  /** @param {string} name @param {number[] | null} color */
  async #load(name, color) {
    // Recursos internos de WMP (res://…, WMPImage_…): no existen fuera de WMP.
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
      if (color) {
        const [r, g, b] = color;
        const px = data.data;
        let changed = false;
        for (let i = 0; i < px.length; i += 4) {
          if (px[i] === r && px[i + 1] === g && px[i + 2] === b) {
            px[i + 3] = 0;
            changed = true;
          }
        }
        if (changed) {
          feather(data);
          ctx.putImageData(data, 0, 0);
        }
      }
      const url = URL.createObjectURL(await canvas.convertToBlob());
      return { url, w: canvas.width, h: canvas.height, data, canvas };
    } catch {
      this.missing.add(name);
      return null;
    }
  }
}

/** ¿Pixel opaco en (x, y)? @param {SkinImage} img @param {number} x @param {number} y */
export function opaqueAt(img, x, y) {
  x = Math.floor(x);
  y = Math.floor(y);
  if (x < 0 || y < 0 || x >= img.w || y >= img.h) return false;
  return img.data.data[(y * img.w + x) * 4 + 3] > 0;
}

/** Color [r, g, b] en (x, y) o null fuera. @param {SkinImage} img @param {number} x @param {number} y */
export function colorAt(img, x, y) {
  x = Math.floor(x);
  y = Math.floor(y);
  if (x < 0 || y < 0 || x >= img.w || y >= img.h) return null;
  const i = (y * img.w + x) * 4;
  const d = img.data.data;
  return [d[i], d[i + 1], d[i + 2]];
}

/**
 * Suaviza el borde de lo recortado por color: un recorte por color clave deja
 * escalones (en WMP también). Se difumina la máscara (gaussiana 5×5) y cada
 * píxel opaco del borde toma una opacidad según cuánto de él queda dentro: las
 * esquinas de los escalones se redondean y lo de dentro no cambia. Lo
 * transparente sigue transparente (su color es el color clave).
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
