// Motor de un skin: construye la vista desde el .wms, ejecuta sus scripts con el
// modelo de WMP, mantiene las expresiones wmpprop:/wmpenabled: al día, reparte los
// clics y calcula la forma de la ventana.

import { parseWms, findTag } from "./wms.js";
import { decodeText } from "./zip.js";
import { Images } from "./images.js";
import { wmpObject, stripJs, num } from "./util.js";
import { createModel, viewMethods } from "./objects.js";
import {
  El, Subview, Button, ButtonElement, ButtonGroup, Slider, CustomSlider, Text, TEXT_TAGS, Effects, Video, Playlist, EditBox, Stub,
} from "./elements.js";

/** @typedef {import("./wms.js").WmsNode} WmsNode */

class View extends Subview {
  /** @param {any} rt @param {WmsNode} node */
  constructor(rt, node) {
    super(rt, node, null);
    this._timer = 0;
    this._interval = 0;
    this._timerOn = true;
    this._title = "";
    Object.assign(this, viewMethods);
  }
  get title() {
    return this._title;
  }
  set title(v) {
    this._title = String(v ?? "");
  }
  get timerinterval() {
    return this._interval;
  }
  set timerinterval(v) {
    this._interval = num(v);
    this._restartTimer();
  }
  get timerenabled() {
    return this._timerOn;
  }
  set timerenabled(v) {
    this._timerOn = !/^(false|0)$/i.test(String(v));
    this._restartTimer();
  }
  _restartTimer() {
    clearInterval(this._timer);
    this._timer = 0;
    if (!this._timerOn || this._interval <= 0 || !this._rt.started) return;
    // Hay skins con timerInterval="10": más de ~30 veces por segundo no aporta nada.
    this._timer = window.setInterval(() => this._rt.fire(this, "timer"), Math.max(33, this._interval));
  }
  _naturalW() {
    return this._mask?.w ?? 0;
  }
  _naturalH() {
    return this._mask?.h ?? 0;
  }
}

/** Etiqueta → clase del elemento. */
function classFor(/** @type {string} */ tag) {
  if (tag === "subview") return Subview;
  if (tag === "buttongroup") return ButtonGroup;
  if (tag === "button" || tag === "imagebutton" || /^\w+button$/.test(tag)) return Button;
  if (/^(slider|seekslider|volumeslider|balanceslider|progressbar)$/.test(tag)) return Slider;
  if (tag === "customslider") return CustomSlider;
  if (tag in TEXT_TAGS) return Text;
  if (tag === "effects" || tag === "wmpeffects") return Effects;
  if (tag === "video" || tag === "wmpvideo") return Video;
  if (/^(playlist|itemsplaylist|dropdownplaylist|listbox|currentplaylist)$/.test(tag)) return Playlist;
  if (tag === "editbox") return EditBox;
  return null;
}

/** Elementos sin dibujo que sí son conocidos (no cuentan como no soportados). */
const KNOWN_STUBS = new Set(["equalizersettings", "videosettings", "automenu", "popup", "skinstudio", "copyright", "unsupported", "bars", "visualization", "alchemy", "column", "item"]);
/** Objetos del reproductor cuyos eventos se declaran como elementos. */
const HOLDERS = new Set(["player", "controls", "settings", "currentmedia", "network", "mediacenter", "currentplaylist"]);
/** Nombres globales que el propio motor necesita: un skin no puede taparlos. */
const RESERVED = new Set(
  ("Math Object Array String Number Boolean Date JSON Proxy Promise Map Set WeakMap Symbol Error TypeError RegExp URL Blob Response " +
    "Uint8Array Uint8ClampedArray DataView ArrayBuffer TextDecoder DecompressionStream OffscreenCanvas ImageData createImageBitmap " +
    "document window self globalThis parent top location frames screen setTimeout clearTimeout setInterval clearInterval " +
    "requestAnimationFrame cancelAnimationFrame performance console Function eval parseInt parseFloat isNaN isFinite Reflect " +
    "queueMicrotask structuredClone addEventListener removeEventListener postMessage devicePixelRatio getComputedStyle Intl " +
    "encodeURIComponent decodeURIComponent escape unescape Infinity NaN undefined").split(" "),
);
const KEYWORDS = new Set(
  ("if for while switch return function typeof new catch with var let const else do try delete in instanceof void case " +
    "break continue default throw finally this null true false class super import export yield await debugger").split(" "),
);

