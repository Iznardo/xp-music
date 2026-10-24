// node --test site/analyser.test.mjs (Node 24 quita los tipos de analyser.ts al importarlo).
import { test } from "node:test";
import assert from "node:assert/strict";
import { Analyzer } from "./src/analyser.ts";

const RATE = 48_000;
const N = 2048;
const silence = () => new Float32Array(N / 2).fill(-140);
const time = new Float32Array(N);
const binOf = (hz) => Math.round(hz / (RATE / N));

test("un tono de 1 kHz enciende su banda y no las graves", () => {
  const a = new Analyzer(RATE, N);
  const f = silence();
  f[binOf(1000)] = -15;
  for (let i = 0; i < 20; i++) a.analyze(f, time);
  const top = a.bands.indexOf(Math.max(...a.bands));
  // 64 bandas logarítmicas de 20 Hz a 16 kHz: 1 kHz cae hacia la 37.
  assert.ok(top >= 35 && top <= 39, `banda ${top}`);
  assert.ok(a.bands[top] > 0.8);
  assert.equal(a.bands[0], 0);
});

test("un golpe de graves tras silencio marca el ritmo y luego cae", () => {
  const a = new Analyzer(RATE, N);
  for (let i = 0; i < 10; i++) a.analyze(silence(), time);
  const kick = silence();
  for (let hz = 50; hz <= 150; hz += 20) kick[binOf(hz)] = -10;
  a.analyze(kick, time);
  assert.equal(a.beat, 1);
  assert.ok(a.bass > 0.9);
  a.analyze(silence(), time);
  assert.ok(a.beat < 1);
});
