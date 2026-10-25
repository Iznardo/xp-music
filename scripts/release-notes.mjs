// Release notes: generated download table + the maintainer's notes.
// The generated block goes between markers so it can be regenerated when the workflow is retried.
import { createHash } from "node:crypto";
import { readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { ARTIFACTS, downloadUrl, listFiles, readVersion, unique } from "./release-targets.mjs";

export const START = "<!-- xp-music:downloads:start -->";
export const END = "<!-- xp-music:downloads:end -->";

/** The maintainer's notes without the generated block (fails if the block is half there). */
export function stripDownloads(body) {
  const text = body ?? "";
  const start = text.indexOf(START);
  const end = text.indexOf(END);
  if (start === -1 && end === -1) return text;
  if (start === -1 || end < start || text.indexOf(START, start + 1) !== -1) {
    throw new Error("The download block in the notes is incomplete or duplicated");
  }
  return `${text.slice(0, start)}${text.slice(end + END.length)}`.trim();
}

/**
 * Table: "System" (OS and architecture), SHA-256 and a link with the file name, which
 * already shows the format. The files only the updater uses (`.sig`, `latest.json`)
 * are not listed: the app downloads them by itself.
 * @param rows {{artifact, url, sha256}}[]
 */
export function renderDownloads(rows) {
  const lines = [START, "## Downloads", "", "| System | SHA-256 | Download |", "| --- | --- | --- |"];
  for (const row of rows.filter((r) => r.artifact.installer)) {
    const { system, arch } = row.artifact;
    const file = decodeURIComponent(new URL(row.url).pathname.split("/").pop());
    lines.push(`| ${system} ${arch} | \`${row.sha256}\` | [${file}](${row.url}) |`);
  }
  lines.push(END);
  return lines.join("\n");
}

/** Download block followed by the maintainer's notes (idempotent). */
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
  for (const artifact of ARTIFACTS.filter((a) => a.installer)) {
    const suffix = artifact.suffix(version);
    const local = unique(files, suffix, "local");
    const remote = unique(release.assets, suffix, "remote");
    const path = join(local.parentPath, local.name);
    if ((await stat(path)).size !== remote.size) throw new Error(`Payload size mismatch for ${remote.name}`);
    const sha256 = createHash("sha256").update(await readFile(path)).digest("hex");
    // GitHub computes its own hash on upload: if it reports it, it must match the local one.
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
