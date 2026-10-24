// Skin engine utilities.

/**
 * WMP model object as seen from scripts: COM is case-insensitive, so
 * `Player.Controls.Play()` and `player.controls.play()` are the same. The
 * classes define their members in lowercase; unknown things are stored in a
 * bag (skins invent properties) and internals start with "_".
 * @template {object} T @param {T} target @returns {T}
 */
export function wmpObject(target) {
  const bag = Object.create(null);
  return new Proxy(target, {
    get(t, k) {
      if (typeof k === "symbol") return /** @type {any} */ (t)[k];
      if (k === "__target") return t;
      const lk = k.toLowerCase();
      if (lk[0] !== "_" && lk in t) {
        const v = /** @type {any} */ (t)[lk];
        return typeof v === "function" ? v.bind(t) : v;
      }
      if (lk === "tostring") return () => `[object ${t.constructor?.name ?? "Object"}]`;
      if (lk === "valueof") return () => t;
      return bag[lk];
    },
    set(t, k, v) {
      if (typeof k === "symbol") return false;
      const lk = k.toLowerCase();
      const d = lk[0] !== "_" ? describe(t, lk) : undefined;
      // Read-only (a getter without a setter): WMP ignores it without an error.
      if (d && (d.set || (d.writable && typeof d.value !== "function"))) /** @type {any} */ (t)[lk] = v;
      else if (!d) bag[lk] = v;
      return true;
    },
    has(t, k) {
      return typeof k === "string" && (k.toLowerCase() in t || k.toLowerCase() in bag);
    },
  });
}

/** Descriptor of a property on the object or its prototypes. @param {object} o @param {string} k */
function describe(o, k) {
  for (let p = o; p && p !== Object.prototype; p = Object.getPrototypeOf(p)) {
    const d = Object.getOwnPropertyDescriptor(p, k);
    if (d) return d;
  }
  return undefined;
}

/** "#FF00FF", "FF00FF", "#f0f", "none"… → [r, g, b] or null. @param {unknown} v */
export function parseColor(v) {
  if (typeof v !== "string") return null;
  let s = v.trim().toLowerCase();
  if (!s || s === "none" || s === "transparent") return null;
  if (NAMED[s]) s = NAMED[s];
  s = s.replace(/^#/, "");
  if (/^[0-9a-f]{3}$/.test(s)) s = s.replace(/./g, "$&$&");
  if (!/^[0-9a-f]{6}$/.test(s)) return null;
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}

const NAMED = /** @type {Record<string, string>} */ ({
  black: "000000", white: "ffffff", red: "ff0000", green: "008000", lime: "00ff00", blue: "0000ff",
  yellow: "ffff00", magenta: "ff00ff", fuchsia: "ff00ff", cyan: "00ffff", aqua: "00ffff", gray: "808080",
  grey: "808080", silver: "c0c0c0", navy: "000080", maroon: "800000", purple: "800080", olive: "808000",
  teal: "008080", orange: "ffa500",
});

/** CSS color of a skin value, or "" if transparent. @param {unknown} v */
export function cssColor(v) {
  const c = parseColor(v);
  return c ? `rgb(${c[0]} ${c[1]} ${c[2]})` : "";
}

/** "true"/"false"/numbers/booleans from attributes. @param {unknown} v */
export function bool(v) {
  if (typeof v === "string") return !/^(false|0|no|)$/i.test(v.trim());
  return !!v;
}

/** @param {unknown} v @param {number} [def] */
export function num(v, def = 0) {
  const n = typeof v === "number" ? v : parseFloat(String(v));
  return Number.isFinite(n) ? n : def;
}

/** Seconds → "m:ss" (or "h:mm:ss"), like WMP. @param {number} s */
export function timeString(s) {
  if (!Number.isFinite(s) || s < 0) s = 0;
  s = Math.floor(s);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${String(m).padStart(2, "0")}:${ss}`;
}

/** Strips the "jscript:" prefix from a handler or expression. @param {string} code */
export function stripJs(code) {
  return code.replace(/^\s*jscript\s*:/i, "");
}
