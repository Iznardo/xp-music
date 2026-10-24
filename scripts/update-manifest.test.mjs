import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { generateManifest } from "./update-manifest.mjs";

const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));

// Nombres tal y como los genera Tauri (contrato escrito a mano, no sacado de la tabla).
const payloads = [
  [`XP Music_${version}_x64-setup.exe`, ["windows-x86_64-nsis", "windows-x86_64"]],
  [`XP Music_${version}_arm64-setup.exe`, ["windows-aarch64-nsis", "windows-aarch64"]],
  [`XP Music_${version}_amd64.deb`, ["linux-x86_64-deb"]],
  [`XP Music_${version}_amd64.AppImage`, ["linux-x86_64-appimage"]],
  [`XP Music-${version}-1.x86_64.rpm`, ["linux-x86_64-rpm"]],
  [`XP Music_${version}_arm64.deb`, ["linux-aarch64-deb"]],
  [`XP Music_${version}_aarch64.AppImage`, ["linux-aarch64-appimage"]],
  [`XP Music-${version}-1.aarch64.rpm`, ["linux-aarch64-rpm"]],
];

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
    const signaturePaths = [];
    for (const [index, [name]] of payloads.entries()) {
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
    // Instaladores sin firma de updater (.dmg) y notas con el bloque generado: no cuentan.
    release.assets.push({ name: `XP.Music_${version}_x64.dmg`, size: 1,
      browser_download_url: `https://github.com/Iznardo/xp-music/releases/download/v${version}/XP.Music_${version}_x64.dmg` });
    release.body = "<!-- xp-music:downloads:start -->\n## Descargas\n<!-- xp-music:downloads:end -->\n\nExisting release notes\n";
    await writeFile(metadata, JSON.stringify(release));
    // A different cwd must not change where package.json is read.
    execFileSync(process.execPath, [fileURLToPath(new URL("./update-manifest.mjs", import.meta.url)),
      metadata, packages, output], { cwd: directory });
    const manifest = JSON.parse(await readFile(output, "utf8"));
    assert.equal(manifest.version, version);
    assert.equal(manifest.notes, "Existing release notes");
    assert.equal(manifest.pub_date, release.created_at);
    assert.deepEqual(Object.keys(manifest.platforms).sort(), payloads.flatMap(([, keys]) => keys).sort());
    for (const [index, [, keys]] of payloads.entries()) {
      for (const key of keys) {
        assert.deepEqual(manifest.platforms[key], {
          signature: `signature-${index}`, url: release.assets[index * 2].browser_download_url,
        });
      }
    }
    release.body = "Existing release notes";

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
    // Faltan las entradas de Windows ARM64 o Linux ARM64: no se publica un manifiesto parcial.
    await rejectsRelease((r) => { r.assets.splice(2, 2); }, /remote artifact/);
    await rejectsRelease((r) => { r.assets.splice(14, 2); }, /remote artifact/);
    await rejectsRelease((r) => { r.assets.push(r.assets[0]); }, /found 2/);
    await rejectsRelease((r) => { r.assets.push(r.assets[1]); }, /found 2/);
    await rejectsRelease((r) => { r.assets[0].size++; }, /size mismatch/);
    await rejectsRelease((r) => { r.assets[1].size = 0; }, /empty uploaded signature/);
    await rejectsRelease((r) => { r.body = "<!-- xp-music:downloads:start -->"; }, /incompleto/);
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
    const duplicate = join(packages, payloads[0][0]);
    await writeFile(duplicate, "payload");
    await assert.rejects(generateManifest(metadata, packages, output), /found 2/);
    await rm(duplicate);
    // Sin el paquete local de Linux ARM64 tampoco se genera.
    await rm(join(packages, "7", "bundle", payloads[7][0]));
    await assert.rejects(generateManifest(metadata, packages, output), /local artifact/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
