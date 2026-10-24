// Notas de la release: tabla de descargas generada + notas del mantenedor.
// El bloque generado va entre marcadores para poder regenerarlo al reintentar el workflow.
import { createHash } from "node:crypto";
import { readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { ARTIFACTS, downloadUrl, listFiles, readVersion, unique } from "./release-targets.mjs";

export const START = "<!-- xp-music:downloads:start -->";
export const END = "<!-- xp-music:downloads:end -->";

/** Notas del mantenedor sin el bloque generado (falla si el bloque está a medias). */
export function stripDownloads(body) {
  const text = body ?? "";
  const start = text.indexOf(START);
  const end = text.indexOf(END);
  if (start === -1 && end === -1) return text;
  if (start === -1 || end < start || text.indexOf(START, start + 1) !== -1) {
    throw new Error("El bloque de descargas de las notas está incompleto o duplicado");
  }
  return `${text.slice(0, start)}${text.slice(end + END.length)}`.trim();
}

/** @param rows {{artifact, url, sha256}}[] */
export function renderDownloads(rows) {
  const link = (row) => `[Descargar](${row.url})`;
  const table = (items, headers, cells) => [
    `| ${headers.join(" | ")} |`,
    `| ${headers.map(() => "---").join(" | ")} |`,
    ...items.map((row) => `| ${cells(row).join(" | ")} |`),
  ];
  const installers = rows.filter((row) => row.artifact.installer);
  const updaterOnly = rows.filter((row) => !row.artifact.installer);
  const lines = [START, "## Descargas", "",
    ...table(installers, ["Sistema", "Arquitectura", "Formato", "SHA-256", "Descarga"],
      (row) => [row.artifact.system, row.artifact.arch, row.artifact.format, `\`${row.sha256}\``, link(row)])];
  if (updaterOnly.length) {
    lines.push("", "<details>", "<summary>Archivos del actualizador</summary>", "",
      "Los usa la actualización automática de la app (junto a sus `.sig` y `latest.json`); no hace falta descargarlos.", "",
      ...table(updaterOnly, ["Sistema", "Arquitectura", "Formato", "SHA-256", "Descarga"],
        (row) => [row.artifact.system, row.artifact.arch, row.artifact.format, `\`${row.sha256}\``, link(row)]),
      "", "</details>");
  }
  lines.push(END);
  return lines.join("\n");
}

/** Bloque de descargas seguido de las notas del mantenedor (idempotente). */
export function composeNotes(body, rows) {
  const notes = stripDownloads(body);
  return `${renderDownloads(rows)}\n${notes ? `\n${notes}\n` : ""}`;
}

export async function generateNotes(metadataPath, packagesDirectory, outputPath) {
  const version = await readVersion();
  const release = JSON.parse(await readFile(metadataPath, "utf8"));
  if (release.tag_name !== `v${version}`) throw new Error(`Expected release tag v${version}`);
  if (!Array.isArray(release.assets) || (release.body != null && typeof release.body !== "string")) {
    throw new Error("Release assets or notes are invalid");
  }
  const files = await listFiles(packagesDirectory);
  const rows = [];
  for (const artifact of ARTIFACTS.filter((a) => a.installer || a.updater)) {
    const suffix = artifact.suffix(version);
    const local = unique(files, suffix, "local");
    const remote = unique(release.assets, suffix, "remote");
    const path = join(local.parentPath, local.name);
    if ((await stat(path)).size !== remote.size) throw new Error(`Payload size mismatch for ${remote.name}`);
    const sha256 = createHash("sha256").update(await readFile(path)).digest("hex");
    // GitHub calcula su propio hash al subir: si lo informa, debe coincidir con el local.
    if (typeof remote.digest === "string" && remote.digest !== `sha256:${sha256}`) {
      throw new Error(`SHA-256 mismatch for ${remote.name}`);
    }
    rows.push({ artifact, url: downloadUrl(remote, release.tag_name), sha256 });
  }
  const notes = composeNotes(release.body, rows);
  await writeFile(outputPath, notes);
  return notes;
}

if (import.meta.main) {
  try {
    if (process.argv.length !== 5) {
      throw new Error("Usage: node scripts/release-notes.mjs RELEASE_METADATA_JSON PACKAGES_DIRECTORY OUTPUT_MD");
    }
    await generateNotes(...process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
