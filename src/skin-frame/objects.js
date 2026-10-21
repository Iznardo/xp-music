// Modelo de objetos de WMP que usan los skins (player, theme, mediacenter…),
// conectado al reproductor de la app. Lo que no tiene sentido aquí (CD, DVD,
// abrir archivos…) existe pero no hace nada, para que los scripts no fallen.

import { wmpObject, bool, num, timeString } from "./util.js";

/** @typedef {{ title: string, artist: string, album: string, durationMs: number | null }} Item */

const PLAYSTATE = /** @type {Record<string, number>} */ ({ Idle: 1, Loading: 9, Buffering: 6, Playing: 3, Paused: 2, Error: 1 });
const STATUS = /** @type {Record<string, string>} */ ({
  Idle: "Detenido",
  Loading: "Conectando…",
  Buffering: "Almacenando en búfer…",
  Playing: "Reproduciendo",
  Paused: "En pausa",
  Error: "Error",
});

class Media {
  /** @param {Item | null} item */
  constructor(item) {
    this._item = item;
  }
  get name() {
    return this._item?.title ?? "";
  }
  set name(_v) {}
  get duration() {
    return (this._item?.durationMs ?? 0) / 1000;
  }
  get durationstring() {
    return this._item?.durationMs ? timeString(this._item.durationMs / 1000) : "";
  }
  get sourceurl() {
    return "";
  }
  get imagesourcewidth() {
    return 0;
  }
  get imagesourceheight() {
    return 0;
  }
  get attributecount() {
    return 4;
  }
  getattributename(/** @type {number} */ i) {
    return ["Title", "Author", "WM/AlbumTitle", "Duration"][num(i)] ?? "";
  }
  /** @param {string} attr */
  getiteminfo(attr) {
    const a = String(attr ?? "").toLowerCase();
    const it = this._item;
    if (!it) return "";
    if (a === "title" || a === "name") return it.title;
    if (a === "author" || a === "artist" || a === "wm/albumartist" || a === "displayartist") return it.artist;
    if (a === "wm/albumtitle" || a === "album") return it.album;
    if (a === "duration") return String(this.duration);
    return "";
  }
  /** Así lo llaman algunos skins. @param {string} attr */
  getinfo(attr) {
    return this.getiteminfo(attr);
  }
  getiteminfobyatom() {
    return "";
  }
  setiteminfo() {}
  isidentical(/** @type {any} */ m) {
    return !!m && m.name === this.name;
  }
  isreadonlyitem() {
    return true;
  }
  getmarkername() {
    return "";
  }
  get markercount() {
    return 0;
  }
}

class Controls {
  /** @param {any} rt */
  constructor(rt) {
    this._rt = rt;
  }
  play() {
    this._rt.command("play");
  }
  pause() {
    this._rt.command("pause");
  }
  stop() {
    this._rt.command("stop");
  }
  next() {
    this._rt.command("next");
  }
  previous() {
    this._rt.command("previous");
  }
  fastforward() {
    this.currentposition = this.currentposition + 10;
  }
  fastreverse() {
    this.currentposition = Math.max(0, this.currentposition - 10);
  }
  step() {}
  playitem(/** @type {any} */ m) {
    const items = this._rt.queue.items;
    const i = items.findIndex((/** @type {Item} */ it) => it.title === m?.name);
    if (i >= 0) this._rt.command("playIndex", i);
  }
  get currentposition() {
    return this._rt.state.positionMs / 1000;
  }
  set currentposition(v) {
    const s = num(v);
    // Las barras de posición lo reasignan sin parar: solo se salta si cambia de verdad.
    if (Math.abs(s - this.currentposition) < 0.75) return;
    this._rt.command("seek", Math.max(0, Math.round(s * 1000)));
  }
  get currentpositionstring() {
    return this._rt.state.hasTrack ? timeString(this.currentposition) : "";
  }
  get currentpositiontimecode() {
    return `[00000]${this.currentpositionstring}.00`;
  }
  get currentitem() {
    return this._rt.player.currentmedia;
  }
  set currentitem(m) {
    this.playitem(m);
  }
  get currentmarker() {
    return 0;
  }
  set currentmarker(_v) {}
  get audiolanguagecount() {
    return 0;
  }
  /** @param {string} what */
  isavailable(what) {
    const s = this._rt.state;
    const w = String(what ?? "").toLowerCase();
    const playing = s.status === "Playing" || s.status === "Buffering" || s.status === "Loading";
    if (w === "play") return s.hasTrack && !playing;
    if (w === "pause") return playing;
    if (w === "stop") return playing || s.status === "Paused";
    if (w === "next" || w === "previous") return s.hasTrack;
    if (w === "fastforward" || w === "fastreverse" || w === "currentposition") return s.hasTrack;
    return true;
  }
}

