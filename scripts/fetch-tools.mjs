// Windows conserva su script de actualización; Linux usa los binarios x86_64 fijados.
import { createHash } from "node:crypto";
import { chmod, mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} terminó con código ${result.status}`);
}

if (process.platform === "win32") {
  run("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
    "scripts/fetch-tools.ps1", ...process.argv.slice(2)]);
} else {
  if (process.platform !== "linux" || process.arch !== "x64") {
    throw new Error("Solo se distribuyen herramientas para Windows y Linux x86_64.");
  }
  if (process.argv.length > 2) throw new Error("Linux usa las versiones de tools.lock.json, sin argumentos.");
  const lock = JSON.parse((await readFile(join(root, "tools.lock.json"), "utf8")).replace(/^\uFEFF/, "")).linux;
  const temp = await mkdtemp(join(tmpdir(), "xp-music-tools-"));
  try {
    for (const [key, repo, dest] of [
      ["ytDlp", "yt-dlp/yt-dlp", "yt-dlp"],
      ["deno", "denoland/deno", "."],
    ]) {
      const { version, asset, sha256 } = lock[key];
      console.log(`Descargando ${asset} ${version}…`);
      const response = await fetch(`https://github.com/${repo}/releases/download/${version}/${asset}`);
      if (!response.ok) throw new Error(`Descarga de ${asset}: HTTP ${response.status}`);
      const bytes = Buffer.from(await response.arrayBuffer());
      if (createHash("sha256").update(bytes).digest("hex") !== sha256) {
        throw new Error(`SHA-256 de ${asset} no coincide con tools.lock.json`);
      }
      const zip = join(temp, asset);
      await writeFile(zip, bytes);
      const target = join(root, "tools", dest);
      if (dest !== ".") await rm(target, { recursive: true, force: true });
      await mkdir(target, { recursive: true });
      if (asset.endsWith(".zip")) {
        run("unzip", ["-oq", zip, "-d", target]);
      } else {
        // AppImage no debe inspeccionar las bibliotecas Python del paquete onedir.
        const executable = join(target, asset);
        await writeFile(executable, bytes);
        await chmod(executable, 0o755);
      }
    }
    run(join(root, "tools/yt-dlp/yt-dlp_linux"), ["--version"]);
    run(join(root, "tools/deno"), ["--version"]);
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}
