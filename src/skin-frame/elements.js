// Elements of a WMP skin. Each one is a class with its properties in lowercase
// (scripts see it through wmpObject) and a positioned <div>.
// Coordinates are relative to their container (view or subview).

import { wmpObject, cssColor, parseColor, bool, num, timeString } from "./util.js";
import { opaqueAt, colorAt } from "./images.js";
import { queueName } from "./objects.js";

/** @typedef {import("./wms.js").WmsNode} WmsNode */
/** @typedef {import("./images.js").SkinImage} SkinImage */

/** Named WMP buttons: what they do when pressed and their default state. */
const NAMED = /** @type {Record<string, {click?: string, defaults?: Record<string, string>}>} */ ({
  play: { click: "player.controls.play()", defaults: { enabled: "wmpenabled:player.controls.play" } },
  pause: { click: "player.controls.pause()", defaults: { enabled: "wmpenabled:player.controls.pause" } },
  stop: { click: "player.controls.stop()", defaults: { enabled: "wmpenabled:player.controls.stop" } },
  prev: { click: "player.controls.previous()", defaults: { enabled: "wmpenabled:player.controls.previous" } },
  next: { click: "player.controls.next()", defaults: { enabled: "wmpenabled:player.controls.next" } },
  rew: { click: "player.controls.fastReverse()" },
  ffwd: { click: "player.controls.fastForward()" },
  mute: { click: "player.settings.mute = !player.settings.mute", defaults: { sticky: "true", down: "wmpprop:player.settings.mute" } },
  shuffle: { click: "player.settings.setMode('shuffle', !player.settings.getMode('shuffle'))", defaults: { sticky: "true", down: "wmpprop:player.settings.getMode('shuffle')" } },
  repeat: { click: "player.settings.setMode('loop', !player.settings.getMode('loop'))", defaults: { sticky: "true", down: "wmpprop:player.settings.getMode('loop')" } },
  close: { click: "view.close()" },
  minimize: { click: "view.minimize()" },
  maximize: { click: "view.maximize()" },
  restore: { click: "view.restore()" },
  return: { click: "view.returnToMediaCenter()" },
});

/** "playbutton" / "playelement" → NAMED entry. @param {string} tag */
function named(tag) {
  const m = /^(\w+?)(button|element)$/.exec(tag);
  return m ? NAMED[m[1]] : undefined;
}

export const TEXT_TAGS = {
  text: null,
  currentpositiontext: "wmpprop:player.controls.currentPositionString",
  durationtext: "wmpprop:player.currentMedia.durationString",
  statustext: "wmpprop:player.status",
  tracknametext: "wmpprop:player.currentMedia.name",
};