export class Runtime {
  /**
   * @param {ReturnType<typeof import("./zip.js").openZip>} zip
   * @param {(msg: any) => void} send
   * @param {{ prefs?: Record<string, string> }} [opts]
   */
  constructor(zip, send, opts = {}) {
    this.zip = zip;
    this.send = send;
    this.images = new Images(zip);
    this.prefs = /** @type {Record<string, string>} */ ({});
    for (const [k, v] of Object.entries(opts.prefs ?? {})) this.prefs[k.toLowerCase()] = v;
    this.state = { status: "Idle", title: "", artist: "", album: "", durationMs: 0, positionMs: 0, volume: 0.8, muted: false, shuffle: false, repeat: "Off", cover: /** @type {string | null} */ (null), hasTrack: false };
    this.queue = { items: /** @type {import("./objects.js").Item[]} */ ([]), current: /** @type {number | null} */ (null), name: "" };
    this.model = createModel(this);
    this.player = this.model.player;
    this.title = "";
    /** @type {View | null} */
    this.view = null;
    /** @type {Map<string, El>} id en minúsculas → elemento */
    this.ids = new Map();
    /** @type {El[]} */
    this.all = [];
    /** @type {{ el: El, prop: string, fn: () => unknown, last: unknown }[]} */
    this.bindings = [];
    /** @type {Map<string, Map<string, string[]>>} objeto → evento → código */
    this.listeners = new Map();
    /** @type {string[]} */
    this.scripts = [];
    /** Código de manejadores y expresiones (para los nombres globales). */
    this.code = /** @type {string[]} */ ([]);
    /** @type {(() => void)[]} */
    this.deferred = [];
    this.effects = new Set();
    this.covers = new Set();
    this.lists = new Set();
    this.unsupported = new Set();
    this.errors = 0;
    this.fromBinding = false;
    this.started = false;
    /** Hasta que no hay nombres globales ni scripts, no se ejecuta código del skin. */
    this.ready = false;
    /** @type {any} */
    this.event = null;
    /** Evento del navegador que se está atendiendo. @type {Event | null} */
    this.domEvent = null;
    this.compiled = new Map();
    this.anims = new Map();
    this.stubbed = /** @type {string[]} */ ([]);
    /** @type {Set<string>} */
    this.baseline = new Set();
    /** Propiedades del modelo: en JScript `player.settings.mute()` lee la propiedad. */
    this.props = getterNames([
      ...["player", "controls", "settings", "currentplaylist", "currentmedia"].map((k) => /** @type {any} */ (k === "player" ? this.model.player : this.model.player[k]).__target),
      /** @type {any} */ (this.model.mediacenter).__target,
      ...[El, Subview, Button, ButtonGroup, Slider, CustomSlider, Text, Effects, Video, Playlist, EditBox].map((c) => c.prototype),
    ]);
    this.handlerCount = 0;
  }

  // --- Carga -----------------------------------------------------------------------
  async load() {
    const wmsName = this.zip.names().find((n) => /\.wms$/i.test(n));
    if (!wmsName) throw new Error("El skin no tiene archivo .wms.");
    const root = parseWms(decodeText(await /** @type {Promise<Uint8Array>} */ (this.zip.read(wmsName))));
    const theme = findTag(root, "theme") ?? root;
    this.title = theme.attrs.title ?? "";
    for (const s of theme.children.filter((c) => c.tag === "script")) this.scripts.push(s.text);
    const viewNode = findTag(theme, "view");
    if (!viewNode) throw new Error("El skin no tiene ninguna vista.");

    const view = new View(this, viewNode);
    this.view = view;
    Object.assign(view._dom.style, { position: "absolute", left: "0", top: "0", overflow: "hidden" });
    this.#setup(view, viewNode);
    for (const file of (viewNode.attrs.scriptfile ?? "").split(";")) {
      const name = file.trim();
      if (!name || /^res:/i.test(name)) continue;
      const bytes = this.zip.read(name);
      if (bytes) this.scripts.push(decodeText(await bytes));
      else this.log("warn", `Falta el script ${name}`);
    }
    this.#build(viewNode, view);

    // Las imágenes cargan en paralelo; se espera a todas para medir la vista.
    await this.#settle();
    this.#otherViews(theme, viewNode);
    /** Lo que ya había en `window` antes del skin (el navegador). */
    this.baseline = new Set(Object.getOwnPropertyNames(window));
    this.#globals();
    for (const code of this.scripts) {
      try {
        (0, eval)(this.fix(code));
      } catch (e) {
        this.#error("script", e);
      }
    }
    this.#stubs();
    this.ready = true;
    for (const f of this.deferred) f();
    this.refresh();
    return view;
  }

