import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { detectTarget, planFor } from "./fetch-tools.mjs";

const lock = JSON.parse(readFileSync(new URL("../tools.lock.json", import.meta.url), "utf8").replace(/^\uFEFF/, ""));

// target -> [yt-dlp asset, yt-dlp destino, deno asset, deno destino]
const expected = {
  "windows-x64": ["yt-dlp_win.zip", "yt-dlp/yt-dlp.exe", "deno-x86_64-pc-windows-msvc.zip", "deno.exe"],
  "windows-arm64": ["yt-dlp_win_arm64.zip", "yt-dlp/yt-dlp.exe", "deno-aarch64-pc-windows-msvc.zip", "deno.exe"],
  "linux-x64": ["yt-dlp_linux", "yt-dlp/yt-dlp_linux", "deno-x86_64-unknown-linux-gnu.zip", "deno"],
  "linux-arm64": ["yt-dlp_linux_aarch64", "yt-dlp/yt-dlp_linux", "deno-aarch64-unknown-linux-gnu.zip", "deno"],
  "macos-x64": ["yt-dlp_macos", "yt-dlp/yt-dlp_macos", "deno-x86_64-apple-darwin.zip", "deno"],
  "macos-arm64": ["yt-dlp_macos", "yt-dlp/yt-dlp_macos", "deno-aarch64-apple-darwin.zip", "deno"],
};

test("los seis objetivos resuelven a su entrada del lock y a nombres normalizados", () => {
  for (const [target, [ytAsset, ytPath, denoAsset, denoPath]] of Object.entries(expected)) {
    const plan = planFor(target, lock);
    assert.equal(plan.ytDlp.asset, ytAsset, target);
    assert.equal(plan.ytDlp.path, ytPath, target);
    assert.equal(plan.deno.asset, denoAsset, target);
    assert.equal(plan.deno.path, denoPath, target);
    assert.equal(plan.ytDlp.version, "2026.08.19", target);
    assert.equal(plan.deno.version, "v2.9.7", target);
    assert.match(plan.ytDlp.sha256, /^[0-9a-f]{64}$/, target);
    assert.match(plan.deno.sha256, /^[0-9a-f]{64}$/, target);
  }
});

test("detectTarget usa plataforma/arquitectura, el override y rechaza combinaciones no soportadas", () => {
  assert.equal(detectTarget("win32", "x64", ""), "windows-x64");
  assert.equal(detectTarget("win32", "arm64", ""), "windows-arm64");
  assert.equal(detectTarget("linux", "arm64", ""), "linux-arm64");
  assert.equal(detectTarget("darwin", "x64", ""), "macos-x64");
  assert.equal(detectTarget("linux", "x64", "macos-arm64"), "macos-arm64");
  assert.throws(() => detectTarget("linux", "ia32", ""), /no soportado/);
  assert.throws(() => detectTarget("freebsd", "x64", ""), /no soportado/);
  assert.throws(() => detectTarget("linux", "x64", "linux-riscv64"), /no soportado/);
});

test("el zip de Windows ARM64 renombra yt-dlp_arm64.exe y un solo archivo no es archive", () => {
  assert.equal(planFor("windows-arm64", lock).ytDlp.packagedAs, "yt-dlp_arm64.exe");
  assert.equal(planFor("windows-x64", lock).ytDlp.packagedAs, undefined);
  assert.equal(planFor("linux-arm64", lock).ytDlp.archive, false);
  assert.equal(planFor("windows-arm64", lock).ytDlp.archive, true);
});