export class El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    this._rt = rt;
    this._node = node;
    this._tag = node.tag;
    this._parent = parent;
    /** @type {El[]} */
    this._children = [];
    /** @type {Record<string, string>} event (lowercase, without "on") → code */
    this._handlers = {};
    this._dom = document.createElement("div");
    this._dom.className = `wmp-${node.tag}`;
    // As in WMP, nothing is painted outside its own box.
    this._dom.style.overflow = "hidden";
    this._left = 0;
    this._top = 0;
    /** @type {number | null} */
    this._width = null;
    /** @type {number | null} */
    this._height = null;
    this._visible = true;
    this._enabled = true;
    this._z = 0;
    this._alpha = 255;
    this._id = "";
    this._tooltip = "";
    this._cursor = "";
    this._bgColor = "";
    this._bgName = "";
    this._transparency = "";
    this._clipping = "";
    this._clippingImage = "";
    /** Image that defines the shape (background or main image). @type {SkinImage | null} */
    this._mask = null;
    this._hover = false;
    this._pressed = false;
    /** Scope of its handlers (created by the engine). @type {object | null} */
    this._scope = null;
    this.__proxy = wmpObject(this);
  }

  /** What scripts see. */
  get _proxy() {
    return this.__proxy;
  }

  // --- Common properties ---------------------------------------------------
  get id() {
    return this._id;
  }
  set id(v) {
    this._id = String(v);
  }
  get left() {
    return this._left;
  }
  set left(v) {
    this._left = num(v);
    this._dom.style.left = `${this._left}px`;
    this._rt.shapeDirty();
  }
  get top() {
    return this._top;
  }
  set top(v) {
    this._top = num(v);
    this._dom.style.top = `${this._top}px`;
    this._rt.shapeDirty();
  }
  get width() {
    return this._width ?? this._naturalW();
  }
  set width(v) {
    this._width = num(v);
    this._layout();
  }
  get height() {
    return this._height ?? this._naturalH();
  }
  set height(v) {
    this._height = num(v);
    this._layout();
  }
  get visible() {
    return this._visible;
  }
  set visible(v) {
    const b = bool(v);
    if (b === this._visible) return;
    this._visible = b;
    this._dom.style.display = b ? "" : "none";
    this._rt.shapeDirty();
  }
  get enabled() {
    return this._enabled;
  }
  set enabled(v) {
    this._enabled = bool(v);
    this._paint();
  }
  get zindex() {
    return this._z;
  }
  set zindex(v) {
    this._z = num(v);
    this._dom.style.zIndex = String(this._z);
  }
  get alphablend() {
    return this._alpha;
  }
  set alphablend(v) {
    this._alpha = Math.max(0, Math.min(255, num(v, 255)));
    this._dom.style.opacity = String(this._alpha / 255);
  }
  get tooltip() {
    return this._tooltip;
  }
  set tooltip(v) {
    this._tooltip = String(v ?? "");
  }
  get uptooltip() {
    return this._tooltip;
  }
  set uptooltip(v) {
    this._tooltip = String(v ?? "");
  }
  get cursor() {
    return this._cursor;
  }
  set cursor(v) {
    this._cursor = String(v ?? "");
  }
  get backgroundcolor() {
    return this._bgColor;
  }
  set backgroundcolor(v) {
    this._bgColor = String(v ?? "");
    this._dom.style.backgroundColor = cssColor(v);
  }
  get backgroundimage() {
    return this._bgName;
  }
  set backgroundimage(v) {
    this._bgName = String(v ?? "");
    this._loadBackground();
  }
  get transparencycolor() {
    return this._transparency;
  }
  set transparencycolor(v) {
    this._transparency = String(v ?? "");
    if (this._bgName) this._loadBackground();
  }
  get clippingcolor() {
    return this._clipping;
  }
  set clippingcolor(v) {
    this._clipping = String(v ?? "");
    if (this._bgName) this._loadBackground();
  }
  get clippingimage() {
    return this._clippingImage;
  }
  set clippingimage(v) {
    // Some editors write clippingImage="0" for "none".
    this._clippingImage = /^(0|none|false)?$/i.test(String(v ?? "").trim()) ? "" : String(v);
    this._loadBackground();
  }

  // --- Script methods --------------------------------------------------
  /** @param {unknown} x @param {unknown} y @param {unknown} ms */
  moveto(x, y, ms) {
    this._rt.animate(this, { left: num(x, this._left), top: num(y, this._top) }, num(ms), "endmove");
  }
  /** @param {unknown} a @param {unknown} ms */
  alphablendto(a, ms) {
    this._rt.animate(this, { alphablend: num(a, 255) }, num(ms), "endalphablend");
  }
  setfocus() {}
  focus() {}

  // --- Internal -----------------------------------------------------------------
  /** Key color for the images of this element. */
  _key() {
    return this._transparency || this._clipping || null;
  }

  _naturalW() {
    return this._mask?.w ?? 0;
  }
  _naturalH() {
    return this._mask?.h ?? 0;
  }

  async _loadBackground() {
    const name = this._bgName;
    const img = name ? await this._rt.images.get(name, this._key()) : null;
    if (name !== this._bgName) return;
    this._dom.style.backgroundImage = img ? `url("${img.url}")` : "";
    this._dom.style.backgroundRepeat = "no-repeat";
    // The click shape: clippingImage if there is one; otherwise the background.
    let mask = img;
    if (this._clippingImage) mask = (await this._rt.images.get(this._clippingImage, this._clipping || "#ff00ff")) ?? img;
    this._mask = mask;
    // With clippingColor/clippingImage, WMP also clips what is inside:
    // some skins have images that fill the area outside the shape with the key color.
    const clip = (this._clipping || this._clippingImage) && mask ? `url("${mask.url}")` : "";
    Object.assign(this._dom.style, { maskImage: clip, maskRepeat: "no-repeat", maskPosition: "0 0", maskMode: "alpha" });
    this._layout();
    this._rt.shapeDirty();
  }

  _layout() {
    const w = this._width ?? this._naturalW();
    const h = this._height ?? this._naturalH();
    this._dom.style.width = `${w}px`;
    this._dom.style.height = `${h}px`;
    this._rt.shapeDirty();
  }

  /** Repaints the visual state (buttons, sliders…). */
  _paint() {}

  /** Accepts clicks (has handlers or behavior of its own). */
  _interactive() {
    const h = this._handlers;
    return !!(h.click || h.mousedown || h.mouseup || h.dblclick || h.mouseover || h.mouseout || h.mousemove);
  }

  /** Does the point (relative to the element) belong to it? */
  _hits(/** @type {number} */ x, /** @type {number} */ y) {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return false;
    return this._mask ? opaqueAt(this._mask, x, y) : true;
  }

  /** Pointer target inside this element (or itself). @returns {El | null} */
  _target(/** @type {number} */ x, /** @type {number} */ y) {
    return this._interactive() && this._hits(x, y) ? this : null;
  }

  /** What it paints, for the window shape: image or rectangle. */
  _coverage() {
    if (this._mask) return this._mask;
    return this._bgColor && cssColor(this._bgColor) ? "rect" : null;
  }

  /** @param {string} type @param {{x: number, y: number}} p @param {PointerEvent | MouseEvent} e */
  _pointer(type, p, e) {
    if (type === "over" || type === "out") {
      this._hover = type === "over";
      this._paint();
      this._rt.fire(this, type === "over" ? "mouseover" : "mouseout");
      return;
    }
    if (type === "down") {
      this._pressed = true;
      this._paint();
      this._rt.fire(this, "mousedown");
    } else if (type === "move") {
      this._rt.fire(this, "mousemove");
    } else if (type === "up") {
      const was = this._pressed;
      this._pressed = false;
      this._paint();
      this._rt.fire(this, "mouseup");
      if (was && this._hits(p.x, p.y) && this._enabled) this._click(e);
    } else if (type === "dblclick") {
      this._rt.fire(this, "dblclick");
    }
  }

  /** @param {PointerEvent | MouseEvent} _e */
  _click(_e) {
    this._rt.fire(this, "click");
  }

  /** Property changed by the user or a script: notifies `prop_onchange`. */
  _changed(/** @type {string} */ prop) {
    if (!this._rt.fromBinding) this._rt.fire(this, `${prop}_onchange`);
  }
}

// ---------------------------------------------------------------------------------
export class Subview extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._dom.style.overflow = "hidden";
  }
  _naturalW() {
    return this._mask?.w ?? this._parent?.width ?? 0;
  }
  _naturalH() {
    return this._mask?.h ?? this._parent?.height ?? 0;
  }
  _target(/** @type {number} */ x, /** @type {number} */ y) {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return null;
    const kids = this._rt.paintOrder(this._children);
    for (let i = kids.length - 1; i >= 0; i--) {
      const c = kids[i];
      if (!c._visible) continue;
      const t = c._target(x - c._left, y - c._top);
      if (t) return t;
    }
    return super._target(x, y);
  }
}

