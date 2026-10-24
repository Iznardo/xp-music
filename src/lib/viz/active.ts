// Several visualizations can coexist for a moment (when changing preset the new
// one is mounted before the old one is unmounted, and a skin can have its own):
// the backend analyzer turns on with the first visible one and off when none
// is left.

import { commands } from "../bindings";

let visible = 0;

export function vizVisible(delta: 1 | -1) {
  const before = visible;
  visible = Math.max(0, visible + delta);
  if (before === 0 && visible > 0) void commands.vizSetActive(true);
  if (before > 0 && visible === 0) void commands.vizSetActive(false);
}
