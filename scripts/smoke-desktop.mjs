// Smoke test de Windows y macOS: lanza la app y espera a que el log diga que la interfaz
// está montada y que encontró yt-dlp/deno; luego la cierra. (Linux: smoke-linux.sh.)
//   node scripts/smoke-desktop.mjs EJECUTABLE [SEGUNDOS]
import { spawn, spawnSync } from "node:child_process";
import { closeSync, openSync, readSync, readdirSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const [binary, seconds = "60"] = process.argv.slice(2);
if (!binary) {
  console.error("Uso: node scripts/smoke-desktop.mjs EJECUTABLE [SEGUNDOS]");
  process.exit(2);
}

// Misma ruta que AppDirs::logs (dirs::data_local_dir()/xp-music/logs).
const dataLocal = process.platform === "win32" ? process.env.LOCALAPPDATA
  : process.platform === "darwin" ? join(homedir(), "Library", "Application Support")
  : process.env.XDG_DATA_HOME ?? join(homedir(), ".local", "share");
const logs = join(dataLocal, "xp-music", "logs");
const expected = ["UI montada", "herramientas encontradas"];

const sizes = () => {
  try {
    return new Map(readdirSync(logs).map((name) => [name, statSync(join(logs, name)).size]));
  } catch {
    return new Map();
  }
};
// Solo cuenta lo escrito después de lanzar la app (puede haber logs de ejecuciones previas).
const before = sizes();
function newLog() {
  let text = "";
  for (const [name, size] of sizes()) {
    const from = before.get(name) ?? 0;
    if (size <= from) continue;
    const buffer = Buffer.alloc(size - from);
    const fd = openSync(join(logs, name), "r");
    try { readSync(fd, buffer, 0, buffer.length, from); } finally { closeSync(fd); }
    text += buffer.toString("utf8");
  }
  return text;
}

const child = spawn(binary, [], { stdio: "ignore" });
let exited = false;
child.on("exit", () => { exited = true; });

function stop() {
  if (exited) return;
  // En Windows hay que cerrar también los procesos de WebView2.
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"]);
  else child.kill("SIGKILL");
}

let log = "";
const deadline = Date.now() + Number(seconds) * 1000;
while (Date.now() < deadline && !exited) {
  log = newLog();
  if (expected.every((line) => log.includes(line))) {
    stop();
    console.log(`Smoke OK (${process.platform}): ${expected.join(" y ")}.`);
    process.exit(0);
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
}
log = newLog();
stop();
console.error(`${exited ? "La app terminó antes de tiempo" : "Tiempo agotado"}; log de la app:\n${log || "(vacío)"}`);
process.exit(1);
