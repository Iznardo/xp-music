import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { END, START, composeNotes, generateNotes, stripDownloads } from "./release-notes.mjs";
import { ARTIFACTS } from "./release-targets.mjs";

const { version } = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const sha = (text) => createHash("sha256").update(text).digest("hex");

test("composeNotes is idempotent and keeps only the maintainer notes under the table", () => {
  const rows = ARTIFACTS.map((artifact, index) => ({ artifact, url: `https://example.test/XP.Music_${index}.bin`, sha256: "a".repeat(64) }));
  const once = composeNotes("Arreglos varios", rows);
  assert.equal(composeNotes(once, rows), once);
  assert.ok(once.startsWith(`${START}\n## Descargas\n`));
  assert.ok(once.endsWith(`${END}\n\nArreglos varios\n`));
  assert.equal(stripDownloads(once), "Arreglos varios");
  assert.equal(stripDownloads(null), "");
  assert.equal(composeNotes("", rows).endsWith(`${END}\n`), true);
  const [header, separator, first] = once.split("\n").slice(3, 6);
  assert.equal(header, "| Sistema | SHA-256 | Descarga |");
  assert.equal(separator, "| --- | --- | --- |");
  assert.equal(first, `| Windows x64 | \`${"a".repeat(64)}\` | [XP.Music_0.bin](https://example.test/XP.Music_0.bin) |`);
  // Los archivos que solo usa el updater no se listan.
  assert.ok(!once.includes("app.tar.gz") && !once.includes("<details>") && !once.includes("Actualizador"));
  assert.equal(once.split("\n").filter((line) => /^\| (Windows|Linux|macOS) /.test(line)).length, ARTIFACTS.filter((a) => a.installer).length);
  assert.throws(() => stripDownloads(`${START}\nsin cierre`), /incompleto/);
  assert.throws(() => stripDownloads(`${END}${START}`), /incompleto/);
});

test("generateNotes hashes local files, uses uploaded URLs and fails closed", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xp-music-notes-"));
  const packages = join(directory, "packages");
  const metadata = join(directory, "release.json");
  const output = join(directory, "notes.md");
  try {
    const release = { tag_name: `v${version}`, draft: true, prerelease: false, body: "Mis notas", assets: [] };
    for (const [index, artifact] of ARTIFACTS.entries()) {
      const name = `XP Music${artifact.suffix(version)}`;
      const nested = join(packages, String(index));
      await mkdir(nested, { recursive: true });
      await writeFile(join(nested, name), `payload-${index}`);
      const sanitized = name.replaceAll(" ", ".");
      release.assets.push({
        name: sanitized, size: `payload-${index}`.length, digest: `sha256:${sha(`payload-${index}`)}`,
        browser_download_url: `https://github.com/Iznardo/xp-music/releases/download/v${version}/${sanitized}`,
      });
    }
    await writeFile(metadata, JSON.stringify(release));
    const notes = await generateNotes(metadata, packages, output);
    assert.equal(await readFile(output, "utf8"), notes);
    assert.ok(notes.includes(`| Linux ARM64 | \`${sha("payload-6")}\` | [${release.assets[6].name}](${release.assets[6].browser_download_url}) |`));
    assert.ok(notes.includes(`| macOS Apple Silicon | \`${sha("payload-9")}\` | [${release.assets[9].name}](${release.assets[9].browser_download_url}) |`));
    assert.ok(!notes.includes("app.tar.gz"));
    assert.ok(notes.endsWith("\nMis notas\n"));
    // Regenerar sobre unas notas ya publicadas da el mismo resultado.
    release.body = notes;
    await writeFile(metadata, JSON.stringify(release));
    assert.equal(await generateNotes(metadata, packages, output), notes);

    async function rejects(change, pattern) {
      const invalid = structuredClone(release);
      change(invalid);
      await writeFile(metadata, JSON.stringify(invalid));
      await assert.rejects(generateNotes(metadata, packages, output), pattern);
    }
    await rejects((r) => { r.tag_name = "v0.0.0"; }, /Expected release tag/);
    await rejects((r) => { r.assets.splice(3, 1); }, /remote artifact/);
    await rejects((r) => { r.assets[3].size++; }, /size mismatch/);
    await rejects((r) => { r.assets[3].digest = `sha256:${"0".repeat(64)}`; }, /SHA-256 mismatch/);
    await rejects((r) => { r.assets[3].browser_download_url += "?x=1"; }, /Unsafe/);
    await rm(join(packages, "3"), { recursive: true });
    await writeFile(metadata, JSON.stringify(release));
    await assert.rejects(generateNotes(metadata, packages, output), /local artifact/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