// ---------------------------------------------------------------------------------
export class Button extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._img = { image: "", hoverimage: "", downimage: "", disabledimage: "", hoverdownimage: "" };
    /** @type {Record<string, SkinImage | null>} */
    this._loaded = {};
    this._sticky = false;
    this._down = false;
    this._downTip = "";
    this._named = named(node.tag);
    this._dom.style.backgroundRepeat = "no-repeat";
  }
  get image() {
    return this._img.image;
  }
  set image(v) {
    this._setImg("image", v);
  }
  get hoverimage() {
    return this._img.hoverimage;
  }
  set hoverimage(v) {
    this._setImg("hoverimage", v);
  }
  get downimage() {
    return this._img.downimage;
  }
  set downimage(v) {
    this._setImg("downimage", v);
  }
  get disabledimage() {
    return this._img.disabledimage;
  }
  set disabledimage(v) {
    this._setImg("disabledimage", v);
  }
  get hoverdownimage() {
    return this._img.hoverdownimage;
  }
  set hoverdownimage(v) {
    this._setImg("hoverdownimage", v);
  }
  get sticky() {
    return this._sticky;
  }
  set sticky(v) {
    this._sticky = bool(v);
  }
  get down() {
    return this._down;
  }
  set down(v) {
    const b = bool(v);
    if (b === this._down) return;
    this._down = b;
    this._paint();
    this._changed("down");
  }
  get downtooltip() {
    return this._downTip;
  }
  set downtooltip(v) {
    this._downTip = String(v ?? "");
  }
  get tooltip() {
    return this._down && this._downTip ? this._downTip : this._tooltip;
  }
  set tooltip(v) {
    this._tooltip = String(v ?? "");
  }
  set transparencycolor(v) {
    this._transparency = String(v ?? "");
    for (const k of Object.keys(this._img)) this._setImg(/** @type {any} */ (k), this._img[/** @type {keyof typeof this._img} */ (k)]);
  }
  get transparencycolor() {
    return this._transparency;
  }

  /** @param {keyof Button["_img"]} slot @param {unknown} v */
  async _setImg(slot, v) {
    const name = String(v ?? "");
    this._img[slot] = name;
    // Igual que en ButtonGroup: sin transparencyColor, el magenta sigue siendo la clave.
    const img = name ? await this._rt.images.get(name, this._key() ?? "#ff00ff") : null;
    if (this._img[slot] !== name) return;
    this._loaded[slot] = img;
    if (slot === "image") {
      this._mask = img;
      this._layout();
    }
    this._paint();
  }

  _interactive() {
    return true;
  }

  _paint() {
    const L = this._loaded;
    let img = L.image;
    if (!this._enabled) img = L.disabledimage ?? img;
    else if (this._pressed || this._down) img = (this._hover && L.hoverdownimage) || L.downimage || (this._hover && L.hoverimage) || img;
    else if (this._hover) img = L.hoverimage ?? img;
    this._dom.style.backgroundImage = img ? `url("${img.url}")` : "";
  }

  /** @param {PointerEvent | MouseEvent} e */
  _click(e) {
    if (this._sticky) this.down = !this._down;
    if (this._handlers.click) this._rt.fire(this, "click");
    else if (this._named?.click) this._rt.run(this._named.click, this, e);
  }
}

// ---------------------------------------------------------------------------------
/** Buttons of a BUTTONGROUP: the shape and the click come from the color in the map. */
export class ButtonElement extends Button {
  /** @param {any} rt @param {WmsNode} node @param {ButtonGroup} group */
  constructor(rt, node, group) {
    super(rt, node, group);
    this._group = group;
    this._mapping = /** @type {number[] | null} */ (null);
    this._canvas = document.createElement("canvas");
    this._canvas.style.position = "absolute";
    this._dom = this._canvas;
    /** @type {HTMLCanvasElement | null} */
    this._maskCanvas = null;
    this._box = { x: 0, y: 0, w: 0, h: 0 };
  }
  get mappingcolor() {
    return this._mapping ? `#${this._mapping.map((c) => c.toString(16).padStart(2, "0")).join("")}` : "";
  }
  set mappingcolor(v) {
    this._mapping = parseColor(v);
  }
  // In a group, the group paints the state with its images; the element's own
  // (uncommon) ones are ignored.
  async _setImg() {}
  _layout() {}

