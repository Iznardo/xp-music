// Skin frame bootstrap. Runs isolated (sandboxed iframe without same-origin):
// it cannot see the app or its IPC; it only talks to the window via postMessage.

import { openZip } from "./zip.js";
import { Runtime } from "./runtime.js";

const host = window.parent;

// Extra defense: the frame has no origin, its CSP only lets it connect to
// skin.localhost and the app grants it no permissions; even so, WebView2 injects
// Tauri's IPC objects and its `chrome.webview` channel into every frame. They are
// nullified before any skin loads. `__TAURI_INTERNALS__` and `ipc` cannot be
// deleted, but without `chrome.webview` and with this CSP they reach nothing: the
// frame checks this on load and only warns if the IPC were to respond.
const exposed = ["__TAURI_INTERNALS__", "__TAURI__", "ipc", "__TAURI_IPC__", "__TAURI_POST_MESSAGE__"].filter((k) => k in window);
for (const k of exposed) {
  try {
    delete (/** @type {any} */ (window))[k];
    Object.defineProperty(window, k, { value: undefined, configurable: false, writable: false });
  } catch {
    /* not configurable: it stays, but without a channel */
  }
}
const chromeObj = /** @type {any} */ (window).chrome;
if (chromeObj?.webview) {
  try {
    Object.defineProperty(chromeObj, "webview", { value: undefined, configurable: false, writable: false });
  } catch {
    /* it stays, but the CSP does not let it out */
  }
}
/** @param {any} msg */
const send = (msg) => host.postMessage(msg, "*");

/** @type {Runtime | null} */
let rt = null;
const root = /** @type {HTMLElement} */ (document.getElementById("root"));

window.addEventListener("message", (e) => {
  if (e.source !== host) return;
  const m = e.data;
  if (!m || typeof m !== "object") return;
  if (m.t === "init") void init(m);
  else if (m.t === "state" && rt) rt.setState(m.s);
  else if (m.t === "queue" && rt) rt.setQueue(m.q);
  else if (m.t === "viz" && rt) {
    for (const fx of rt.effects) if (fx._visible) fx._draw(m.f);
  } else if (m.t === "stats" && rt) send({ t: "stats", ...stats(rt) });
});

window.addEventListener("error", (e) => send({ t: "log", level: "warn", msg: String(e.message).slice(0, 300) }));
window.addEventListener("unhandledrejection", (e) => send({ t: "log", level: "warn", msg: String(e.reason).slice(0, 300) }));

/** @param {{ url: string, prefs?: Record<string, string>, state?: any, queue?: any }} m */
async function init(m) {
  // Active check: the app IPC must not respond from here.
  fetch("http://ipc.localhost/plugin%3Aapp%7Cversion", { method: "POST", body: "{}" })
    .then((r) => send({ t: "log", level: "error", msg: `isolation: the IPC responded from the frame (${r.status})` }))
    .catch(() => {});
  // The other IPC path (window.ipc → chrome.webview) must not work either.
  try {
    /** @type {any} */ (window).ipc?.postMessage("{}");
    send({ t: "log", level: "error", msg: "isolation: window.ipc.postMessage did not fail" });
  } catch {
    /* expected */
  }
  try {
    const res = await fetch(m.url);
    if (!res.ok) throw new Error(`Could not read the skin (${res.status}).`);
    const zip = openZip(await res.arrayBuffer());
    rt = new Runtime(zip, send, { prefs: m.prefs });
    if (m.state) rt.setState(m.state);
    if (m.queue) rt.setQueue(m.queue);
    const view = await rt.load();
    root.append(view._dom);
    attach(rt);
    rt.start();
    const shape = rt.coverage();
    send({ t: "ready", w: Math.round(view.width), h: Math.round(view.height), title: rt.title, rects: shape ? shape.rects : null, effects: rt.effects.size > 0, ...stats(rt) });
  } catch (e) {
    send({ t: "error", msg: e instanceof Error ? e.message : String(e) });
  }
}

/** @param {Runtime} rt */
function stats(rt) {
  return {
    elements: rt.all.length,
    unsupported: [...rt.unsupported],
    missing: [...rt.images.missing].slice(0, 20),
    errors: rt.errors,
    stubbed: rt.stubbed.slice(0, 20),
    handlers: rt.handlerCount,
  };
}

/** Mouse and keyboard: the engine decides which element receives each thing. @param {Runtime} rt */
function attach(rt) {
  const view = /** @type {import("./elements.js").El} */ (rt.view);
  /** @type {import("./elements.js").El | null} */
  let hover = null;
  /** @type {import("./elements.js").El | null} */
  let captured = null;
  /** @param {import("./elements.js").El} el @param {MouseEvent} e */
  const local = (el, e) => {
    const p = rt.absPos(el);
    return { x: e.clientX - p.x, y: e.clientY - p.y };
  };
  /** @param {MouseEvent} e */
  const targetAt = (e) => view._target(e.clientX, e.clientY);
  /** @param {import("./elements.js").El | null} t @param {MouseEvent} e */
  const setHover = (t, e) => {
    if (t === hover) return;
    const prev = hover;
    hover = t;
    if (prev) prev._pointer("out", local(prev, e), e);
    if (t) t._pointer("over", local(t, e), e);
    const tip = t ? String(/** @type {any} */ (t)._proxy.tooltip ?? "") : "";
    root.title = tip && !/^res:/i.test(tip) ? tip : "";
    root.style.cursor = t && /^hand$/i.test(t._cursor || "hand") ? "pointer" : "default";
  };
  root.addEventListener("pointermove", (e) => {
    rt.domEvent = e;
    if (captured) {
      captured._pointer("move", local(captured, e), e);
      return;
    }
    const t = targetAt(e);
    setHover(t, e);
    t?._pointer("move", local(t, e), e);
  });
  root.addEventListener("pointerleave", (e) => {
    rt.domEvent = e;
    if (!captured) setHover(null, e);
  });
  root.addEventListener("pointerdown", (e) => {
    rt.domEvent = e;
    if (e.button !== 0) return;
    const t = targetAt(e);
    if (t) {
      captured = t;
      root.setPointerCapture(e.pointerId);
      t._pointer("down", local(t, e), e);
      return;
    }
    // Outside the controls: drag the window (if it is part of the skin).
    if (inside(rt, e.clientX, e.clientY)) send({ t: "win", c: "drag" });
  });
  root.addEventListener("pointerup", (e) => {
    rt.domEvent = e;
    if (!captured) return;
    const t = captured;
    captured = null;
    t._pointer("up", local(t, e), e);
    setHover(targetAt(e), e);
  });
  root.addEventListener("dblclick", (e) => {
    rt.domEvent = e;
    const t = targetAt(e);
    t?._pointer("dblclick", local(t, e), e);
  });
  root.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    send({ t: "win", c: "menu", x: e.clientX, y: e.clientY });
  });
  // App shortcuts (Ctrl+1 goes back to full mode, Ctrl+P…).
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    send({ t: "key", key: e.key, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey });
  });
}

/** Is the point a visible part of the skin? @param {Runtime} rt @param {number} x @param {number} y */
function inside(rt, x, y) {
  const c = rt.coverage();
  if (!c) return true;
  return c.rects.some(([rx, ry, rw, rh]) => x >= rx && x < rx + rw && y >= ry && y < ry + rh);
}
