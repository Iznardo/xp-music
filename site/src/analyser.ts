// Browser port of src-tauri/src/viz/bands.rs: turns the data from an AnalyserNode
// (dB spectrum and waveform) into the VizData the app's visualizations expect.
// No browser dependencies, so it can be tested with Node (site/analyser.test.mjs).

import type { VizData } from "../../src/lib/viz/types";

const BANDS = 64;
const WAVE = 256;
const MIN_HZ = 20;
const MAX_HZ = 16_000;
const FLOOR_DB = -70;
const ATTACK = 0.6;
const RELEASE = 0.15;
const PEAK_FALL = 0.01;
const BEAT_LO_HZ = 40;
const BEAT_HI_HZ = 160;
const BEAT_JUMP = 1.45;
const BEAT_GAP = 7;
/** The AnalyserNode scales the FFT by 1/N and uses a Blackman window: a full-scale
 *  sine ends up ~13 dB lower than with the app's normalization. */
export const DB_OFFSET = 13;

export class Analyzer {
  bands: number[] = new Array(BANDS).fill(0);
  peaks: number[] = new Array(BANDS).fill(0);
  wave: number[] = new Array(WAVE).fill(0);
  level = 0;
  beat = 0;
  bass = 0;
  private ranges: [number, number][];
  private bassBins: [number, number];
  private bassAvg = 0;
  private bassMax = 1e-6;
  private sinceBeat = BEAT_GAP;

  constructor(sampleRate: number, fftSize: number) {
    const binHz = sampleRate / fftSize;
    const maxBin = fftSize / 2;
    const ratio = (MAX_HZ / MIN_HZ) ** (1 / BANDS);
    this.ranges = Array.from({ length: BANDS }, (_, b) => {
      const fLo = MIN_HZ * ratio ** b;
      const lo = Math.min(Math.max(Math.round(fLo / binHz), 1), maxBin - 1);
      const hi = Math.min(Math.max(Math.round((fLo * ratio) / binHz), lo + 1), maxBin);
      return [lo, hi];
    });
    const lo = Math.max(Math.round(BEAT_LO_HZ / binHz), 1);
    this.bassBins = [lo, Math.max(Math.round(BEAT_HI_HZ / binHz) + 1, lo + 1)];
  }

  /** `freqDb`: getFloatFrequencyData (fftSize/2). `time`: getFloatTimeDomainData (fftSize). */
  analyze(freqDb: Float32Array, time: Float32Array): VizData {
    for (let b = 0; b < BANDS; b++) {
      const [lo, hi] = this.ranges[b]!;
      let db = -Infinity;
      for (let i = lo; i < hi; i++) db = Math.max(db, freqDb[i]!);
      const target = Math.min(1, Math.max(0, (db + DB_OFFSET - FLOOR_DB) / -FLOOR_DB));
      const v = this.bands[b]!;
      this.bands[b] = v + (target - v) * (target > v ? ATTACK : RELEASE);
      this.peaks[b] = Math.max(this.peaks[b]! - PEAK_FALL, this.bands[b]!);
    }

    const start = time.length - WAVE * 4;
    let sumSq = 0;
    for (const x of time) sumSq += x * x;
    for (let i = 0; i < WAVE; i++) this.wave[i] = time[start + i * 4]!;
    this.level += (Math.min(1, Math.sqrt(sumSq / time.length) * 2) - this.level) * 0.3;

    // Beat: jump in bass energy over its recent average (as in the app).
    let e = 0;
    for (let i = this.bassBins[0]; i < this.bassBins[1]; i++) e += 10 ** ((freqDb[i]! + DB_OFFSET) / 10);
    this.bassMax = Math.max(this.bassMax * 0.996, e, 1e-6);
    this.bass = Math.min(1, Math.sqrt(e / this.bassMax));
    const avg = this.bassAvg;
    this.bassAvg = avg * 0.92 + e * 0.08;
    this.sinceBeat++;
    if (this.sinceBeat >= BEAT_GAP && e > avg * BEAT_JUMP && e > this.bassMax * 0.2) {
      this.beat = 1;
      this.sinceBeat = 0;
    } else {
      this.beat *= 0.8;
    }
    return this;
  }
}