  /** Computes the mask from the group map. @param {SkinImage} map */
  _build(map) {
    if (!this._mapping) return;
    const [r, g, b] = this._mapping;
    const d = map.data.data;
    let x0 = map.w, y0 = map.h, x1 = -1, y1 = -1;
    for (let y = 0; y < map.h; y++) {
      for (let x = 0; x < map.w; x++) {
        const i = (y * map.w + x) * 4;
        if (d[i] === r && d[i + 1] === g && d[i + 2] === b) {
          if (x < x0) x0 = x;
          if (x > x1) x1 = x;
          if (y < y0) y0 = y;
          if (y > y1) y1 = y;
        }
      }
    }
    if (x1 < 0) return;
    const w = x1 - x0 + 1;
    const h = y1 - y0 + 1;
    this._box = { x: x0, y: y0, w, h };
    const mask = document.createElement("canvas");
    mask.width = w;
    mask.height = h;
    const mctx = /** @type {CanvasRenderingContext2D} */ (mask.getContext("2d"));
    const md = mctx.createImageData(w, h);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = ((y + y0) * map.w + (x + x0)) * 4;
        if (d[i] === r && d[i + 1] === g && d[i + 2] === b) md.data[(y * w + x) * 4 + 3] = 255;
      }
    }
    mctx.putImageData(md, 0, 0);
    this._maskCanvas = mask;
    Object.assign(this._canvas.style, { left: `${x0}px`, top: `${y0}px`, width: `${w}px`, height: `${h}px` });
    this._canvas.width = w;
    this._canvas.height = h;
    this._paint();
  }

  _paint() {
    const g = this._group;
    if (!g || !this._maskCanvas) return;
    const L = g._loaded;
    let img = null;
    if (!this._enabled) img = L.disabledimage;
    else if (this._pressed || this._down) img = (this._hover && L.hoverdownimage) || L.downimage || null;
    else if (this._hover) img = L.hoverimage;
    const ctx = /** @type {CanvasRenderingContext2D} */ (this._canvas.getContext("2d"));
    const { x, y, w, h } = this._box;
    ctx.globalCompositeOperation = "source-over";
    ctx.clearRect(0, 0, w, h);
    if (!img) return;
    ctx.drawImage(img.canvas, -x, -y);
    ctx.globalCompositeOperation = "destination-in";
    ctx.drawImage(this._maskCanvas, 0, 0);
  }

  get visible() {
    return this._visible;
  }
  set visible(v) {
    this._visible = bool(v);
    this._canvas.style.display = this._visible ? "" : "none";
  }
  get width() {
    return this._box.w;
  }
  set width(_v) {}
  get height() {
    return this._box.h;
  }
  set height(_v) {}
  _coverage() {
    return null;
  }
}

export class ButtonGroup extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._names = { mappingimage: "", image: "", hoverimage: "", downimage: "", disabledimage: "", hoverdownimage: "" };
    /** @type {Record<string, SkinImage | null>} */
    this._loaded = {};
    /** @type {SkinImage | null} */
    this._map = null;
    /** @type {ButtonElement[]} */
    this._elements = [];
  }
  get mappingimage() {
    return this._names.mappingimage;
  }
  set mappingimage(v) {
    this._set("mappingimage", v);
  }
  get image() {
    return this._names.image;
  }
  set image(v) {
    this._set("image", v);
  }
  get hoverimage() {
    return this._names.hoverimage;
  }
  set hoverimage(v) {
    this._set("hoverimage", v);
  }
  get downimage() {
    return this._names.downimage;
  }
  set downimage(v) {
    this._set("downimage", v);
  }
  get disabledimage() {
    return this._names.disabledimage;
  }
  set disabledimage(v) {
    this._set("disabledimage", v);
  }
  get hoverdownimage() {
    return this._names.hoverdownimage;
  }
  set hoverdownimage(v) {
    this._set("hoverdownimage", v);
  }

  /** @param {keyof ButtonGroup["_names"]} slot @param {unknown} v */
  async _set(slot, v) {
    const name = String(v ?? "");
    this._names[slot] = name;
    // Las imágenes de un grupo de botones llevan magenta fuera de la forma aunque el
    // grupo no declare transparencyColor (WMP no lo pinta): magenta es el color clave por defecto.
    const img = name ? await (slot === "mappingimage" ? this._rt.images.raw(name) : this._rt.images.get(name, this._key() ?? "#ff00ff")) : null;
    if (this._names[slot] !== name) return;
    this._loaded[slot] = img;
    if (slot === "mappingimage") {
      this._map = img;
      this._layout();
      if (img) for (const el of this._elements) el._build(img);
    } else if (slot === "image") {
      this._dom.style.backgroundImage = img ? `url("${img.url}")` : "";
    }
    for (const el of this._elements) el._paint();
  }

  _naturalW() {
    return this._map?.w ?? 0;
  }
  _naturalH() {
    return this._map?.h ?? 0;
  }
  _interactive() {
    return true;
  }
  _target(/** @type {number} */ x, /** @type {number} */ y) {
    if (!this._map) return null;
    const c = colorAt(this._map, x, y);
    if (!c) return null;
    for (let i = this._elements.length - 1; i >= 0; i--) {
      const el = this._elements[i];
      const m = el._mapping;
      if (m && el._visible && m[0] === c[0] && m[1] === c[1] && m[2] === c[2]) return el;
    }
    return null;
  }
  _coverage() {
    return this._loaded.image ?? null;
  }
}

/** The point, relative to the group button, is still inside its color. */
ButtonElement.prototype._hits = function (x, y) {
  const g = this._group;
  return !!g && g._target(x + this._box.x, y + this._box.y) === this;
};

