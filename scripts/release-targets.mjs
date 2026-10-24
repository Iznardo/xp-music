// Fuente única de los artefactos de cada release: la usan el manifiesto del updater
// (update-manifest.mjs), las notas (release-notes.mjs) y la recogida en CI
// (collect-artifacts.mjs). Los nombres son los que genera el bundler de Tauri 2.10
// (tauri-bundler: nsis/mod.rs, linux/{debian,rpm}.rs, appimage/linuxdeploy.rs, dmg/mod.rs),
// salvo el `.app.tar.gz` de macOS, que Tauri llama «XP Music.app.tar.gz» sin arquitectura:
// collect-artifacts.mjs lo renombra para que x64 y aarch64 no choquen en la release.
import { readFile, readdir } from "node:fs/promises";

export const PRODUCT = "XP Music";

// target: id del job de CI (nombre del artefacto de Actions); coincide con el updater de Tauri.
// suffix: final del nombre de archivo (los assets de GitHub cambian los espacios por puntos,
//   por eso se compara por sufijo).
// installer: aparece en la tabla de descargas de las notas.
// updater: claves de `latest.json` para este archivo (con su `.sig`); la última puede ser
//   un alias sin formato que usan las instalaciones ya existentes.
// built: sufijo con el que lo deja Tauri, si es distinto de `suffix`.
export const ARTIFACTS = [
  { target: "windows-x86_64", system: "Windows", arch: "x64", format: "Instalador (.exe)",
    suffix: (v) => `_${v}_x64-setup.exe`, installer: true,
    updater: ["windows-x86_64-nsis", "windows-x86_64"] },
  { target: "windows-aarch64", system: "Windows", arch: "ARM64", format: "Instalador (.exe)",
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
  { target: "darwin-x86_64", system: "macOS", arch: "Intel (x64)", format: "Imagen de disco (.dmg)",
    suffix: (v) => `_${v}_x64.dmg`, installer: true },
  { target: "darwin-x86_64", system: "macOS", arch: "Intel (x64)", format: "Actualizador (.app.tar.gz)",
    suffix: (v) => `_${v}_x64.app.tar.gz`, built: ".app.tar.gz",
    updater: ["darwin-x86_64-app", "darwin-x86_64"] },
  { target: "darwin-aarch64", system: "macOS", arch: "Apple Silicon (ARM64)", format: "Imagen de disco (.dmg)",
    suffix: (v) => `_${v}_aarch64.dmg`, installer: true },
  { target: "darwin-aarch64", system: "macOS", arch: "Apple Silicon (ARM64)", format: "Actualizador (.app.tar.gz)",
    suffix: (v) => `_${v}_aarch64.app.tar.gz`, built: ".app.tar.gz",
    updater: ["darwin-aarch64-app", "darwin-aarch64"] },
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

// Mientras la release es un draft, GitHub da a sus assets una URL con un tag
// provisional (`.../download/untagged-<hash>/archivo`); al publicarla pasa a
// `.../download/<tag>/archivo`. Se valida la URL que da la API y se devuelve la
// definitiva, que es la que irá en las notas y en latest.json.
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

// `node scripts/release-targets.mjs signed-count`: nº de firmas `.sig` que debe haber en una release.
if (import.meta.main && process.argv[2] === "signed-count") {
  console.log(ARTIFACTS.filter((a) => a.updater).length);
}
