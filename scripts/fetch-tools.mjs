// Descarga yt-dlp y deno a tools/ con las versiones y SHA-256 fijados en tools.lock.json.
// Destino normalizado e idéntico en todas las arquitecturas (lo busca la app Rust):
//   windows: tools/deno.exe, tools/yt-dlp/yt-dlp.exe (onedir)
//   linux:   tools/deno,     tools/yt-dlp/yt-dlp_linux (un solo archivo)
//   macos:   tools/deno,     tools/yt-dlp/yt-dlp_macos (un solo archivo, universal)
// Objetivo: process.platform/arch, o XP_MUSIC_TOOLS_TARGET (p. ej. "linux-arm64").
// Carpeta de salida: tools/, o XP_MUSIC_TOOLS_DEST. `-Update` en Windows delega en el .ps1.
import { createHash } from "node:crypto";
import { chmod, mkdtemp, mkdir, readFile, rename, rm, writeFile, access } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

// win = ejecutable de yt-dlp dentro del zip si no se llama yt-dlp.exe (release arm64: yt-dlp_arm64.exe).
// lock = clave de tools.lock.json (null: claves ytDlp/deno en la raíz, Windows x64 histórico).
const TARGETS = {
  "windows-x64": { lock: null, os: "windows", ytDlp: "yt-dlp.exe" },
  "windows-arm64": { lock: "windows-arm64", os: "windows", ytDlp: "yt-dlp.exe", packagedYtDlp: "yt-dlp_arm64.exe" },
  "linux-x64": { lock: "linux", os: "linux", ytDlp: "yt-dlp_linux" },
  "linux-arm64": { lock: "linux-arm64", os: "linux", ytDlp: "yt-dlp_linux" },
  "macos-x64": { lock: "macos-x64", os: "macos", ytDlp: "yt-dlp_macos" },
  "macos-arm64": { lock: "macos-arm64", os: "macos", ytDlp: "yt-dlp_macos" },
};
const OS = { win32: "windows", linux: "linux", darwin: "macos" };

export function detectTarget(platform, arch, override) {
  const target = override || `${OS[platform]}-${arch}`;
  if (!TARGETS[target]) {
    throw new Error(`Objetivo no soportado: ${target}. Disponibles: ${Object.keys(TARGETS).join(", ")}`);
  }
  return target;
}

// Función pura: objetivo + contenido de tools.lock.json -> qué descargar y dónde queda (relativo a tools/).
export function planFor(target, lockJson) {
  const t = TARGETS[target];
  if (!t) throw new Error(`Objetivo no soportado: ${target}`);
  const entry = t.lock ? lockJson[t.lock] : lockJson;
  if (!entry?.ytDlp || !entry?.deno) throw new Error(`tools.lock.json no tiene entrada para ${target}`);
  const denoExe = t.os === "windows" ? "deno.exe" : "deno";
  return {
    target,
    deno: { ...entry.deno, repo: "denoland/deno", archive: true, path: denoExe },
    ytDlp: {
      ...entry.ytDlp,
      repo: "yt-dlp/yt-dlp",
      archive: entry.ytDlp.asset.endsWith(".zip"),
      path: `yt-dlp/${t.ytDlp}`,
      packagedAs: t.packagedYtDlp,
    },
  };
}

function tryRun(command, args) {
  const r = spawnSync(command, args, { encoding: "utf8" });
  return r.error || r.status !== 0 ? (r.error?.message ?? r.stderr) : null;
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} terminó con código ${result.status}`);
}

// GNU tar no abre zips; bsdtar (Windows 10+, macOS) sí; en Linux suele haber unzip.
function extractZip(zip, dir) {
  const attempts = [["unzip", ["-oq", zip, "-d", dir]], ["tar", ["-xf", zip, "-C", dir]], ["bsdtar", ["-xf", zip, "-C", dir]]];
  const errors = [];
  for (const [command, args] of attempts) {
    const error = tryRun(command, args);
    if (!error) return;
    errors.push(`${command}: ${String(error).trim()}`);
  }
  throw new Error(`No se pudo extraer ${zip}:\n${errors.join("\n")}`);
}

async function exists(path) {
  return access(path).then(() => true, () => false);
}

async function main() {
  const native = detectTarget(process.platform, process.arch, "");
  const target = detectTarget(process.platform, process.arch, process.env.XP_MUSIC_TOOLS_TARGET);
  if (process.argv.length > 2) {
    if (process.platform !== "win32") throw new Error("Sin argumentos: se usan las versiones de tools.lock.json.");
    return run("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts/fetch-tools.ps1", ...process.argv.slice(2)]);
  }
  const lockJson = JSON.parse((await readFile(join(root, "tools.lock.json"), "utf8")).replace(/^\uFEFF/, ""));
  const plan = planFor(target, lockJson);
  const tools = process.env.XP_MUSIC_TOOLS_DEST || join(root, "tools");
  const temp = await mkdtemp(join(tmpdir(), "xp-music-tools-"));
  try {
    for (const spec of [plan.ytDlp, plan.deno]) {
      const { version, asset, sha256, repo } = spec;
      console.log(`Descargando ${asset} ${version} (${target})…`);
      const response = await fetch(`https://github.com/${repo}/releases/download/${version}/${asset}`);
      if (!response.ok) throw new Error(`Descarga de ${asset}: HTTP ${response.status}`);
      const bytes = Buffer.from(await response.arrayBuffer());
      if (createHash("sha256").update(bytes).digest("hex") !== sha256) {
        throw new Error(`SHA-256 de ${asset} no coincide con tools.lock.json`);
      }
      const out = join(tools, spec.path);
      const dir = spec === plan.ytDlp ? join(tools, "yt-dlp") : tools;
      if (spec === plan.ytDlp) await rm(dir, { recursive: true, force: true });
      await mkdir(dir, { recursive: true });
      if (spec.archive) {
        const zip = join(temp, asset);
        await writeFile(zip, bytes);
        extractZip(zip, dir);
        if (spec.packagedAs && !(await exists(out))) {
          await rename(join(dir, spec.packagedAs), out);
        }
      } else {
        await writeFile(out, bytes);
      }
      if (!(await exists(out))) throw new Error(`Falta ${spec.path} tras extraer ${asset}`);
      if (process.platform !== "win32") await chmod(out, 0o755);
    }
    if (target.startsWith("windows")) await rm(join(tools, "yt-dlp.exe"), { force: true });
    if (target === native) {
      run(join(tools, plan.ytDlp.path), ["--version"]);
      run(join(tools, plan.deno.path), ["--version"]);
    } else {
      console.log(`Objetivo ${target} distinto de ${native}: solo se verificó el SHA-256 (no se ejecutan).`);
    }
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
