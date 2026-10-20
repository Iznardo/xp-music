// Silueta de la ventana principal (skin WMP 9): no es un rectángulo con esquinas
// redondeadas sino un contorno propio. La ventana es transparente y se recorta con
// `clip-path: path(...)`; el mismo trazado se dibuja encima como borde.
//
//   ╭─╮ ─────────────────────────────────╮   oreja redonda arriba a la izquierda (logo/menú)
//   ╰╮                                    │
//    )  panel de tareas (borde convexo)    │
//   ╭╯                                     │
//  (▶)──────────────────────────────╮     │   disco del botón ▶ que sobresale abajo a la izquierda
//   ╰─╯                               ╰───╯   curva larga abajo a la derecha

export const SHAPE = {
  /// Franja transparente sobre la barra de título (solo la oreja llega arriba del todo).
  top: 8,
  /// Margen izquierdo del cuerpo; el borde se curva hacia fuera entre oreja y disco.
  left: 14,
  /// Margen inferior del transporte; solo el disco llega abajo del todo.
  bottom: 10,
  ear: { cx: 22, cy: 22, r: 22 },
  /// Disco del ▶: centro relativo a la esquina inferior izquierda.
  pod: { cx: 50, fromBottom: 48, r: 48 },
  /// Altura a la que empieza la curva de abajo a la derecha.
  tail: 64,
} as const;

export function shellPath(w: number, h: number): string {
  const { top, left: l, bottom, ear, pod } = SHAPE;
  const b = h - bottom;
  const podCy = h - pod.fromBottom;
  // Cortes de las circunferencias con los bordes rectos.
  const earTopX = ear.cx + Math.sqrt(ear.r ** 2 - (top - ear.cy) ** 2);
  const earLeftY = ear.cy + Math.sqrt(ear.r ** 2 - (l - ear.cx) ** 2);
  const podBottomX = pod.cx + Math.sqrt(pod.r ** 2 - (b - podCy) ** 2);
  const podLeftY = podCy - Math.sqrt(pod.r ** 2 - (l - pod.cx) ** 2);
  const r = 12;
  const f = (n: number) => n.toFixed(1);
  return [
    `M ${f(earTopX)} ${top}`,
    `L ${w - r} ${top}`,
    `Q ${w} ${top} ${w} ${top + r}`,
    `L ${w} ${f(b - SHAPE.tail)}`,
    // Abajo a la derecha el transporte cae en una curva larga, no en una esquina.
    `C ${w} ${f(b - 14)} ${w - 70} ${b} ${w - 200} ${b}`,
    `L ${f(podBottomX)} ${b}`,
    `A ${pod.r} ${pod.r} 0 0 1 ${l} ${f(podLeftY)}`,
    // Borde izquierdo abombado hacia fuera.
    `Q ${l - 12} ${f((podLeftY + earLeftY) / 2)} ${l} ${f(earLeftY)}`,
    `A ${ear.r} ${ear.r} 0 1 1 ${f(earTopX)} ${top}`,
    "Z",
  ].join(" ");
}
