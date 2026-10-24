// Visualizaciones de la app que se muestran en la portada (sin «Portada del álbum»).

import { Bars, Waves } from "../../src/lib/viz/canvas2d";
import { Alchemy, Ambience, Battery, Spiral, Water } from "../../src/lib/viz/webgl";
import type { VizRenderer } from "../../src/lib/viz/types";
import { t } from "../../src/lib/strings";

export const PRESETS: { name: string; make: (c: HTMLCanvasElement) => VizRenderer }[] = [
  { name: t.options.vizNames.Water, make: (c) => new Water(c) },
  { name: t.options.vizNames.Alchemy, make: (c) => new Alchemy(c) },
  { name: t.options.vizNames.Spiral, make: (c) => new Spiral(c) },
  { name: t.options.vizNames.Ambience, make: (c) => new Ambience(c) },
  { name: t.options.vizNames.Battery, make: (c) => new Battery(c) },
  { name: t.options.vizNames.Bars, make: (c) => new Bars(c) },
  { name: t.options.vizNames.Waves, make: (c) => new Waves(c) },
];
