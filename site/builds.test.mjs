// node --test site/builds.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { classify, detectOs, kindLabel } from "./src/builds.ts";

test("recognizes a release matrix", () => {
  const cases = {
    "XP.Music_1.0.2_x64-setup.exe": ["windows", "x64", "exe"],
    "XP.Music_1.0.2_arm64-setup.exe": ["windows", "arm64", "exe"],
    "XP.Music_1.0.2_amd64.deb": ["linux", "x64", "deb"],
    "XP.Music_1.0.2_amd64.AppImage": ["linux", "x64", "appimage"],
    "XP.Music-1.0.2-1.x86_64.rpm": ["linux", "x64", "rpm"],
    "XP.Music_1.0.2_arm64.deb": ["linux", "arm64", "deb"],
    "XP.Music_1.0.2_aarch64.AppImage": ["linux", "arm64", "appimage"],
    "XP.Music-1.0.2-1.aarch64.rpm": ["linux", "arm64", "rpm"],
    "XP.Music_1.0.2_x64.dmg": ["macos", "x64", "dmg"],
    "XP.Music_1.0.2_aarch64.dmg": ["macos", "arm64", "dmg"],
  };
  for (const [name, [os, arch, kind]] of Object.entries(cases)) assert.deepEqual(classify(name), { os, arch, kind }, name);
});

test("ignores anything that is not an installer", () => {
  for (const name of ["latest.json", "XP.Music_1.0.2_x64-setup.exe.sig", "XP.Music_aarch64.app.tar.gz", "XP.Music_1.0.2_amd64.AppImage.sig"])
    assert.equal(classify(name), null, name);
});

test("labels builds for the buttons", () => {
  assert.equal(kindLabel("exe", "Installer"), "Installer");
  assert.equal(kindLabel("appimage", "Installer"), "AppImage");
  assert.equal(kindLabel("deb", "Installer"), ".deb");
});

test("detects the visitor's system", () => {
  assert.equal(detectOs("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"), "windows");
  assert.equal(detectOs("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"), "macos");
  assert.equal(detectOs("Mozilla/5.0 (X11; Linux x86_64)"), "linux");
  assert.equal(detectOs("Mozilla/5.0 (Linux; Android 14; Pixel 8)"), null);
});
