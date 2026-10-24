import { readFile, readdir, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

function unique(items, suffix, source) {
  const matches = items.filter((item) => item.name.endsWith(suffix));
  if (matches.length !== 1) {
    throw new Error(`Expected one ${source} artifact ending in ${suffix}; found ${matches.length}`);
  }
  return matches[0];
}

function downloadUrl(asset, tag) {
  const url = new URL(asset.browser_download_url);
  const parts = url.pathname.split("/");
  if (url.protocol !== "https:" || url.host !== "github.com" || url.username || url.password ||
      url.search || url.hash || parts.length !== 7 || !parts[1] || !parts[2] ||
      parts[3] !== "releases" || parts[4] !== "download" ||
      decodeURIComponent(parts[5]) !== tag || decodeURIComponent(parts[6]) !== asset.name) {
    throw new Error(`Unsafe GitHub download URL for ${asset.name}`);
  }
  return asset.browser_download_url;
}

export async function generateManifest(metadataPath, packagesDirectory, outputPath) {
  const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
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

  const files = (await readdir(packagesDirectory, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile());
  const platforms = {};
  for (const [platform, suffix] of [
    ["windows-x86_64-nsis", `_${version}_x64-setup.exe`],
    ["linux-x86_64-deb", `_${version}_amd64.deb`],
    ["linux-x86_64-appimage", `_${version}_amd64.AppImage`],
  ]) {
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
    platforms[platform] = { signature, url: downloadUrl(remote, release.tag_name) };
  }
  platforms["windows-x86_64"] = platforms["windows-x86_64-nsis"];
  const manifest = {
    version,
    notes: release.body ?? "",
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
