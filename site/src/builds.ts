// Qué es cada archivo de una release, a partir de su nombre (los que genera Tauri:
// XP.Music_1.0.2_x64-setup.exe, XP.Music_1.0.2_amd64.deb, XP.Music-1.0.2-1.aarch64.rpm…).
// Sin dependencias del navegador para probarlo con Node (site/builds.test.mjs).

export type Os = "windows" | "macos" | "linux";
export type Arch = "x64" | "arm64";
export type Build = { os: Os; arch: Arch; kind: string };

/** Solo instaladores: fuera firmas (.sig), latest.json y los paquetes del actualizador (.tar.gz). */
const INSTALLER = /\.(exe|msi|dmg|deb|rpm|AppImage)$/i;

export function classify(name: string): Build | null {
  const ext = INSTALLER.exec(name)?.[1]?.toLowerCase();
  if (!ext) return null;
  // aarch64 no contiene "x64", pero x86_64 sí: ARM primero.
  const arch: Arch | null = /arm64|aarch64/i.test(name) ? "arm64" : /x64|amd64|x86_64/i.test(name) ? "x64" : null;
  if (!arch) return null;
  const os: Os = ext === "exe" || ext === "msi" ? "windows" : ext === "dmg" ? "macos" : "linux";
  const kind = ext === "exe" ? "Instalador" : ext === "appimage" ? "AppImage" : `.${ext}`;
  return { os, arch, kind };
}

export const OS_NAME: Record<Os, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };

export function archName(os: Os, arch: Arch) {
  if (os === "macos") return arch === "arm64" ? "Apple Silicon" : "Intel";
  return arch === "arm64" ? "ARM64" : "x64";
}

/** Sistema del visitante, para poner su tarjeta la primera (null en móviles). */
export function detectOs(ua: string): Os | null {
  if (/Android|iPhone|iPad/i.test(ua)) return null;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac OS X|Macintosh/i.test(ua)) return "macos";
  if (/Linux|X11/i.test(ua)) return "linux";
  return null;
}
