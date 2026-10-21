// Geometría del skin WMP 9 (specs/008). La ventana no es un rectángulo: es la
// unión de piezas que se superponen, como en el reproductor original:
//
//            ╭──────── pestaña de la lista (sobresale arriba) ─╮
//   ╭────────┴─ cuerpo plateado ───────────────────────────────┤
//  ╭┴╮ lateral                                                 │
//  │ │ (empieza más abajo,  [ visualización ]  [ lista ]       │
//  │ │  esquinas redondas)                                     │
//  ╰┬╯                                                         │
//   ╰─(  píldora de controles  )── barra inferior ─────────── ◢┘ ← esquina recta con agarre
//      (sobresale por debajo)
//
// Todo en píxeles CSS a partir del tamaño de la ventana.

export type Rect = { x: number; y: number; w: number; h: number };
/// Radios por esquina: arriba-izq, arriba-dcha, abajo-dcha, abajo-izq.
export type Piece = Rect & { r: [number, number, number, number] };

export const SKIN = {
  sideW: 118,
  /// La lateral empieza más abajo que el cuerpo.
  sideTop: 46,
  /// El cuerpo tapa el borde derecho de la lateral.
  sideOverlap: 10,
  /// La pestaña de la lista sobresale esto por encima del cuerpo.
  tabRise: 16,
  tabW: 286,
  tabH: 44,
  /// Franja superior del cuerpo (botón de menú y botones redondos).
  topBar: 28,
  /// Hueco bajo el cuerpo: solo la píldora llega abajo del todo.
  bottomGap: 26,
  /// Barra inferior del cuerpo (controles a la derecha de la píldora).
  bottomBar: 58,
  pod: { dx: 22, w: 360, h: 88 },
  frame: 7,
} as const;

export type SkinLayout = {
  side: Piece | null;
  body: Piece;
  tab: Piece;
  pod: Piece;
  /// Zona negra de contenido (vistas).
  content: Rect;
  /// Franja superior del cuerpo, a la izquierda de la pestaña.
  topBar: Rect;
  /// Zona de la barra inferior a la derecha de la píldora.
  bar: Rect;
};

export function skinLayout(w: number, h: number, sideOpen: boolean): SkinLayout {
  const s = SKIN;
  const bodyX = sideOpen ? s.sideW - s.sideOverlap : 0;
  const bodyY = s.tabRise;
  const bodyBottom = h - s.bottomGap;
  const podY = h - s.pod.h;
  const podX = bodyX + s.pod.dx;
  const side: Piece | null = sideOpen
    ? { x: 0, y: s.sideTop, w: s.sideW, h: podY + 30 - s.sideTop, r: [24, 0, 0, 30] }
    : null;
  const body: Piece = { x: bodyX, y: bodyY, w: w - bodyX, h: bodyBottom - bodyY, r: [16, 0, 0, 0] };
  const tab: Piece = { x: w - s.tabW, y: 0, w: s.tabW, h: s.tabH, r: [12, 12, 0, 0] };
  const pod: Piece = { x: podX, y: podY, w: s.pod.w, h: s.pod.h, r: [46, 46, 46, 46] };
  const contentTop = bodyY + s.topBar;
  const contentBottom = bodyBottom - s.bottomBar;
  return {
    side,
    body,
    tab,
    pod,
    content: {
      x: bodyX + (sideOpen ? s.sideOverlap : s.frame),
      y: contentTop,
      w: w - bodyX - (sideOpen ? s.sideOverlap : s.frame) - s.frame,
      h: contentBottom - contentTop,
    },
    topBar: { x: bodyX, y: bodyY, w: w - s.tabW - bodyX, h: s.topBar },
    bar: { x: podX + s.pod.w, y: contentBottom, w: w - podX - s.pod.w, h: bodyBottom - contentBottom },
  };
}

/// Estilo CSS de posición para un rectángulo.
export function place(r: Rect): string {
  return `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;
}

export function radii(p: Piece): string {
  return p.r.map((x) => `${x}px`).join(" ");
}

/// Polígono de un rectángulo con esquinas redondeadas (en píxeles físicos).
function roundedPoly(p: Piece, scale: number): [number, number][] {
  const pts: [number, number][] = [];
  const corners: [number, number, number, number][] = [
    // centro x, centro y, radio, ángulo inicial (grados)
    [p.x + p.w - p.r[1], p.y + p.r[1], p.r[1], -90],
    [p.x + p.w - p.r[2], p.y + p.h - p.r[2], p.r[2], 0],
    [p.x + p.r[3], p.y + p.h - p.r[3], p.r[3], 90],
    [p.x + p.r[0], p.y + p.r[0], p.r[0], 180],
  ];
  for (const [cx, cy, r, a0] of corners) {
    const steps = r > 0 ? Math.max(4, Math.round(r / 3)) : 0;
    for (let i = 0; i <= steps; i++) {
      const a = ((a0 + (90 * i) / Math.max(steps, 1)) * Math.PI) / 180;
      pts.push([Math.round((cx + r * Math.cos(a)) * scale), Math.round((cy + r * Math.sin(a)) * scale)]);
    }
  }
  return pts;
}

/// Región de la ventana: un polígono por pieza (Windows hace la unión).
export function regionPolygons(l: SkinLayout, scale: number): [number, number][][] {
  return [l.side, l.body, l.tab, l.pod].filter((p): p is Piece => p !== null).map((p) => roundedPoly(p, scale));
}
