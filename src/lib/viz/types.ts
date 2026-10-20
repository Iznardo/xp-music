// Contrato común de los renderizadores de visualización (specs/005/plan.md).

import type { VizFrame } from "../bindings";

/// Fotograma ya normalizado (specta tipa los `f32` como `number | null`).
export type VizData = { bands: number[]; peaks: number[]; wave: number[]; level: number; beat: number; bass: number };

export function toData(f: VizFrame): VizData {
  const n = (x: number | null) => x ?? 0;
  return { bands: f.bands.map(n), peaks: f.peaks.map(n), wave: f.wave.map(n), level: n(f.level), beat: n(f.beat), bass: n(f.bass) };
}

export interface VizRenderer {
  /// Dibuja un fotograma. `t` en segundos (para animaciones continuas).
  draw(frame: VizData, t: number): void;
  resize(width: number, height: number): void;
  dispose(): void;
  /// Si necesita animarse aunque no lleguen datos nuevos (WebGL con rastro).
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
