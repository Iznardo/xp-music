import { readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { ARTIFACTS, downloadUrl, listFiles, readVersion, unique } from "./release-targets.mjs";
import { stripDownloads } from "./release-notes.mjs";

export async function generateManifest(metadataPath, packagesDirectory, outputPath) {
  const version = await readVersion();
  const release = JSON.parse(await readFile(metadataPath, "utf8"));
  if (release.tag_name !== `v${version}`) throw new Error(`Expected release tag v${version}`);
  if (release.draft !== true || release.prerelease !== false) {
    throw new Error("Release must be a draft, not a prerelease or already published");
  }
  if (!Array.isArray(release.assets) || release.assets.some((asset) => typeof asset.name !== "string")) {
    throw new Error("Release assets must have names");
  }
  if ((release.body != null && typeof release.body !== "string") ||
      typeof release.created_at !== "string" || !Number.isFinite(Date.parse(release.created_at))) {
    throw new Error("Release notes or creation date are invalid");
  }

  const files = await listFiles(packagesDirectory);
  const platforms = {};
  for (const artifact of ARTIFACTS.filter((a) => a.updater)) {
    const suffix = artifact.suffix(version);
    const local = unique(files, suffix, "local");
    const localSignature = unique(files, `${suffix}.sig`, "local");
    if (join(localSignature.parentPath, localSignature.name) !== join(local.parentPath, `${local.name}.sig`)) {
      throw new Error(`Signature must accompany ${local.name}`);
    }
    const remote = unique(release.assets, suffix, "remote");
    const remoteSignature = unique(release.assets, `${suffix}.sig`, "remote");
    if (remoteSignature.name !== `${remote.name}.sig` || !(remoteSignature.size > 0)) {
      throw new Error(`Missing or empty uploaded signature for ${remote.name}`);
    }
    const size = (await stat(join(local.parentPath, local.name))).size;
    if (size <= 0 || size !== remote.size) throw new Error(`Payload size mismatch for ${remote.name}`);
    const signature = (await readFile(join(localSignature.parentPath, localSignature.name), "utf8")).trim();
    if (!signature) throw new Error(`Empty signature for ${local.name}`);
    downloadUrl(remoteSignature, release.tag_name);
    const entry = { signature, url: downloadUrl(remote, release.tag_name) };
    // Clave específica del formato y, si existe, el alias heredado (mismo objeto).
    for (const key of artifact.updater) platforms[key] = entry;
  }
  const manifest = {
    version,
    // Solo las notas del mantenedor: la tabla de descargas se genera aparte.
    notes: stripDownloads(release.body),
    pub_date: release.created_at,
    platforms,
  };
  await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

if (import.meta.main) {
  try {
    if (process.argv.length !== 5) {
      throw new Error("Usage: node scripts/update-manifest.mjs RELEASE_METADATA_JSON PACKAGES_DIRECTORY OUTPUT_JSON");
    }
    await generateManifest(...process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
