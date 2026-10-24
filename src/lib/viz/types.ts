// Common contract for the visualization renderers (specs/005/plan.md).

import type { VizFrame } from "../bindings";

/// Already-normalized frame (specta types `f32` as `number | null`).
export type VizData = { bands: number[]; peaks: number[]; wave: number[]; level: number; beat: number; bass: number };

/// The backend sends bands and peaks as bytes 0..255 and the wave as -127..127.
export function toData(f: VizFrame): VizData {
  const n = (x: number | null) => x ?? 0;
  const u = (x: number) => x / 255;
  return { bands: f.bands.map(u), peaks: f.peaks.map(u), wave: f.wave.map((x) => x / 127), level: n(f.level), beat: n(f.beat), bass: n(f.bass) };
}

export interface VizRenderer {
  /// Draws a frame. `t` in seconds (for continuous animations).
  draw(frame: VizData, t: number): void;
  resize(width: number, height: number): void;
  dispose(): void;
  /// Whether it needs to animate even when no new data arrives (WebGL with trails).
  readonly continuous: boolean;
}

export const EMPTY_FRAME: VizData = {
  bands: new Array(64).fill(0),
  peaks: new Array(64).fill(0),
  wave: new Array(256).fill(0),
  level: 0,
  beat: 0,
  bass: 0,
};