// ---------------------------------------------------------------------------------
/** SLIDER, SEEKSLIDER, VOLUMESLIDER, BALANCESLIDER and PROGRESSBAR. */
export class Slider extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._min = 0;
    this._max = 100;
    this._value = 0;
    this._vertical = false;
    this._border = 0;
    this._slide = true;
    this._tiled = false;
    this._names = { backgroundimage: "", foregroundimage: "", thumbimage: "", thumbhoverimage: "", thumbdownimage: "", thumbdisabledimage: "" };
    /** @type {Record<string, SkinImage | null>} */
    this._loaded = {};
    this._fg = document.createElement("div");
    this._thumb = document.createElement("div");
    for (const d of [this._fg, this._thumb]) {
      d.style.position = "absolute";
      d.style.backgroundRepeat = "no-repeat";
      this._dom.append(d);
    }
    this._dom.style.backgroundRepeat = "no-repeat";
    this._dragging = false;
    this._progress = node.tag === "progressbar";
  }
  get min() {
    return this._min;
  }
  set min(v) {
    this._min = num(v);
    this._paint();
  }
  get max() {
    return this._max;
  }
  set max(v) {
    this._max = num(v, 100);
    this._paint();
  }
  get value() {
    return this._value;
  }
  set value(v) {
    const n = num(v);
    if (n === this._value) return;
    this._value = n;
    this._paint();
    this._changed("value");
  }
  get direction() {
    return this._vertical ? "vertical" : "horizontal";
  }
  set direction(v) {
    this._vertical = String(v).toLowerCase() === "vertical";
    this._paint();
  }
  get bordersize() {
    return this._border;
  }
  set bordersize(v) {
    this._border = num(v);
    this._paint();
  }
  get slide() {
    return this._slide;
  }
  set slide(v) {
    this._slide = bool(v);
  }
  get tiled() {
    return this._tiled;
  }
  set tiled(v) {
    this._tiled = bool(v);
    this._paint();
  }
  get backgroundimage() {
    return this._names.backgroundimage;
  }
  set backgroundimage(v) {
    this._set("backgroundimage", v);
  }
  get foregroundimage() {
    return this._names.foregroundimage;
  }
  set foregroundimage(v) {
    this._set("foregroundimage", v);
  }
  get thumbimage() {
    return this._names.thumbimage;
  }
  set thumbimage(v) {
    this._set("thumbimage", v);
  }
  get thumbhoverimage() {
    return this._names.thumbhoverimage;
  }
  set thumbhoverimage(v) {
    this._set("thumbhoverimage", v);
  }
  get thumbdownimage() {
    return this._names.thumbdownimage;
  }
  set thumbdownimage(v) {
    this._set("thumbdownimage", v);
  }
  get thumbdisabledimage() {
    return this._names.thumbdisabledimage;
  }
  set thumbdisabledimage(v) {
    this._set("thumbdisabledimage", v);
  }
  set transparencycolor(v) {
    this._transparency = String(v ?? "");
    for (const k of Object.keys(this._names)) this._set(/** @type {any} */ (k), this._names[/** @type {keyof typeof this._names} */ (k)]);
  }
  get transparencycolor() {
    return this._transparency;
  }

  /** @param {keyof Slider["_names"]} slot @param {unknown} v */
  async _set(slot, v) {
    const name = String(v ?? "");
    this._names[slot] = name;
    const img = name ? await this._rt.images.get(name, this._key()) : null;
    if (this._names[slot] !== name) return;
    this._loaded[slot] = img;
    if (slot === "backgroundimage") {
      this._mask = img;
      this._layout();
    }
    this._paint();
  }

  _frac() {
    const span = this._max - this._min;
    return span > 0 ? Math.max(0, Math.min(1, (this._value - this._min) / span)) : 0;
  }

  _paint() {
    const L = this._loaded;
    const w = this.width;
    const h = this.height;
    const bg = L.backgroundimage;
    this._dom.style.backgroundImage = bg ? `url("${bg.url}")` : "";
    this._dom.style.backgroundRepeat = this._tiled ? "repeat" : "no-repeat";
    const thumb = (!this._enabled && L.thumbdisabledimage) || (this._dragging && L.thumbdownimage) || (this._hover && L.thumbhoverimage) || L.thumbimage;
    const tw = thumb?.w ?? 0;
    const th = thumb?.h ?? 0;
    const f = this._frac();
    const fg = L.foregroundimage;
    if (!this._vertical) {
      const usable = Math.max(0, w - 2 * this._border - tw);
      const x = this._border + f * usable;
      Object.assign(this._thumb.style, { left: `${x}px`, top: `${Math.round((h - th) / 2)}px`, width: `${tw}px`, height: `${th}px` });
      const fw = tw ? x + tw / 2 : this._border + f * (w - 2 * this._border);
      Object.assign(this._fg.style, { left: "0", top: "0", width: `${fw}px`, height: `${h}px`, backgroundPosition: "0 0" });
    } else {
      const usable = Math.max(0, h - 2 * this._border - th);
      const y = this._border + (1 - f) * usable;
      Object.assign(this._thumb.style, { top: `${y}px`, left: `${Math.round((w - tw) / 2)}px`, width: `${tw}px`, height: `${th}px` });
      const fy = th ? y + th / 2 : this._border + (1 - f) * (h - 2 * this._border);
      Object.assign(this._fg.style, { left: "0", top: `${fy}px`, width: `${w}px`, height: `${h - fy}px`, backgroundPosition: `0 ${-fy}px` });
    }
    this._fg.style.backgroundImage = fg ? `url("${fg.url}")` : "";
    this._fg.style.backgroundRepeat = this._tiled ? "repeat" : "no-repeat";
    this._thumb.style.backgroundImage = thumb ? `url("${thumb.url}")` : "";
  }

  _interactive() {
    return !this._progress || super._interactive();
  }
  _hits(/** @type {number} */ x, /** @type {number} */ y) {
    return x >= 0 && y >= 0 && x < this.width && y < this.height;
  }

  /** Value for a point on the slider. @param {{x: number, y: number}} p */
  _valueAt(p) {
    const tw = this._loaded.thumbimage?.w ?? 0;
    const th = this._loaded.thumbimage?.h ?? 0;
    let f;
    if (!this._vertical) f = (p.x - this._border - tw / 2) / Math.max(1, this.width - 2 * this._border - tw);
    else f = 1 - (p.y - this._border - th / 2) / Math.max(1, this.height - 2 * this._border - th);
    f = Math.max(0, Math.min(1, f));
    return this._min + f * (this._max - this._min);
  }

  /** @param {string} type @param {{x: number, y: number}} p @param {PointerEvent | MouseEvent} e */
  _pointer(type, p, e) {
    if (this._progress || !this._enabled) return super._pointer(type, p, e);
    if (type === "down") {
      this._dragging = true;
      this._rt.fire(this, "mousedown");
      this._drag(p);
    } else if (type === "move" && this._dragging) {
      this._drag(p);
    } else if (type === "up" && this._dragging) {
      this._dragging = false;
      this._drag(p);
      if (!this._slide) this._rt.fire(this, "value_onchange");
      this._rt.fire(this, "mouseup");
      this._paint();
    } else if (type === "over" || type === "out") {
      super._pointer(type, p, e);
    }
  }

  /** @param {{x: number, y: number}} p */
  _drag(p) {
    const v = this._valueAt(p);
    if (v === this._value) return;
    this._value = v;
    this._paint();
    // With slide="false" the change is notified on release.
    if (this._slide) this._changed("value");
  }
}

