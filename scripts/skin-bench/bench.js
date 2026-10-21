// Banco de pruebas de skins (solo vista previa: `XP_SKINS_DIR=… npm run dev:mock`
// y abrir /skin/bench/bench.html). Carga cada .wmz en el marco real y cuenta
// cuántos arrancan, con qué errores y qué les falta. Con ?skin=<id> muestra uno.

const out = document.getElementById("out");
const stage = document.getElementById("stage");
const params = new URLSearchParams(location.search);

const STATE = {
  status: "Playing", title: "Canción de prueba", artist: "Artista", album: "Álbum",
  durationMs: 215000, positionMs: 64000, volume: 0.7, muted: false, shuffle: false, repeat: "Off", cover: null, hasTrack: true,
};
const QUEUE = {
  items: Array.from({ length: 12 }, (_, i) => ({ title: `Canción ${i + 1}`, artist: "Artista", album: "Álbum", durationMs: 180000 + i * 7000 })),
  current: 2,
  name: "Mi lista",
};

/** Carga un skin en un marco y devuelve lo que informa. */
function run(id, { keep = false, timeout = 10000 } = {}) {
  return new Promise((resolve) => {
    const f = document.createElement("iframe");
    // El navegador de la vista previa bloquea los marcos con origen opaco; en la
    // app el marco va aislado del todo (sin allow-same-origin).
    f.sandbox = "allow-scripts allow-same-origin";
    f.src = "/skin/frame/index.html";
    f.style.cssText = "border:0;background:transparent;width:1px;height:1px";
    const logs = [];
    let ready = null;
    let timer;
    const finish = (r) => {
      clearTimeout(timer);
      window.removeEventListener("message", onMsg);
      if (!keep) f.remove();
      resolve({ id, ...r, logs: logs.slice(0, 8) });
    };
    const onMsg = (e) => {
      if (e.source !== f.contentWindow) return;
      const m = e.data;
      if (m.t === "log") logs.push(`${m.level}: ${m.msg}`);
      else if (m.t === "error") finish({ result: "fatal", error: m.msg });
      else if (m.t === "ready") {
        ready = m;
        if (keep) {
          f.style.width = `${m.w}px`;
          f.style.height = `${m.h}px`;
        }
        f.contentWindow.postMessage({ t: "state", s: STATE }, "*");
        f.contentWindow.postMessage({ t: "queue", q: QUEUE }, "*");
        setTimeout(() => f.contentWindow.postMessage({ t: "stats" }, "*"), 700);
      } else if (m.t === "stats" && ready) {
        const clean = !m.errors && !m.unsupported.length && !m.missing.length;
        finish({ result: clean ? "ok" : "partial", w: ready.w, h: ready.h, title: ready.title, shaped: !!ready.rects, ...m });
      }
    };
    window.addEventListener("message", onMsg);
    f.onload = () => f.contentWindow.postMessage({ t: "init", url: `/skin/wmz/${encodeURIComponent(id)}` }, "*");
    timer = setTimeout(() => finish({ result: "timeout" }), timeout);
    stage.append(f);
  });
}

async function single(id) {
  const r = await run(id, { keep: true, timeout: 20000 });
  out.textContent = JSON.stringify(r, null, 1);
  window.__bench = r;
}

async function all() {
  const skins = await (await fetch("/skin/list")).json();
  const results = [];
  const t0 = performance.now();
  const limit = Number(params.get("limit")) || skins.length;
  const queue = skins.slice(0, limit);
  const workers = Array.from({ length: 4 }, async () => {
    while (queue.length) {
      const s = queue.shift();
      results.push(await run(s.id));
      out.textContent = `${results.length} / ${limit}…`;
    }
  });
  await Promise.all(workers);
  const count = (k) => results.filter((r) => r.result === k).length;
  const tally = (key) => {
    const c = {};
    for (const r of results) for (const v of r[key] ?? []) c[v] = (c[v] ?? 0) + 1;
    return Object.entries(c).sort((a, b) => b[1] - a[1]).slice(0, 25);
  };
  const errs = {};
  for (const r of results) for (const l of r.logs ?? []) {
    const k = l.replace(/#\w+/g, "#…").replace(/"[^"]*"/g, '"…"').slice(0, 90);
    errs[k] = (errs[k] ?? 0) + 1;
  }
  const summary = {
    skins: results.length,
    seconds: Math.round((performance.now() - t0) / 100) / 10,
    ok: count("ok"),
    partial: count("partial"),
    fatal: count("fatal"),
    timeout: count("timeout"),
    sinErroresDeScript: results.filter((r) => r.result !== "fatal" && r.result !== "timeout" && !r.errors).length,
    conForma: results.filter((r) => r.shaped).length,
    noSoportados: tally("unsupported"),
    stubs: tally("stubbed"),
    errores: Object.entries(errs).sort((a, b) => b[1] - a[1]).slice(0, 30),
    fatales: results.filter((r) => r.result === "fatal").slice(0, 30).map((r) => `${r.id}: ${r.error}`),
  };
  window.__bench = { summary, results };
  out.textContent = JSON.stringify(summary, null, 1);
}

if (params.get("skin")) void single(params.get("skin"));
else void all();
