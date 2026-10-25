// Downloads yt-dlp and deno into tools/ with the versions and SHA-256 pinned in tools.lock.json.
// Normalized destination, the same on every architecture (the Rust app looks for it):
//   windows: tools/deno.exe, tools/yt-dlp/yt-dlp.exe (onedir)
//   linux:   tools/deno,     tools/yt-dlp/yt-dlp_linux (a single file)
//   macos:   tools/deno,     tools/yt-dlp/yt-dlp_macos (a single universal file)
// Target: process.platform/arch, or XP_MUSIC_TOOLS_TARGET (e.g. "linux-arm64").
// Output folder: tools/, or XP_MUSIC_TOOLS_DEST. `-Update` on Windows delegates to the .ps1.
import { createHash } from "node:crypto";
import { chmod, mkdtemp, mkdir, readFile, rename, rm, writeFile, access } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

// packagedYtDlp = yt-dlp executable inside the zip if it is not named yt-dlp.exe (arm64 release: yt-dlp_arm64.exe).
// lock = tools.lock.json key (null: ytDlp/deno keys at the root, the historical Windows x64).
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

// Pure function: target + tools.lock.json contents -> what to download and where it goes (relative to tools/).
export function planFor(target, lockJson) {
  const t = TARGETS[target];
  if (!t) throw new Error(`Objetivo no soportado: ${target}`);
  const entry = t.lock ? lockJson[t.lock] : lockJson;
  if (!entry?.ytDlp || !entry?.deno) throw new Error(`tools.lock.json has no entry for ${target}`);
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
  if (result.status !== 0) throw new Error(`${command} exited with code ${result.status}`);
}

// GNU tar does not open zips; bsdtar (Windows 10+, macOS) does; Linux usually has unzip.
function extractZip(zip, dir) {
  const attempts = [["unzip", ["-oq", zip, "-d", dir]], ["tar", ["-xf", zip, "-C", dir]], ["bsdtar", ["-xf", zip, "-C", dir]]];
  const errors = [];
  for (const [command, args] of attempts) {
    const error = tryRun(command, args);
    if (!error) return;
    errors.push(`${command}: ${String(error).trim()}`);
  }
  throw new Error(`Could not extract ${zip}:\n${errors.join("\n")}`);
}

async function exists(path) {
  return access(path).then(() => true, () => false);
}

async function main() {
  const native = detectTarget(process.platform, process.arch, "");
  const target = detectTarget(process.platform, process.arch, process.env.XP_MUSIC_TOOLS_TARGET);
  if (process.argv.length > 2) {
    if (process.platform !== "win32") throw new Error("No arguments: the versions in tools.lock.json are used.");
    return run("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts/fetch-tools.ps1", ...process.argv.slice(2)]);
  }
  const lockJson = JSON.parse((await readFile(join(root, "tools.lock.json"), "utf8")).replace(/^\uFEFF/, ""));
  const plan = planFor(target, lockJson);
  const tools = process.env.XP_MUSIC_TOOLS_DEST || join(root, "tools");
  const temp = await mkdtemp(join(tmpdir(), "xp-music-tools-"));
  try {
    for (const spec of [plan.ytDlp, plan.deno]) {
      const { version, asset, sha256, repo } = spec;
      console.log(`Downloading ${asset} ${version} (${target})…`);
      const response = await fetch(`https://github.com/${repo}/releases/download/${version}/${asset}`);
      if (!response.ok) throw new Error(`Download of ${asset} failed: HTTP ${response.status}`);
      const bytes = Buffer.from(await response.arrayBuffer());
      if (createHash("sha256").update(bytes).digest("hex") !== sha256) {
        throw new Error(`SHA-256 of ${asset} does not match tools.lock.json`);
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
      if (!(await exists(out))) throw new Error(`${spec.path} is missing after extracting ${asset}`);
      if (process.platform !== "win32") await chmod(out, 0o755);
    }
    if (target.startsWith("windows")) await rm(join(tools, "yt-dlp.exe"), { force: true });
    if (target === native) {
      run(join(tools, plan.ytDlp.path), ["--version"]);
      run(join(tools, plan.deno.path), ["--version"]);
    } else {
      console.log(`Target ${target} differs from ${native}: only the SHA-256 was checked (they are not run).`);
    }
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