// ---------------------------------------------------------------------------------
/** CUSTOMSLIDER: a frame strip and a grayscale map that gives the value. */
export class CustomSlider extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._min = 0;
    this._max = 100;
    this._value = 0;
    this._names = { image: "", hoverimage: "", downimage: "", positionimage: "" };
    /** @type {Record<string, SkinImage | null>} */
    this._loaded = {};
    this._dragging = false;
    this._dom.style.backgroundRepeat = "no-repeat";
  }
  get min() {
    return this._min;
  }
  set min(v) {
    this._min = num(v);
    this._paint();
  }
  get max() {
    return this._max;
  }
  set max(v) {
    this._max = num(v, 100);
    this._paint();
  }
  get value() {
    return this._value;
  }
  set value(v) {
    const n = num(v);
    if (n === this._value) return;
    this._value = n;
    this._paint();
    this._changed("value");
  }
  get image() {
    return this._names.image;
  }
  set image(v) {
    this._set("image", v);
  }
  get hoverimage() {
    return this._names.hoverimage;
  }
  set hoverimage(v) {
    this._set("hoverimage", v);
  }
  get downimage() {
    return this._names.downimage;
  }
  set downimage(v) {
    this._set("downimage", v);
  }
  get positionimage() {
    return this._names.positionimage;
  }
  set positionimage(v) {
    this._set("positionimage", v);
  }

  /** @param {keyof CustomSlider["_names"]} slot @param {unknown} v */
  async _set(slot, v) {
    const name = String(v ?? "");
    this._names[slot] = name;
    const img = name ? await (slot === "positionimage" ? this._rt.images.raw(name) : this._rt.images.get(name, this._key())) : null;
    if (this._names[slot] !== name) return;
    this._loaded[slot] = img;
    this._layout();
    this._paint();
  }
  _naturalW() {
    return this._loaded.positionimage?.w ?? this._loaded.image?.h ?? 0;
  }
  _naturalH() {
    return this._loaded.positionimage?.h ?? this._loaded.image?.h ?? 0;
  }

  _paint() {
    const L = this._loaded;
    const img = (this._dragging && L.downimage) || (this._hover && L.hoverimage) || L.image;
    if (!img) {
      this._dom.style.backgroundImage = "";
      return;
    }
    const w = Math.max(1, this.width);
    const frames = Math.max(1, Math.floor(img.w / w));
    const span = this._max - this._min;
    const f = span > 0 ? Math.max(0, Math.min(1, (this._value - this._min) / span)) : 0;
    const i = Math.round(f * (frames - 1));
    this._dom.style.backgroundImage = `url("${img.url}")`;
    this._dom.style.backgroundPosition = `${-i * w}px 0`;
  }

  _interactive() {
    return true;
  }
  _hits(/** @type {number} */ x, /** @type {number} */ y) {
    const pos = this._loaded.positionimage;
    if (!pos) return x >= 0 && y >= 0 && x < this.width && y < this.height;
    const c = colorAt(pos, x, y);
    if (!c) return false;
    const key = parseColor(this._transparency || "#ff00ff");
    return !(key && c[0] === key[0] && c[1] === key[1] && c[2] === key[2]);
  }

  /** @param {{x: number, y: number}} p */
  _drag(p) {
    const pos = this._loaded.positionimage;
    const c = pos ? colorAt(pos, p.x, p.y) : null;
    if (!c || !this._hits(p.x, p.y)) return;
    const gray = (c[0] + c[1] + c[2]) / 3;
    const v = this._min + (gray / 255) * (this._max - this._min);
    if (v === this._value) return;
    this._value = v;
    this._paint();
    this._changed("value");
  }

  /** @param {string} type @param {{x: number, y: number}} p @param {PointerEvent | MouseEvent} e */
  _pointer(type, p, e) {
    if (!this._enabled) return;
    if (type === "down") {
      this._dragging = true;
      this._rt.fire(this, "mousedown");
      this._drag(p);
    } else if (type === "move" && this._dragging) this._drag(p);
    else if (type === "up" && this._dragging) {
      this._dragging = false;
      this._paint();
      this._rt.fire(this, "mouseup");
    } else super._pointer(type, p, e);
  }
}