class Settings {
  /** @param {any} rt */
  constructor(rt) {
    this._rt = rt;
  }
  get volume() {
    return Math.round(this._rt.state.volume * 100);
  }
  set volume(v) {
    const n = Math.max(0, Math.min(100, num(v)));
    if (Math.round(n) === this.volume) return;
    this._rt.command("volume", n / 100);
  }
  get mute() {
    return this._rt.state.muted;
  }
  set mute(v) {
    const b = bool(v);
    if (b !== this.mute) this._rt.command("mute", b);
  }
  get balance() {
    return 0;
  }
  set balance(_v) {}
  get rate() {
    return 1;
  }
  set rate(_v) {}
  get autostart() {
    return true;
  }
  set autostart(_v) {}
  get playcount() {
    return 1;
  }
  set playcount(_v) {}
  get enableerrordialogs() {
    return false;
  }
  set enableerrordialogs(_v) {}
  get invokeurls() {
    return false;
  }
  set invokeurls(_v) {}
  /** @param {string} mode */
  getmode(mode) {
    const m = String(mode ?? "").toLowerCase();
    if (m === "shuffle") return this._rt.state.shuffle;
    if (m === "loop") return this._rt.state.repeat !== "Off";
    return false;
  }
  /** @param {string} mode @param {unknown} on */
  setmode(mode, on) {
    const m = String(mode ?? "").toLowerCase();
    if (m === "shuffle") this._rt.command("shuffle", bool(on));
    else if (m === "loop") this._rt.command("repeat", bool(on) ? "All" : "Off");
  }
  isavailable() {
    return true;
  }
}

class PlaylistObj {
  /** @param {any} rt */
  constructor(rt) {
    this._rt = rt;
  }
  get name() {
    return this._rt.queue.name || "Cola de reproducción";
  }
  set name(_v) {}
  get count() {
    return this._rt.queue.items.length;
  }
  item(/** @type {number} */ i) {
    return this._rt.mediaAt(num(i));
  }
  getiteminfo(/** @type {string} */ a) {
    return String(a ?? "").toLowerCase() === "name" ? this.name : "";
  }
  get attributecount() {
    return 0;
  }
  appenditem() {}
  insertitem() {}
  removeitem() {}
  clear() {}
  moveitem() {}
  isidentical() {
    return true;
  }
}

/** Algo que existe pero no hace nada (CD, DVD, subtítulos, colecciones…). */
function inert(extra = {}) {
  return wmpObject(Object.assign(new (class Inert {})(), { count: 0, item: () => null, isavailable: () => false, getall: () => wmpObject({ count: 0, item: () => null }), getbyname: () => wmpObject({ count: 0, item: () => null }) }, extra));
}

