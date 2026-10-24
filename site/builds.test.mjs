// node --test site/builds.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { classify, detectOs } from "./src/builds.ts";

test("reconoce la matriz de una release", () => {
  const cases = {
    "XP.Music_1.0.2_x64-setup.exe": ["windows", "x64", "Instalador"],
    "XP.Music_1.0.2_arm64-setup.exe": ["windows", "arm64", "Instalador"],
    "XP.Music_1.0.2_amd64.deb": ["linux", "x64", ".deb"],
    "XP.Music_1.0.2_amd64.AppImage": ["linux", "x64", "AppImage"],
    "XP.Music-1.0.2-1.x86_64.rpm": ["linux", "x64", ".rpm"],
    "XP.Music_1.0.2_arm64.deb": ["linux", "arm64", ".deb"],
    "XP.Music_1.0.2_aarch64.AppImage": ["linux", "arm64", "AppImage"],
    "XP.Music-1.0.2-1.aarch64.rpm": ["linux", "arm64", ".rpm"],
    "XP.Music_1.0.2_x64.dmg": ["macos", "x64", ".dmg"],
    "XP.Music_1.0.2_aarch64.dmg": ["macos", "arm64", ".dmg"],
  };
  for (const [name, [os, arch, kind]] of Object.entries(cases)) assert.deepEqual(classify(name), { os, arch, kind }, name);
});

test("ignora lo que no es un instalador", () => {
  for (const name of ["latest.json", "XP.Music_1.0.2_x64-setup.exe.sig", "XP.Music_aarch64.app.tar.gz", "XP.Music_1.0.2_amd64.AppImage.sig"])
    assert.equal(classify(name), null, name);
});

test("detecta el sistema del visitante", () => {
  assert.equal(detectOs("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"), "windows");
  assert.equal(detectOs("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"), "macos");
  assert.equal(detectOs("Mozilla/5.0 (X11; Linux x86_64)"), "linux");
  assert.equal(detectOs("Mozilla/5.0 (Linux; Android 14; Pixel 8)"), null);
});
