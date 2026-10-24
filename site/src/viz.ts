// App visualizations shown on the home page (without "Album art"). Names come from t.vizNames.

import { Bars, Waves } from "../../src/lib/viz/canvas2d";
import { Alchemy, Ambience, Battery, Spiral, Water } from "../../src/lib/viz/webgl";
import type { VizRenderer } from "../../src/lib/viz/types";

export const PRESETS: { id: "Water" | "Alchemy" | "Spiral" | "Ambience" | "Battery" | "Bars" | "Waves"; make: (c: HTMLCanvasElement) => VizRenderer }[] = [
  { id: "Water", make: (c) => new Water(c) },
  { id: "Alchemy", make: (c) => new Alchemy(c) },
  { id: "Spiral", make: (c) => new Spiral(c) },
  { id: "Ambience", make: (c) => new Ambience(c) },
  { id: "Battery", make: (c) => new Battery(c) },
  { id: "Bars", make: (c) => new Bars(c) },
  { id: "Waves", make: (c) => new Waves(c) },
];
