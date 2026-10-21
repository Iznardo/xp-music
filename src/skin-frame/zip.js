// Lectura de un .wmz (un zip normal) en el propio marco: directorio central y
// descompresión con DecompressionStream. Los nombres se buscan sin distinguir
// mayúsculas y con o sin carpeta, como hace WMP.

const decoder = new TextDecoder("utf-8");
const legacy = new TextDecoder("windows-1252");

/** @param {ArrayBuffer} buf */
export function openZip(buf) {
  const dv = new DataView(buf);
  const u8 = new Uint8Array(buf);
  let eocd = -1;
  for (let i = u8.length - 22; i >= Math.max(0, u8.length - 22 - 0xffff); i--) {
    if (dv.getUint32(i, true) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) throw new Error("El archivo no es un .wmz válido.");
  const count = dv.getUint16(eocd + 10, true);
  let p = dv.getUint32(eocd + 16, true);
  /** @type {Map<string, {name: string, method: number, offset: number, size: number}>} */
  const byPath = new Map();
  /** @type {Map<string, any>} */
  const byBase = new Map();
  for (let n = 0; n < count; n++) {
    if (p + 46 > u8.length || dv.getUint32(p, true) !== 0x02014b50) throw new Error("El .wmz está dañado.");
    const flags = dv.getUint16(p + 8, true);
    const method = dv.getUint16(p + 10, true);
    const size = dv.getUint32(p + 20, true);
    const nameLen = dv.getUint16(p + 28, true);
    const extraLen = dv.getUint16(p + 30, true);
    const commentLen = dv.getUint16(p + 32, true);
    const offset = dv.getUint32(p + 42, true);
    const raw = u8.subarray(p + 46, p + 46 + nameLen);
    const name = (flags & 0x800 ? decoder : legacy).decode(raw);
    p += 46 + nameLen + extraLen + commentLen;
    if (name.endsWith("/")) continue;
    const entry = { name, method, offset, size };
    const path = normalize(name);
    byPath.set(path, entry);
    const base = path.slice(path.lastIndexOf("/") + 1);
    if (!byBase.has(base)) byBase.set(base, entry);
  }

  /** @type {Map<string, Promise<Uint8Array>>} */
  const cache = new Map();

  async function inflate(entry) {
    const off = entry.offset;
    if (dv.getUint32(off, true) !== 0x04034b50) throw new Error(`Entrada dañada: ${entry.name}`);
    const start = off + 30 + dv.getUint16(off + 26, true) + dv.getUint16(off + 28, true);
    const data = u8.subarray(start, start + entry.size);
    if (entry.method === 0) return data;
    if (entry.method !== 8) throw new Error(`Compresión no soportada en ${entry.name}`);
    const stream = new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }

  return {
    /** Nombres de todos los archivos. */
    names: () => [...byPath.values()].map((e) => e.name),
    /** @param {string} name */
    find(name) {
      const path = normalize(name);
      return byPath.get(path) ?? byBase.get(path.slice(path.lastIndexOf("/") + 1)) ?? null;
    },
    /** @param {string} name @returns {Promise<Uint8Array> | null} */
    read(name) {
      const entry = this.find(name);
      if (!entry) return null;
      let p = cache.get(entry.name);
      if (!p) {
        p = inflate(entry);
        cache.set(entry.name, p);
      }
      return p;
    },
  };
}

/** @param {string} name */
export function normalize(name) {
  return name.replace(/\\/g, "/").replace(/^(\.\/)+/, "").replace(/^\/+/, "").toLowerCase();
}

/** Texto de un .wms o .js: UTF-16 (con o sin BOM), UTF-8 o Windows-1252. */
export function decodeText(/** @type {Uint8Array} */ raw) {
  if (raw[0] === 0xff && raw[1] === 0xfe) return new TextDecoder("utf-16le").decode(raw.subarray(2));
  if (raw[0] === 0xfe && raw[1] === 0xff) return new TextDecoder("utf-16be").decode(raw.subarray(2));
  if (raw.length > 3 && raw[1] === 0 && raw[3] === 0) return new TextDecoder("utf-16le").decode(raw);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(raw).replace(/^﻿/, "");
  } catch {
    return legacy.decode(raw);
  }
}
