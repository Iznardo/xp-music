// What each release file is, from its name (the ones Tauri generates:
// XP.Music_1.0.2_x64-setup.exe, XP.Music_1.0.2_amd64.deb, XP.Music-1.0.2-1.aarch64.rpm…).
// No browser dependencies, so Node can test it (site/builds.test.mjs).

export type Os = "windows" | "macos" | "linux";
export type Arch = "x64" | "arm64";
/** `kind` is the lowercase extension: exe, msi, dmg, deb, rpm, appimage. */
export type Build = { os: Os; arch: Arch; kind: string };

/** Installers only: no signatures (.sig), latest.json or updater packages (.tar.gz). */
const INSTALLER = /\.(exe|msi|dmg|deb|rpm|AppImage)$/i;

export function classify(name: string): Build | null {
  const ext = INSTALLER.exec(name)?.[1]?.toLowerCase();
  if (!ext) return null;
  // aarch64 does not contain "x64", but x86_64 does: ARM first.
  const arch: Arch | null = /arm64|aarch64/i.test(name) ? "arm64" : /x64|amd64|x86_64/i.test(name) ? "x64" : null;
  if (!arch) return null;
  const os: Os = ext === "exe" || ext === "msi" ? "windows" : ext === "dmg" ? "macos" : "linux";
  return { os, arch, kind: ext };
}

/** Button label for a build; `installer` is the translated word for Windows setups. */
export function kindLabel(kind: string, installer: string) {
  return kind === "exe" ? installer : kind === "appimage" ? "AppImage" : `.${kind}`;
}

export const OS_NAME: Record<Os, string> = { windows: "Windows", macos: "macOS", linux: "Linux" };

export function archName(os: Os, arch: Arch) {
  if (os === "macos") return arch === "arm64" ? "Apple Silicon" : "Intel";
  return arch === "arm64" ? "ARM64" : "x64";
}

/** The visitor's system, to put their card first (null on phones). */
export function detectOs(ua: string): Os | null {
  if (/Android|iPhone|iPad/i.test(ua)) return null;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac OS X|Macintosh/i.test(ua)) return "macos";
  if (/Linux|X11/i.test(ua)) return "linux";
  return null;
}
