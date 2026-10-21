// Varias visualizaciones pueden convivir un instante (al cambiar de preset se
// monta la nueva antes de desmontar la vieja, y una máscara puede tener la suya):
// el analizador del backend se enciende con la primera visible y se apaga cuando
// no queda ninguna.

import { commands } from "../bindings";

let visible = 0;

export function vizVisible(delta: 1 | -1) {
  const before = visible;
  visible = Math.max(0, visible + delta);
  if (before === 0 && visible > 0) void commands.vizSetActive(true);
  if (before > 0 && visible === 0) void commands.vizSetActive(false);
}
