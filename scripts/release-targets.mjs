// Single source for the artifacts of each release: used by the updater manifest
// (update-manifest.mjs), the notes (release-notes.mjs) and the CI collection
// (collect-artifacts.mjs). The names are the ones the Tauri 2.10 bundler generates
// (tauri-bundler: nsis/mod.rs, linux/{debian,rpm}.rs, appimage/linuxdeploy.rs, dmg/mod.rs).
// macOS is experimental and has no updater: only the `.dmg` is published, no `.app.tar.gz`.
import { readFile, readdir } from "node:fs/promises";

// target: CI job id (name of the Actions artifact); matches the Tauri updater.
// suffix: end of the file name (GitHub assets replace spaces with dots, which is why
//   files are matched by suffix).
// installer: listed in the download table of the notes.
// updater: `latest.json` keys for this file (with its `.sig`); the last one may be a
//   formatless alias used by existing installations.
export const ARTIFACTS = [
  { target: "windows-x86_64", system: "Windows", arch: "x64", format: "Installer (.exe)",
    suffix: (v) => `_${v}_x64-setup.exe`, installer: true,
    updater: ["windows-x86_64-nsis", "windows-x86_64"] },
  { target: "windows-aarch64", system: "Windows", arch: "ARM64", format: "Installer (.exe)",
    suffix: (v) => `_${v}_arm64-setup.exe`, installer: true,
    updater: ["windows-aarch64-nsis", "windows-aarch64"] },
  { target: "linux-x86_64", system: "Linux", arch: "x64", format: "Debian/Ubuntu (.deb)",
    suffix: (v) => `_${v}_amd64.deb`, installer: true, updater: ["linux-x86_64-deb"] },
  { target: "linux-x86_64", system: "Linux", arch: "x64", format: "AppImage",
    suffix: (v) => `_${v}_amd64.AppImage`, installer: true, updater: ["linux-x86_64-appimage"] },
  { target: "linux-x86_64", system: "Linux", arch: "x64", format: "Fedora/openSUSE (.rpm)",
    suffix: (v) => `-${v}-1.x86_64.rpm`, installer: true, updater: ["linux-x86_64-rpm"] },
  { target: "linux-aarch64", system: "Linux", arch: "ARM64", format: "Debian/Ubuntu (.deb)",
    suffix: (v) => `_${v}_arm64.deb`, installer: true, updater: ["linux-aarch64-deb"] },
  { target: "linux-aarch64", system: "Linux", arch: "ARM64", format: "AppImage",
    suffix: (v) => `_${v}_aarch64.AppImage`, installer: true, updater: ["linux-aarch64-appimage"] },
  { target: "linux-aarch64", system: "Linux", arch: "ARM64", format: "Fedora/openSUSE (.rpm)",
    suffix: (v) => `-${v}-1.aarch64.rpm`, installer: true, updater: ["linux-aarch64-rpm"] },
  { target: "darwin-x86_64", system: "macOS", arch: "Intel", format: "Disk image (.dmg)",
    suffix: (v) => `_${v}_x64.dmg`, installer: true },
  { target: "darwin-aarch64", system: "macOS", arch: "Apple Silicon", format: "Disk image (.dmg)",
    suffix: (v) => `_${v}_aarch64.dmg`, installer: true },
];

export async function readVersion() {
  const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
  return version;
}

export function unique(items, suffix, source) {
  const matches = items.filter((item) => item.name.endsWith(suffix));
  if (matches.length !== 1) {
    throw new Error(`Expected one ${source} artifact ending in ${suffix}; found ${matches.length}`);
  }
  return matches[0];
}

export async function listFiles(directory) {
  return (await readdir(directory, { recursive: true, withFileTypes: true })).filter((entry) => entry.isFile());
}

// While the release is a draft, GitHub gives its assets a URL with a provisional
// tag (`.../download/untagged-<hash>/file`); once published it becomes
// `.../download/<tag>/file`. The URL from the API is validated and the final one is
// returned, which is the one that goes into the notes and latest.json.
export function downloadUrl(asset, tag) {
  const url = new URL(asset.browser_download_url);
  const parts = url.pathname.split("/");
  const given = parts[5] === undefined ? "" : decodeURIComponent(parts[5]);
  if (url.protocol !== "https:" || url.host !== "github.com" || url.username || url.password ||
      url.search || url.hash || parts.length !== 7 || !parts[1] || !parts[2] ||
      parts[3] !== "releases" || parts[4] !== "download" ||
      (given !== tag && !/^untagged-[0-9a-f]+$/.test(given)) ||
      decodeURIComponent(parts[6]) !== asset.name) {
    throw new Error(`Unsafe GitHub download URL for ${asset.name}`);
  }
  return `https://github.com/${parts[1]}/${parts[2]}/releases/download/${encodeURIComponent(tag)}/${parts[6]}`;
}

// `node scripts/release-targets.mjs signed-count`: number of `.sig` signatures a release must have.
if (import.meta.main && process.argv[2] === "signed-count") {
  console.log(ARTIFACTS.filter((a) => a.updater).length);
}
