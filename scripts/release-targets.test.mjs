import assert from "node:assert/strict";
import test from "node:test";
import { downloadUrl } from "./release-targets.mjs";

const asset = (url, name = "XP.Music_1.0.2_x64-setup.exe") => ({ name, browser_download_url: url });
const base = "https://github.com/Iznardo/xp-music/releases/download";

test("un draft usa un tag provisional y se devuelve la URL definitiva", () => {
  const name = "XP.Music_1.0.2_x64-setup.exe";
  assert.equal(downloadUrl(asset(`${base}/untagged-0123abcdef/${name}`), "v1.0.2"), `${base}/v1.0.2/${name}`);
  assert.equal(downloadUrl(asset(`${base}/v1.0.2/${name}`), "v1.0.2"), `${base}/v1.0.2/${name}`);
});

test("rechaza URLs que no son de descarga de esa release", () => {
  const name = "XP.Music_1.0.2_x64-setup.exe";
  for (const url of [
    `${base}/v0.0.0/${name}`,
    `${base}/untagged-XYZ/${name}`,
    `${base}/untagged-/${name}`,
    `${base}/untagged-0123abcdef/otro.exe`,
    `http://github.com/Iznardo/xp-music/releases/download/v1.0.2/${name}`,
    `https://github.com.evil.test/Iznardo/xp-music/releases/download/v1.0.2/${name}`,
    `${base}/v1.0.2/${name}?redirect=evil`,
  ]) {
    assert.throws(() => downloadUrl(asset(url), "v1.0.2"), /Unsafe/, url);
  }
});