  /** Arranca eventos y temporizadores (con la vista ya en pantalla). */
  start() {
    this.started = true;
    /** @type {View} */ (this.view)._restartTimer();
    this.fire(/** @type {View} */ (this.view), "load");
    this.refresh();
    this.shapeDirty();
  }

  async #settle() {
    for (let i = 0; i < 3; i++) {
      await Promise.all([...this.images.cache.values()]);
      await new Promise((r) => setTimeout(r));
    }
  }

  /** @param {WmsNode} node @param {El} parent */
  #build(node, parent) {
    for (const child of node.children) {
      const tag = child.tag;
      if (tag === "script") {
        this.scripts.push(child.text);
        continue;
      }
      if (HOLDERS.has(tag)) {
        this.#holder(child, tag);
        continue;
      }
      if (tag === "view" || tag === "theme") continue;
      if (parent instanceof ButtonGroup) {
        if (tag === "buttonelement" || /element$/.test(tag)) {
          const el = new ButtonElement(this, child, parent);
          parent._elements.push(el);
          parent._dom.append(el._dom);
          this.#setup(el, child);
          if (parent._map) el._build(parent._map);
        }
        continue;
      }
      const Cls = classFor(tag);
      if (!Cls && !KNOWN_STUBS.has(tag)) this.unsupported.add(tag);
      const el = new (Cls ?? Stub)(this, child, parent);
      el._dom.style.position = "absolute";
      parent._children.push(el);
      parent._dom.append(el._dom);
      this.#setup(el, child);
      if (Cls === Subview || Cls === ButtonGroup) this.#build(child, el);
      if (child.tag in TEXT_TAGS && TEXT_TAGS[/** @type {keyof typeof TEXT_TAGS} */ (child.tag)] && !("value" in child.attrs)) {
        this.#attr(el, "value", /** @type {string} */ (TEXT_TAGS[/** @type {keyof typeof TEXT_TAGS} */ (child.tag)]));
      }
    }
  }

  /** Atributos de un elemento: valores, enlaces y manejadores. @param {El} el @param {WmsNode} node */
  #setup(el, node) {
    this.all.push(el);
    const attrs = { ...node.attrs };
    // Valores por defecto de los botones y deslizadores con nombre.
    const tag = node.tag;
    const named = /** @type {any} */ (el)._named;
    if (named?.defaults) for (const [k, v] of Object.entries(named.defaults)) if (!(k in attrs)) attrs[k] = v;
    if (tag === "seekslider") {
      attrs.min ??= "0";
      attrs.max ??= "wmpprop:player.currentMedia.duration";
      attrs.value ??= "wmpprop:player.controls.currentPosition";
      attrs.value_onchange ??= "player.controls.currentPosition = value";
      attrs.slide ??= "false";
    } else if (tag === "volumeslider") {
      attrs.min ??= "0";
      attrs.max ??= "100";
      attrs.value ??= "wmpprop:player.settings.volume";
      attrs.value_onchange ??= "player.settings.volume = value";
    }
    if (attrs.id) {
      el._id = attrs.id;
      this.ids.set(attrs.id.toLowerCase(), el);
    }
    // Primero lo que define el tamaño y el color clave, después las imágenes.
    const order = Object.keys(attrs).sort((a, b) => rank(a) - rank(b));
    for (const k of order) this.#attr(el, k, attrs[k]);
  }

  /** @param {El} el @param {string} k @param {string} v */
  #attr(el, k, v) {
    if (k === "id" || k === "scriptfile") return;
    if (/^on/.test(k) || /_on/.test(k)) {
      el._handlers[eventKey(k)] = stripJs(v);
      this.code.push(v);
      this.handlerCount++;
      return;
    }
    const p = el._proxy;
    // "jscript:wmpenabled:…" también aparece: manda el último prefijo.
    const m = /^\s*(?:jscript\s*:\s*)?(wmpprop|wmpenabled|wmpdisabled|jscript)\s*:(.*)$/is.exec(v);
    if (!m) {
      try {
        p[k] = v;
      } catch (e) {
        this.#error(`${el._tag}.${k}`, e);
      }
      return;
    }
    const kind = m[1].toLowerCase();
    const expr = m[2].trim().replace(/;+\s*$/, "");
    this.code.push(expr);
    if (kind === "jscript") {
      this.deferred.push(() => {
        try {
          p[k] = this.#compile(expr, true).call(p, this.scope(el));
        } catch (e) {
          this.#error(`${el._tag}.${k}`, e);
        }
      });
      return;
    }
    let fn;
    if (kind === "wmpprop") {
      const f = this.#compile(expr, true);
      fn = () => f.call(p, this.scope(el));
    } else {
      // wmpenabled:player.controls.play → player.controls.isAvailable("play")
      const dot = expr.lastIndexOf(".");
      const obj = this.#compile(dot > 0 ? expr.slice(0, dot) : "player.controls", true);
      const what = expr.slice(dot + 1);
      const neg = kind === "wmpdisabled";
      fn = () => {
        const o = /** @type {any} */ (obj.call(p, this.scope(el)));
        const ok = o && typeof o.isavailable === "function" ? !!o.isavailable(what) : true;
        return neg ? !ok : ok;
      };
    }
    this.bindings.push({ el, prop: k, fn, last: Symbol("nada") });
  }

  /** <player playStateChange="…">, <controls currentPosition_onchange="…">… @param {WmsNode} node @param {string} obj */
  #holder(node, obj) {
    let map = this.listeners.get(obj);
    if (!map) this.listeners.set(obj, (map = new Map()));
    for (const [k, v] of Object.entries(node.attrs)) {
      if (k === "id") continue;
      if (!/change|^on|_on/.test(k)) continue;
      const key = eventKey(k);
      map.set(key, [...(map.get(key) ?? []), stripJs(v)]);
      this.code.push(v);
      this.handlerCount++;
    }
    for (const c of node.children) if (HOLDERS.has(c.tag)) this.#holder(c, c.tag);
  }

  /** Ids de las otras vistas: existen (los scripts los tocan) pero no se dibujan. @param {WmsNode} theme @param {WmsNode} current */
  #otherViews(theme, current) {
    /** @param {WmsNode} n */
    const walk = (n) => {
      const id = n.attrs.id;
      if (id && !this.ids.has(id.toLowerCase())) {
        const el = new Stub(this, n, null);
        el._id = id;
        this.ids.set(id.toLowerCase(), el);
      }
      n.children.forEach(walk);
    };
    for (const v of theme.children) if (v.tag === "view" && v !== current) walk(v);
  }

  // --- Nombres globales y scripts ---------------------------------------------------
  /** Los ids y los objetos de WMP, con la grafía que use cada skin. */
  #globals() {
    // Constantes de WMP que los skins usan sin definir (playState y openState).
    const consts = /** @type {Record<string, number>} */ ({});
    "Undefined Stopped Paused Playing ScanForward ScanReverse Buffering Waiting MediaEnded Transitioning Ready Reconnecting Last"
      .split(" ")
      .forEach((n, i) => (consts[`ps${n}`] = i));
    ("Undefined PlaylistChanging PlaylistLocating PlaylistConnecting PlaylistLoading PlaylistOpening PlaylistOpenNoMedia " +
      "PlaylistChanged MediaChanging MediaLocating MediaConnecting MediaLoading MediaOpening MediaOpen BeginCodecAcquisition " +
      "EndCodecAcquisition BeginLicenseAcquisition EndLicenseAcquisition BeginIndividualization EndIndividualization " +
      "MediaWaiting OpeningUnknownURL")
      .split(" ")
      .forEach((n, i) => (consts[`os${n}`] = i));
    for (const [k, v] of Object.entries(consts)) if (!(k in window)) /** @type {any} */ (window)[k] = v;

    /** @type {Map<string, () => unknown>} */
    const names = new Map();
    for (const [id, el] of this.ids) names.set(id, () => el._proxy);
    // Los objetos de WMP mandan sobre un elemento que se llame igual (id="player").
    const m = this.model;
    names.set("player", () => m.player);
    names.set("theme", () => m.theme);
    names.set("mediacenter", () => m.mediacenter);
    names.set("view", () => this.view?._proxy);
    // Fuera de un evento, `event` existe igualmente (hay skins que leen event.screenWidth al cargar).
    const idle = wmpObject(Object.assign(new (class Event {})(), { x: 0, y: 0, offsetx: 0, offsety: 0, button: 0, keycode: 0, screenwidth: screen.width, screenheight: screen.height, returnvalue: true }));
    names.set("event", () => this.event ?? idle);
    const tokens = new Set();
    for (const src of [...this.scripts, ...this.code]) for (const t of src.match(/[A-Za-z_$][\w$]*/g) ?? []) tokens.add(t);
    for (const [, el] of this.ids) tokens.add(el._id);
    for (const t of ["player", "theme", "mediacenter", "view", "event"]) tokens.add(t);
    const wmp = new Set(["player", "theme", "mediacenter", "view", "event"]);
    for (const t of tokens) {
      const lt = t.toLowerCase();
      const get = names.get(lt);
      if (!get || RESERVED.has(t)) continue;
      const exact = wmp.has(lt) || [...this.ids.values()].some((e) => e._id === t);
      // Un nombre del navegador (Date, Image…) solo se tapa si el skin lo usa tal cual.
      if (t in window && !exact) continue;
      try {
        Object.defineProperty(window, t, {
          configurable: true,
          get,
          set(v) {
            Object.defineProperty(window, t, { configurable: true, writable: true, value: v });
          },
        });
      } catch {
        /* propiedad no configurable del navegador (top, location…) */
      }
    }
  }

  /** Funciones que llaman los skins pero no definen (venían de la librería de WMP). */
  #stubs() {
    const calls = new Set();
    for (const src of [...this.scripts, ...this.code]) {
      for (const m of src.matchAll(/(?<![\w$.])([A-Za-z_$][\w$]*)\s*\(/g)) calls.add(m[1]);
    }
    for (const name of calls) {
      if (KEYWORDS.has(name) || name in window) continue;
      /** @type {any} */ (window)[name] = () => undefined;
      this.stubbed.push(name);
    }
  }

  /** `x.mute()` → `x.mute` cuando `mute` es una propiedad (COM lo permite). @param {string} code */
  fix(code) {
    return code.replace(/\.([A-Za-z_]\w*)\s*\(\s*\)/g, (m, n) => (this.props.has(n.toLowerCase()) ? `.${n}` : m));
  }

  /** @param {string} code @param {boolean} expr */
  #compile(code, expr) {
    const key = `${expr ? "e" : "s"}:${code}`;
    let f = this.compiled.get(key);
    if (!f) {
      const body = this.fix(code);
      try {
        f = new Function("__ctx", expr ? `with (__ctx) { return (${body}); }` : `with (__ctx) {\n${body}\n}`);
      } catch (err) {
        this.#error("sintaxis", err);
        f = () => undefined;
      }
      this.compiled.set(key, f);
    }
    return f;
  }

  /**
   * Ámbito de un manejador o expresión: como en WMP, los nombres sueltos son
   * propiedades y métodos del propio elemento (`moveTo(left, top, 500)`), salvo
   * que el skin defina algo global con ese nombre. Lo del navegador (top, name…)
   * no cuenta: ahí gana el elemento.
   * @param {El | null} el @param {Record<string, unknown>} [extra]
   */
  scope(el, extra) {
    const local = extra ? new Map(Object.entries(extra).map(([k, v]) => [k.toLowerCase(), v])) : null;
    if (!local && el?._scope) return el._scope;
    const own = (/** @type {string} */ k) => {
      const lk = k.toLowerCase();
      if (!el || lk[0] === "_" || lk in Object.prototype || !(lk in el)) return false;
      return !(k in window && !this.baseline.has(k));
    };
    const ctx = new Proxy(Object.create(null), {
      has: (_, k) => typeof k === "string" && (!!local?.has(k.toLowerCase()) || own(k)),
      get: (_, k) => {
        if (typeof k !== "string") return undefined;
        const lk = k.toLowerCase();
        if (local?.has(lk)) return local.get(lk);
        return el?._proxy[lk];
      },
      set: (_, k, v) => {
        if (typeof k !== "string") return false;
        if (el) el._proxy[k.toLowerCase()] = v;
        return true;
      },
    });
    if (!local && el) el._scope = ctx;
    return ctx;
  }

  /** Ejecuta código de un manejador. */
  run(/** @type {string} */ code, /** @type {El | null} */ el, /** @type {Event | null} */ e = null, /** @type {Record<string, unknown>} */ extra = {}) {
    const f = this.#compile(code, false);
    const prev = this.event;
    const src = e ?? this.domEvent;
    if (src) this.event = this.#eventObject(src, el);
    try {
      f.call(el?._proxy ?? null, this.scope(el, Object.keys(extra).length ? extra : undefined));
    } catch (err) {
      this.#error(el ? `${el._tag}${el._id ? `#${el._id}` : ""}` : "evento", err);
    } finally {
      this.event = prev;
    }
    this.refreshSoon();
  }

  /** @param {El} el @param {string} key @param {Record<string, unknown>} [extra] */
  fire(el, key, extra) {
    if (!this.ready) return;
    const code = el._handlers[key];
    if (code) this.run(code, el, null, extra);
  }

  /** Eventos de player/controls/settings… @param {string} obj @param {string} key @param {Record<string, unknown>} [extra] */
  emit(obj, key, extra) {
    const map = this.listeners.get(obj);
    if (!map) return;
    // playStateChange y PlayState_onchange son el mismo aviso.
    const alias = key.endsWith("change") && !key.includes("_") ? `${key.slice(0, -6)}_onchange` : null;
    for (const k of alias ? [key, alias] : [key]) for (const code of map.get(k) ?? []) this.run(code, null, null, extra);
  }

  /** @param {Event} e @param {El | null} el */
  #eventObject(e, el) {
    const me = /** @type {MouseEvent & KeyboardEvent} */ (e);
    const pos = el ? this.absPos(el) : { x: 0, y: 0 };
    return wmpObject(
      Object.assign(new (class Event {})(), {
        x: me.clientX ?? 0,
        y: me.clientY ?? 0,
        offsetx: (me.clientX ?? 0) - pos.x,
        offsety: (me.clientY ?? 0) - pos.y,
        screenx: me.screenX ?? 0,
        screeny: me.screenY ?? 0,
        button: (me.button ?? 0) + 1,
        keycode: me.keyCode ?? 0,
        shiftkey: !!me.shiftKey,
        ctrlkey: !!me.ctrlKey,
        altkey: !!me.altKey,
        returnvalue: true,
        cancelbubble: false,
        screenwidth: screen.width,
        screenheight: screen.height,
      }),
    );
  }

  // --- Enlaces wmpprop: -------------------------------------------------------------
  refreshSoon() {
    if (this._refreshQueued) return;
    this._refreshQueued = true;
    queueMicrotask(() => {
      this._refreshQueued = false;
      this.refresh();
    });
  }

  refresh() {
    if (!this.ready) return;
    this.fromBinding = true;
    try {
      for (const b of this.bindings) {
        // No se mueve el control mientras el usuario lo arrastra.
        if (b.prop === "value" && /** @type {any} */ (b.el)._dragging) continue;
        let v;
        try {
          v = b.fn();
        } catch (e) {
          if (b.last !== "error") this.#error(`${b.el._tag}.${b.prop}`, e);
          b.last = "error";
          continue;
        }
        if (v === b.last) continue;
        b.last = v;
        try {
          /** @type {any} */ (b.el._proxy)[b.prop] = v;
        } catch (e) {
          this.#error(`${b.el._tag}.${b.prop}`, e);
        }
      }
    } finally {
      this.fromBinding = false;
    }
  }

  // --- Estado de la app ---------------------------------------------------------------
  /** @param {Partial<Runtime["state"]>} next */
  setState(next) {
    const before = { ...this.state, playstate: this.player.playstate, status: this.player.status, pos: this.player.controls.currentpositionstring };
    Object.assign(this.state, next);
    const p = this.player;
    if (!this.started) return;
    if (before.title !== this.state.title || before.durationMs !== this.state.durationMs || before.hasTrack !== this.state.hasTrack) {
      this.emit("player", "mediachange");
      this.emit("player", "currentitemchange");
      this.emit("controls", "currentitem_onchange");
      this.emit("player", "openstatechange", { newstate: p.openstate });
    }
    if (before.playstate !== p.playstate) this.emit("player", "playstatechange", { newstate: p.playstate });
    if (before.status !== p.status) this.emit("player", "statuschange");
    if (before.positionMs !== this.state.positionMs) {
      this.emit("controls", "currentposition_onchange");
      this.emit("player", "positionchange");
      if (before.pos !== p.controls.currentpositionstring) this.emit("controls", "currentpositionstring_onchange");
    }
    if (before.volume !== this.state.volume) this.emit("settings", "volume_onchange");
    if (before.muted !== this.state.muted) this.emit("settings", "mute_onchange");
    if (before.shuffle !== this.state.shuffle || before.repeat !== this.state.repeat) this.emit("player", "modechange");
    if (before.cover !== this.state.cover) for (const v of this.covers) v._cover(this.state.cover);
    this.refresh();
  }

  /** @param {Runtime["queue"]} q */
  setQueue(q) {
    this.queue = q;
    for (const l of this.lists) l._render();
    if (this.started) this.emit("player", "currentplaylistchange");
    this.refresh();
  }

  /** @param {number} i */
  mediaAt(i) {
    return this.model.media(this.queue.items[i] ?? null);
  }

  /** Orden de la app con efecto inmediato aquí (los controles no esperan al eco). */
  command(/** @type {string} */ c, /** @type {unknown} */ v) {
    const s = this.state;
    if (c === "volume") s.volume = /** @type {number} */ (v);
    else if (c === "mute") s.muted = /** @type {boolean} */ (v);
    else if (c === "shuffle") s.shuffle = /** @type {boolean} */ (v);
    else if (c === "repeat") s.repeat = /** @type {string} */ (v);
    else if (c === "seek") s.positionMs = /** @type {number} */ (v);
    this.send({ t: "cmd", c, v });
    this.refreshSoon();
  }

  /** @param {string} key @param {string} value */
  savePref(key, value) {
    this.prefs[key.toLowerCase()] = value;
    this.send({ t: "pref", k: key.toLowerCase(), v: value });
  }

  // --- Animaciones ---------------------------------------------------------------------
  /** @param {El} el @param {Record<string, number>} to @param {number} ms @param {string} endEvent */
  animate(el, to, ms, endEvent) {
    const key = `${this.all.indexOf(el)}:${Object.keys(to).join(",")}`;
    cancelAnimationFrame(this.anims.get(key) ?? 0);
    const p = /** @type {any} */ (el);
    const from = Object.fromEntries(Object.keys(to).map((k) => [k, num(p[k])]));
    const done = () => {
      for (const [k, v] of Object.entries(to)) p[k] = v;
      this.anims.delete(key);
      this.fire(el, endEvent);
    };
    if (ms <= 0) return done();
    const t0 = performance.now();
    const step = (/** @type {number} */ now) => {
      const f = Math.min(1, (now - t0) / ms);
      if (f >= 1) return done();
      for (const [k, v] of Object.entries(to)) p[k] = from[k] + (v - from[k]) * f;
      this.anims.set(key, requestAnimationFrame(step));
    };
    this.anims.set(key, requestAnimationFrame(step));
  }

  // --- Ratón y forma ---------------------------------------------------------------------
  /** @param {El[]} els */
  paintOrder(els) {
    return els.map((e, i) => ({ e, i })).sort((a, b) => a.e._z - b.e._z || a.i - b.i).map((x) => x.e);
  }

  /** Posición del elemento en la vista. @param {El} el */
  absPos(el) {
    let x = 0;
    let y = 0;
    for (let e = /** @type {El | null} */ (el); e && e !== this.view; e = e._parent) {
      if (e instanceof ButtonElement) {
        x += e._box.x;
        y += e._box.y;
      } else {
        x += e._left;
        y += e._top;
      }
    }
    return { x, y };
  }

  shapeDirty() {
    if (!this.started) return;
    clearTimeout(this._shapeTimer);
    this._shapeTimer = setTimeout(() => this.#sendShape(), 80);
  }

  /** Rectángulos (en píxeles CSS) de lo que pinta la vista. */
  coverage() {
    const view = /** @type {View} */ (this.view);
    const W = Math.max(1, Math.min(4096, Math.round(view.width) || 1));
    const H = Math.max(1, Math.min(4096, Math.round(view.height) || 1));
    const mask = new Uint8Array(W * H);
    /** @param {El} el @param {number} ox @param {number} oy */
    const paint = (el, ox, oy) => {
      if (!el._visible) return;
      const x0 = Math.round(ox);
      const y0 = Math.round(oy);
      const cov = el._coverage();
      const w = Math.round(el.width);
      const h = Math.round(el.height);
      if (cov === "rect") {
        for (let y = Math.max(0, y0); y < Math.min(H, y0 + h); y++) mask.fill(1, y * W + Math.max(0, x0), y * W + Math.min(W, x0 + w));
      } else if (cov) {
        const d = cov.data.data;
        const cw = Math.min(cov.w, w || cov.w);
        const ch = Math.min(cov.h, h || cov.h);
        for (let y = 0; y < ch; y++) {
          const vy = y0 + y;
          if (vy < 0 || vy >= H) continue;
          for (let x = 0; x < cw; x++) {
            const vx = x0 + x;
            if (vx >= 0 && vx < W && d[(y * cov.w + x) * 4 + 3] > 0) mask[vy * W + vx] = 1;
          }
        }
      }
      for (const c of el._children) paint(c, ox + c._left, oy + c._top);
    };
    paint(view, 0, 0);
    let covered = 0;
    for (let i = 0; i < mask.length; i++) covered += mask[i];
    // Sin nada que dibuje forma (o casi), la ventana es el rectángulo entero.
    if (covered === W * H || covered < (W * H) / 100) return null;
    /** @type {[number, number, number, number][]} */
    const rects = [];
    let prevKey = "";
    /** @type {{ x: number, w: number, y: number }[]} */
    let open = [];
    for (let y = 0; y <= H; y++) {
      /** @type {[number, number][]} */
      const runs = [];
      if (y < H) {
        for (let x = 0; x < W; ) {
          if (!mask[y * W + x]) {
            x++;
            continue;
          }
          const s = x;
          while (x < W && mask[y * W + x]) x++;
          runs.push([s, x - s]);
        }
      }
      const key = runs.join(";");
      if (key === prevKey) continue;
      for (const o of open) rects.push([o.x, o.y, o.w, y - o.y]);
      open = runs.map(([x, w]) => ({ x, w, y }));
      prevKey = key;
    }
    return { rects, covered, total: W * H };
  }

  #lastShape = "";
  #sendShape() {
    const c = this.coverage();
    const msg = c ? c.rects : null;
    const key = JSON.stringify(msg);
    if (key === this.#lastShape) return;
    this.#lastShape = key;
    this.send({ t: "shape", rects: msg });
  }

  // --- Errores ------------------------------------------------------------------------------
  /** @param {string} where @param {unknown} e */
  #error(where, e) {
    this.errors++;
    if (this.errors <= 20) this.log("warn", `${where}: ${e instanceof Error ? e.message : String(e)}`);
  }

  /** @param {string} level @param {string} msg */
  log(level, msg) {
    this.send({ t: "log", level, msg: msg.slice(0, 300) });
  }
}

/** Orden en que se aplican los atributos. @param {string} k */
function rank(k) {
  if (k === "transparencycolor" || k === "clippingcolor") return 0;
  if (k === "width" || k === "height" || k === "min" || k === "max" || k === "direction" || k === "bordersize") return 1;
  if (k === "value") return 3;
  return 2;
}

/** "onClick" → "click"; "value_onchange" se queda igual. @param {string} k */
function eventKey(k) {
  return /^on[a-z]/.test(k) && !k.includes("_") ? k.slice(2) : k;
}

/** Nombres de las propiedades (getters) de unos objetos y sus prototipos. @param {object[]} objs */
function getterNames(objs) {
  const out = new Set();
  for (let o of objs) {
    for (; o && o !== Object.prototype; o = Object.getPrototypeOf(o)) {
      for (const [k, d] of Object.entries(Object.getOwnPropertyDescriptors(o))) if (d.get) out.add(k);
    }
  }
  return out;
}
