// Order of the visualizations (◀ ▶ in Now Playing, click on the stage, and Options).

import type { Settings, VizPreset } from "../bindings";

const VIZ_ORDER: VizPreset[] = ["Water", "Alchemy", "Spiral", "Ambience", "Battery", "Bars", "Waves", "Cover"];

/// One stop of the ◀ ▶ cycle: what to write in the settings to select it.
export type VizEntry = { id: string; patch: Pick<Settings, "visualization" | "customVisualization"> };

/// Built-in presets, then the user's, with "Album art" always last.
export function vizEntries(custom: string[]): VizEntry[] {
  const preset = (p: VizPreset): VizEntry => ({ id: p, patch: { visualization: p, customVisualization: null } });
  return [
    ...VIZ_ORDER.filter((p) => p !== "Cover").map(preset),
    ...custom.map((n) => ({ id: `custom:${n}`, patch: { customVisualization: n } as VizEntry["patch"] })),
    preset("Cover"),
  ];
}
