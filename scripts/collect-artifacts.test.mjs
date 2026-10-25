import assert from "node:assert/strict";
import { mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { collect } from "./collect-artifacts.mjs";

test("collect gathers the expected files and fails on gaps", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xp-music-collect-"));
  const bundle = join(directory, "bundle");
  const out = join(directory, "out");
  const v = "1.2.3";
  try {
    for (const [dir, name] of [
      ["nsis", "XP Music_1.2.3_arm64-setup.exe"], ["nsis", "XP Music_1.2.3_arm64-setup.exe.sig"],
      ["dmg", "XP Music_1.2.3_aarch64.dmg"],
      ["macos", "XP Music.app.tar.gz"], ["macos", "XP Music.app.tar.gz.sig"],
    ]) {
      await mkdir(join(bundle, dir), { recursive: true });
      await writeFile(join(bundle, dir, name), name);
    }
    assert.deepEqual(await collect("windows-aarch64", bundle, out, true, v),
      ["XP Music_1.2.3_arm64-setup.exe", "XP Music_1.2.3_arm64-setup.exe.sig"]);
    // macOS has no updater: even with a signed .app.tar.gz, only the .dmg is collected.
    assert.deepEqual(await collect("darwin-aarch64", bundle, out, true, v), ["XP Music_1.2.3_aarch64.dmg"]);
    assert.deepEqual((await readdir(out)).sort(),
      ["XP Music_1.2.3_aarch64.dmg", "XP Music_1.2.3_arm64-setup.exe", "XP Music_1.2.3_arm64-setup.exe.sig"]);
    // Without the updater signature only the installer is collected.
    await rm(out, { recursive: true });
    assert.deepEqual(await collect("windows-aarch64", bundle, out, false, v), ["XP Music_1.2.3_arm64-setup.exe"]);
    // Missing signature, or a target that was not built: error.
    await rm(join(bundle, "nsis", "XP Music_1.2.3_arm64-setup.exe.sig"));
    await assert.rejects(collect("windows-aarch64", bundle, out, true, v), /found 0/);
    await assert.rejects(collect("linux-x86_64", bundle, out, false, v), /found 0/);
    await assert.rejects(collect("plan9-x86_64", bundle, out, false, v), /Unknown target/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