// ---------------------------------------------------------------------------------
export class Text extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._value = "";
    this._fg = "#000000";
    this._hoverFg = "";
    this._font = "Tahoma";
    this._size = 8;
    this._style = "";
    this._justify = "left";
    this._scrolling = false;
    this._wrap = false;
    this._span = document.createElement("span");
    this._dom.append(this._span);
    Object.assign(this._dom.style, { whiteSpace: "nowrap", overflow: "hidden" });
    this._applyFont();
  }
  get value() {
    return this._value;
  }
  set value(v) {
    const s = v === undefined || v === null ? "" : String(v);
    if (s === this._value) return;
    this._value = s;
    this._span.textContent = s;
    this._layout();
    this._marquee();
    this._changed("value");
  }
  get foregroundcolor() {
    return this._fg;
  }
  set foregroundcolor(v) {
    this._fg = String(v ?? "");
    this._paint();
  }
  get hoverforegroundcolor() {
    return this._hoverFg;
  }
  set hoverforegroundcolor(v) {
    this._hoverFg = String(v ?? "");
  }
  get fontface() {
    return this._font;
  }
  set fontface(v) {
    this._font = String(v ?? "Tahoma");
    this._applyFont();
  }
  get fontsize() {
    return this._size;
  }
  set fontsize(v) {
    this._size = num(v, 8);
    this._applyFont();
  }
  get fontstyle() {
    return this._style;
  }
  set fontstyle(v) {
    this._style = String(v ?? "").toLowerCase();
    this._applyFont();
  }
  get justification() {
    return this._justify;
  }
  set justification(v) {
    this._justify = String(v ?? "left").toLowerCase();
    this._dom.style.textAlign = this._justify === "center" ? "center" : this._justify === "right" ? "right" : "left";
  }
  get scrolling() {
    return this._scrolling;
  }
  set scrolling(v) {
    this._scrolling = bool(v);
    this._marquee();
  }
  get wordwrap() {
    return this._wrap;
  }
  set wordwrap(v) {
    this._wrap = bool(v);
    this._dom.style.whiteSpace = this._wrap ? "normal" : "nowrap";
  }
  get textwidth() {
    return Math.ceil(this._span.offsetWidth || this._measure());
  }
  get textheight() {
    return Math.ceil(this._span.offsetHeight || this._size * 1.6);
  }

  _measure() {
    const c = Text._ctx ??= /** @type {CanvasRenderingContext2D} */ (document.createElement("canvas").getContext("2d"));
    c.font = this._dom.style.font;
    return c.measureText(this._value).width;
  }

  _applyFont() {
    const s = this._style;
    const px = (this._size * 96) / 72;
    this._dom.style.font = `${s.includes("italic") ? "italic " : ""}${s.includes("bold") ? "bold " : ""}${px}px "${this._font}", Tahoma, sans-serif`;
    this._dom.style.textDecoration = s.includes("underline") ? "underline" : "";
    this._dom.style.lineHeight = this._height ? `${this._height}px` : "";
    this._layout();
  }
  _naturalW() {
    return this._measure();
  }
  _naturalH() {
    return Math.ceil((this._size * 96) / 72 + 4);
  }
  _layout() {
    super._layout();
    this._dom.style.lineHeight = this._height && !this._wrap ? `${this._height}px` : "";
  }
  _paint() {
    this._dom.style.color = cssColor(this._hover && this._hoverFg ? this._hoverFg : this._fg) || "transparent";
  }
  _marquee() {
    const run = this._scrolling && this._width !== null && this._measure() > this._width;
    this._span.classList.toggle("marquee", run);
    if (run) this._span.style.setProperty("--dist", `${-(this._measure() + 20)}px`);
  }
  _coverage() {
    return this._bgColor && cssColor(this._bgColor) ? "rect" : null;
  }
}

// ---------------------------------------------------------------------------------
/** Visualization: bars with the app's audio analysis. */
export class Effects extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._canvas = document.createElement("canvas");
    Object.assign(this._canvas.style, { width: "100%", height: "100%", display: "block" });
    this._dom.append(this._canvas);
    const bars = node.children.find((c) => /^(bars|visualization|alchemy)$/.test(c.tag));
    this._level = bars?.attrs.levelcolor || node.attrs.foregroundcolor || "#00cc66";
    this._peak = bars?.attrs.peakcolor || this._level;
    this._mode = 0;
    rt.effects.add(this);
  }
  get currenteffecttitle() {
    return ["Barras", "Onda", "Nivel"][this._mode];
  }
  get currentpresettitle() {
    return this.currenteffecttitle;
  }
  get currenteffecttype() {
    return this.currenteffecttitle;
  }
  get currentpreset() {
    return this._mode;
  }
  set currentpreset(v) {
    this._mode = Math.abs(Math.floor(num(v))) % 3;
  }
  get effectcount() {
    return 3;
  }
  get presetcount() {
    return 3;
  }
  next() {
    this._mode = (this._mode + 1) % 3;
  }
  previous() {
    this._mode = (this._mode + 2) % 3;
  }
  nextpreset() {
    this.next();
  }
  previouspreset() {
    this.previous();
  }
  nexteffect() {
    this.next();
  }
  previouseffect() {
    this.previous();
  }
  _coverage() {
    return "rect";
  }

  /** @param {{bands: number[], peaks: number[], wave: number[]} | null} f */
  _draw(f) {
    const c = this._canvas;
    const w = Math.max(1, Math.round(this.width));
    const h = Math.max(1, Math.round(this.height));
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    const ctx = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));
    ctx.clearRect(0, 0, w, h);
    if (!f) return;
    const level = cssColor(this._level) || "#0c6";
    const peak = cssColor(this._peak) || level;
    if (this._mode === 0) {
      const n = Math.max(4, Math.min(f.bands.length, Math.floor(w / 3)));
      const step = f.bands.length / n;
      const bw = w / n;
      for (let i = 0; i < n; i++) {
        const v = f.bands[Math.floor(i * step)] / 255;
        const p = f.peaks[Math.floor(i * step)] / 255;
        ctx.fillStyle = level;
        ctx.fillRect(i * bw, h - v * h, Math.max(1, bw - 1), v * h);
        ctx.fillStyle = peak;
        ctx.fillRect(i * bw, h - p * h - 1, Math.max(1, bw - 1), 1);
      }
    } else if (this._mode === 1) {
      ctx.strokeStyle = level;
      ctx.beginPath();
      f.wave.forEach((s, i) => {
        const x = (i / (f.wave.length - 1)) * w;
        const y = h / 2 - (s / 127) * (h / 2);
        if (i) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
      });
      ctx.stroke();
    } else {
      const avg = f.bands.reduce((a, b) => a + b, 0) / (f.bands.length * 255);
      ctx.fillStyle = level;
      ctx.fillRect(0, 0, w * Math.min(1, avg * 2), h);
    }
  }
}

