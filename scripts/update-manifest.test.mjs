import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { generateManifest } from "./update-manifest.mjs";

const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));

test("manifest uses uploaded URLs and fails closed on incomplete or unsafe releases", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xp-music-manifest-"));
  const packages = join(directory, "packages");
  const metadata = join(directory, "release.json");
  const output = join(directory, "latest.json");
  try {
    const release = {
      tag_name: `v${version}`, draft: true, prerelease: false,
      body: "Existing release notes", created_at: "2026-05-01T12:34:56Z", assets: [],
    };
    const names = [
      `XP Music_${version}_x64-setup.exe`,
      `XP Music_${version}_amd64.deb`,
      `XP Music_${version}_amd64.AppImage`,
    ];
    const signaturePaths = [];
    for (const [index, name] of names.entries()) {
      const nested = join(packages, String(index), "bundle");
      await mkdir(nested, { recursive: true });
      await writeFile(join(nested, name), "payload");
      const signaturePath = join(nested, `${name}.sig`);
      signaturePaths.push(signaturePath);
      await writeFile(signaturePath, ` signature-${index}\n`);
      for (const [assetName, size] of [[name, 7], [`${name}.sig`, 14]]) {
        const sanitized = assetName.replaceAll(" ", ".");
        release.assets.push({
          name: sanitized, size,
          browser_download_url: `https://github.com/Iznardo/xp-music/releases/download/v${version}/${sanitized}`,
        });
      }
    }
    await writeFile(metadata, JSON.stringify(release));
    // A different cwd must not change where package.json is read.
    execFileSync(process.execPath, [fileURLToPath(new URL("./update-manifest.mjs", import.meta.url)),
      metadata, packages, output], { cwd: directory });
    const manifest = JSON.parse(await readFile(output, "utf8"));
    assert.equal(manifest.version, version);
    assert.equal(manifest.notes, release.body);
    assert.equal(manifest.pub_date, release.created_at);
    assert.deepEqual(Object.keys(manifest.platforms).sort(), [
      "linux-x86_64-appimage", "linux-x86_64-deb", "windows-x86_64", "windows-x86_64-nsis",
    ]);
    for (const [index, platform] of ["windows-x86_64-nsis", "linux-x86_64-deb", "linux-x86_64-appimage"].entries()) {
      assert.deepEqual(manifest.platforms[platform], {
        signature: `signature-${index}`, url: release.assets[index * 2].browser_download_url,
      });
    }
    assert.deepEqual(manifest.platforms["windows-x86_64"], manifest.platforms["windows-x86_64-nsis"]);

    async function rejectsRelease(change, pattern) {
      const invalid = structuredClone(release);
      change(invalid);
      await writeFile(metadata, JSON.stringify(invalid));
      await rm(output, { force: true });
      await assert.rejects(generateManifest(metadata, packages, output), pattern);
      await assert.rejects(readFile(output), { code: "ENOENT" });
    }
    await rejectsRelease((r) => { r.tag_name = "v0.0.0"; }, /Expected release tag/);
    await rejectsRelease((r) => { r.draft = false; }, /must be a draft/);
    await rejectsRelease((r) => { r.prerelease = true; }, /must be a draft/);
    await rejectsRelease((r) => { r.assets.splice(0, 1); }, /remote artifact/);
    await rejectsRelease((r) => { r.assets.splice(1, 1); }, /remote artifact/);
    await rejectsRelease((r) => { r.assets.push(r.assets[0]); }, /found 2/);
    await rejectsRelease((r) => { r.assets.push(r.assets[1]); }, /found 2/);
    await rejectsRelease((r) => { r.assets[0].size++; }, /size mismatch/);
    await rejectsRelease((r) => { r.assets[1].size = 0; }, /empty uploaded signature/);
    for (const url of [
      release.assets[0].browser_download_url.replace("https:", "http:"),
      release.assets[0].browser_download_url.replace("github.com", "github.com.evil.test"),
      release.assets[0].browser_download_url.replace("github.com", "user@github.com"),
      release.assets[0].browser_download_url.replace(`/v${version}/`, "/v0.0.0/"),
      `${release.assets[0].browser_download_url}?redirect=evil`,
      "https://github.com/Iznardo/xp-music/blob/main/installer.exe",
    ]) {
      await rejectsRelease((r) => { r.assets[0].browser_download_url = url; }, /Unsafe/);
    }
    await writeFile(metadata, JSON.stringify(release));
    await writeFile(signaturePaths[2], " \n");
    await assert.rejects(generateManifest(metadata, packages, output), /Empty signature/);
    await rm(signaturePaths[2]);
    await assert.rejects(generateManifest(metadata, packages, output), /local artifact/);
    await writeFile(signaturePaths[2], "signature-2");
    const duplicate = join(packages, names[0]);
    await writeFile(duplicate, "payload");
    await assert.rejects(generateManifest(metadata, packages, output), /found 2/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
