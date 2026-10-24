// node --test site/analyser.test.mjs (Node 24 strips the types from analyser.ts when importing it).
import { test } from "node:test";
import assert from "node:assert/strict";
import { Analyzer } from "./src/analyser.ts";

const RATE = 48_000;
const N = 2048;
const silence = () => new Float32Array(N / 2).fill(-140);
const time = new Float32Array(N);
const binOf = (hz) => Math.round(hz / (RATE / N));

test("a 1 kHz tone lights up its band and not the bass", () => {
  const a = new Analyzer(RATE, N);
  const f = silence();
  f[binOf(1000)] = -15;
  for (let i = 0; i < 20; i++) a.analyze(f, time);
  const top = a.bands.indexOf(Math.max(...a.bands));
  // 64 logarithmic bands from 20 Hz to 16 kHz: 1 kHz lands around band 37.
  assert.ok(top >= 35 && top <= 39, `band ${top}`);
  assert.ok(a.bands[top] > 0.8);
  assert.equal(a.bands[0], 0);
});

test("a bass hit after silence marks the beat and then decays", () => {
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