class Player {
  /** @param {any} rt */
  constructor(rt) {
    this._rt = rt;
    this.controls = wmpObject(new Controls(rt));
    this.settings = wmpObject(new Settings(rt));
    this.currentplaylist = wmpObject(new PlaylistObj(rt));
    this.network = inert({ bitrate: 128000, bandwidth: 128000, bufferingprogress: 100, downloadprogress: 100, receptionquality: 100, sourceprotocol: "https" });
    this.cdromcollection = inert();
    this.dvd = inert();
    this.closedcaption = inert({ captioningid: "", samifilename: "" });
    this.mediacollection = inert();
    this.playlistcollection = inert();
    this.error = inert({ errorcount: 0, clearerrorqueue() {}, webhelp() {} });
    /** @type {Media | null} */
    this._media = null;
  }
  get currentmedia() {
    const s = this._rt.state;
    if (!s.hasTrack) return wmpObject(new Media(null));
    if (!this._media || this._media._item?.title !== s.title || this._media._item?.durationMs !== s.durationMs) {
      this._media = new Media({ title: s.title, artist: s.artist, album: s.album, durationMs: s.durationMs });
    }
    return wmpObject(this._media);
  }
  set currentmedia(_v) {}
  get playstate() {
    return this._rt.state.hasTrack || this._rt.state.status !== "Idle" ? PLAYSTATE[this._rt.state.status] ?? 1 : 10;
  }
  get openstate() {
    return this._rt.state.hasTrack ? 13 : 0;
  }
  get status() {
    return STATUS[this._rt.state.status] ?? "";
  }
  get url() {
    return "";
  }
  set url(_v) {}
  get enabled() {
    return true;
  }
  set enabled(_v) {}
  get fullscreen() {
    return false;
  }
  set fullscreen(_v) {}
  get uimode() {
    return "full";
  }
  set uimode(_v) {}
  get versioninfo() {
    return "12.0.7601.0";
  }
  get isonline() {
    return true;
  }
  get stretchtofit() {
    return true;
  }
  set stretchtofit(_v) {}
  get windowlessvideo() {
    return true;
  }
  set windowlessvideo(_v) {}
  launchurl() {}
  newmedia() {
    return wmpObject(new Media(null));
  }
  newplaylist() {
    return wmpObject(new PlaylistObj(this._rt));
  }
  close() {}
  openplayer() {}
}

class Theme {
  /** @param {any} rt */
  constructor(rt) {
    this._rt = rt;
  }
  get title() {
    return this._rt.title;
  }
  get currentviewid() {
    return this._rt.view?._id ?? "";
  }
  /** @param {string} id */
  openview(id) {
    this._rt.log("info", `openView("${id}") no está soportado todavía`);
  }
  openviewrelative(/** @type {string} */ id) {
    this.openview(id);
  }
  /** @param {string} id */
  closeview(id) {
    if (String(id).toLowerCase() === this.currentviewid.toLowerCase()) this._rt.send({ t: "win", c: "close" });
  }
  /** @param {string} name @param {unknown} value */
  savepreference(name, value) {
    this._rt.savePref(String(name), String(value));
  }
  /** WMP devuelve "--" si no hay nada guardado. @param {string} name */
  loadpreference(name) {
    return this._rt.prefs[String(name).toLowerCase()] ?? "--";
  }
  loadstring() {
    return "";
  }
  opendialog() {
    return "";
  }
  playsound() {}
  /** @param {unknown} s */
  logstring(s) {
    this._rt.log("info", String(s));
  }
  showerrordialog() {}
  messagebox() {
    return 0;
  }
}

class MediaCenter {
  constructor() {
    this._v = /** @type {Record<string, unknown>} */ ({ effecttype: "", effectpreset: 0, videozoom: 100, showeffects: true });
  }
  get effecttype() {
    return this._v.effecttype;
  }
  set effecttype(v) {
    this._v.effecttype = v;
  }
  get effectpreset() {
    return this._v.effectpreset;
  }
  set effectpreset(v) {
    this._v.effectpreset = v;
  }
  get videozoom() {
    return this._v.videozoom;
  }
  set videozoom(v) {
    this._v.videozoom = v;
  }
  get showeffects() {
    return this._v.showeffects;
  }
  set showeffects(v) {
    this._v.showeffects = v;
  }
}

/** Métodos y propiedades de la vista (la ventana del skin). */
export const viewMethods = {
  close() {
    /** @type {any} */ (this)._rt.send({ t: "win", c: "close" });
  },
  minimize() {
    /** @type {any} */ (this)._rt.send({ t: "win", c: "minimize" });
  },
  maximize() {},
  restore() {},
  returntomediacenter() {
    /** @type {any} */ (this)._rt.send({ t: "win", c: "full" });
  },
  setmousecapture() {},
  releasemousecapture() {},
};

/** @param {any} rt */
export function createModel(rt) {
  return {
    player: wmpObject(new Player(rt)),
    theme: wmpObject(new Theme(rt)),
    // MEDIACENTER: valores que los skins leen y escriben (visualización elegida…).
    mediacenter: wmpObject(new MediaCenter()),
    media: (/** @type {Item | null} */ it) => wmpObject(new Media(it)),
  };
}