// ---------------------------------------------------------------------------------
/** Video: there is no video here; the song cover is shown. */
export class Video extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._img = document.createElement("img");
    Object.assign(this._img.style, { width: "100%", height: "100%", objectFit: "contain", display: "none" });
    this._img.alt = "";
    this._dom.append(this._img);
    rt.covers.add(this);
  }
  /** @param {string | null} url */
  _cover(url) {
    if (url) this._img.src = url;
    this._img.style.display = url ? "" : "none";
  }
  get fullscreen() {
    return false;
  }
  set fullscreen(_v) {}
  _coverage() {
    return "rect";
  }
}

// ---------------------------------------------------------------------------------
/** PLAYLIST and similar: the app queue. */
export class Playlist extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._list = document.createElement("div");
    this._list.className = "wmp-list";
    this._dom.append(this._list);
    this._fg = "#ffffff";
    this._playing = "#ffff00";
    this._selected = -1;
    this._dropdown = node.tag === "dropdownplaylist";
    rt.lists.add(this);
  }
  get foregroundcolor() {
    return this._fg;
  }
  set foregroundcolor(v) {
    this._fg = String(v ?? "");
    this._render();
  }
  get itemplayingcolor() {
    return this._playing;
  }
  set itemplayingcolor(v) {
    this._playing = String(v ?? "");
    this._render();
  }
  get selectedindex() {
    return this._selected;
  }
  set selectedindex(v) {
    this._selected = num(v, -1);
    this._render();
  }
  get itemcount() {
    return this._rt.queue.items.length;
  }
  getitem(/** @type {number} */ i) {
    return this._rt.mediaAt(num(i));
  }
  setcolumnresizemode() {}
  getcolumnwidth() {
    return 0;
  }
  setcolumnwidth() {}
  isselected(/** @type {number} */ i) {
    return num(i) === this._selected;
  }
  setselectedstate(/** @type {number} */ i, /** @type {unknown} */ on) {
    if (bool(on)) this._selected = num(i);
    this._render();
  }
  _interactive() {
    return true;
  }
  _coverage() {
    return "rect";
  }
  _render() {
    const q = this._rt.queue;
    const fg = cssColor(this._fg) || "#fff";
    const playing = cssColor(this._playing) || fg;
    this._list.style.color = fg;
    if (this._dropdown) {
      this._list.textContent = q.name || queueName();
      return;
    }
    this._list.replaceChildren(
      ...q.items.map((it, i) => {
        const row = document.createElement("div");
        row.className = "wmp-row";
        if (i === q.current) row.style.color = playing;
        if (i === this._selected) row.classList.add("sel");
        const name = document.createElement("span");
        name.textContent = it.artist ? `${it.title} - ${it.artist}` : it.title;
        const dur = document.createElement("span");
        dur.textContent = it.durationMs ? timeString(it.durationMs / 1000) : "";
        row.append(name, dur);
        return row;
      }),
    );
    const cur = /** @type {HTMLElement | undefined} */ (this._list.children[q.current ?? -1]);
    cur?.scrollIntoView({ block: "nearest" });
  }
  /** @param {{x: number, y: number}} p */
  _rowAt(p) {
    const rows = [...this._list.children];
    const y = p.y + this._list.scrollTop;
    return rows.findIndex((r) => /** @type {HTMLElement} */ (r).offsetTop <= y && y < /** @type {HTMLElement} */ (r).offsetTop + /** @type {HTMLElement} */ (r).offsetHeight);
  }
  /** @param {string} type @param {{x: number, y: number}} p @param {PointerEvent | MouseEvent} e */
  _pointer(type, p, e) {
    if (this._dropdown) return super._pointer(type, p, e);
    if (type === "down") {
      this._selected = this._rowAt(p);
      this._render();
    } else if (type === "dblclick") {
      const i = this._rowAt(p);
      if (i >= 0) this._rt.send({ t: "cmd", c: "playIndex", v: i });
    }
    super._pointer(type, p, e);
  }
}

// ---------------------------------------------------------------------------------
/** EDITBOX: a simple text field. */
export class EditBox extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._input = document.createElement("input");
    Object.assign(this._input.style, { width: "100%", height: "100%", border: "0", padding: "0 2px", font: "inherit", background: "transparent", color: "inherit", outline: "none" });
    this._dom.append(this._input);
  }
  get value() {
    return this._input.value;
  }
  set value(v) {
    this._input.value = String(v ?? "");
  }
  get foregroundcolor() {
    return this._input.style.color;
  }
  set foregroundcolor(v) {
    this._dom.style.color = cssColor(v);
  }
  _interactive() {
    return true;
  }
  /** @param {string} type @param {{x: number, y: number}} p @param {PointerEvent | MouseEvent} e */
  _pointer(type, p, e) {
    if (type === "down") setTimeout(() => this._input.focus());
    super._pointer(type, p, e);
  }
  _coverage() {
    return "rect";
  }
}

// ---------------------------------------------------------------------------------
/** Elements with no drawing (equalizer, automatic menus…) and unknown ones. */
export class Stub extends El {
  /** @param {any} rt @param {WmsNode} node @param {El | null} parent */
  constructor(rt, node, parent) {
    super(rt, node, parent);
    this._dom.style.display = "none";
  }
  // AUTOMENU and POPUP: the app menu opens.
  show() {
    this._rt.send({ t: "win", c: "menu" });
    return 0;
  }
  // EQUALIZERSETTINGS: neutral values.
  get enabled() {
    return false;
  }
  set enabled(_v) {}
  gainlevel() {
    return 0;
  }
  reset() {}
  nextpreset() {}
  previouspreset() {}
  get currentpresettitle() {
    return "";
  }
  get presetcount() {
    return 0;
  }
  get visible() {
    return this._visible;
  }
  set visible(v) {
    this._visible = bool(v);
  }
  _coverage() {
    return null;
  }
  _target() {
    return null;
  }
}
