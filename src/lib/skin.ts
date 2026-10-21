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
  /// Altura a la que el borde izquierdo de la lateral empieza a curvarse hacia
  /// arriba, en S, hasta fundirse con el borde superior del cuerpo.
  sideTop: 84,
  /// El cuerpo tapa el borde derecho de la lateral.
  sideOverlap: 10,
  /// La pestaña de la lista sobresale esto por encima del cuerpo.
  tabRise: 22,
  tabW: 286,
  tabH: 44,
  /// El borde superior del cuerpo sube en S hasta la pestaña: empieza esto antes
  /// de la pestaña y llega arriba esto dentro de ella.
  tabSlope: 90,
  tabInset: 34,
  /// Franja superior del cuerpo (botón de menú y botones redondos).
  topBar: 28,
  /// Hueco bajo el cuerpo: solo la píldora llega abajo del todo.
  bottomGap: 26,
  /// Barra inferior del cuerpo (controles a la derecha de la píldora).
  bottomBar: 58,
  pod: { dx: 22, w: 360, h: 88 },
  frame: 7,
  /// Radio de los empalmes cóncavos donde se unen las piezas.
  fillet: 11,
} as const;

export type SkinLayout = {
  side: Piece | null;
  /// Recorte de la lateral (coordenadas locales) con su curva en S.
  sideClip: string | null;
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
  // La lateral ocupa desde arriba del cuerpo; su silueta la da `sideClip`.
  const side: Piece | null = sideOpen
    ? { x: 0, y: bodyY, w: s.sideW, h: bodyBottom - bodyY, r: [0, 0, 0, 30] }
    : null;
  const body: Piece = { x: bodyX, y: bodyY, w: w - bodyX, h: bodyBottom - bodyY, r: [sideOpen ? 0 : 16, 0, 0, 0] };
  // La pestaña empieza donde la curva ya ha llegado arriba.
  const tab: Piece = { x: w - s.tabW + s.tabInset, y: 0, w: s.tabW - s.tabInset, h: s.tabH, r: [0, 12, 0, 0] };
  const pod: Piece = { x: podX, y: podY, w: s.pod.w, h: s.pod.h, r: [s.pod.h / 2, s.pod.h / 2, s.pod.h / 2, s.pod.h / 2] };
  const contentTop = bodyY + s.topBar;
  const contentBottom = bodyBottom - s.bottomBar;
  let sideClip: string | null = null;
  if (side) {
    const c = sideCurve(bodyX, bodyY);
    const t = (y: number) => y - bodyY;
    const r3 = side.r[3];
    sideClip = [
      `M ${r3} ${side.h}`,
      `A ${r3} ${r3} 0 0 1 0 ${side.h - r3}`,
      `L 0 ${t(c.p0[1])}`,
      `C ${c.c1[0]} ${t(c.c1[1])} ${c.c2[0]} ${t(c.c2[1])} ${c.p3[0]} ${t(c.p3[1])}`,
      `L ${side.w} 0`,
      `L ${side.w} ${side.h}`,
      "Z",
    ].join(" ");
  }
  return {
    side,
    sideClip,
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

/// Curva en S de la lateral: del borde izquierdo (vertical) al borde superior del
/// cuerpo (horizontal), convexa al salir y cóncava al llegar.
function sideCurve(bodyX: number, bodyY: number) {
  return {
    p0: [0, SKIN.sideTop] as const,
    c1: [0, bodyY + (SKIN.sideTop - bodyY) * 0.15] as const,
    c2: [bodyX * 0.45, bodyY] as const,
    p3: [bodyX, bodyY] as const,
  };
}

/// Empalme cóncavo entre la recta horizontal `y = line` (la forma queda encima) y
/// una circunferencia que la atraviesa y sobresale por debajo: devuelve el punto
/// de tangencia en la recta y en la circunferencia. `side` = -1 a la izquierda.
function lineCircleFillet(line: number, cx: number, cy: number, r: number, f: number, side: 1 | -1) {
  const dy = line + f - cy;
  const fx = cx + side * Math.sqrt((r + f) ** 2 - dy ** 2);
  const k = r / (r + f);
  return { onLine: [fx, line] as const, onCircle: [cx + (fx - cx) * k, cy + (line + f - cy) * k] as const };
}

/// Contorno exterior completo: la unión de las piezas con empalmes cóncavos en
/// cada unión (lateral–cuerpo, cuerpo–pestaña y a ambos lados de la píldora), para
/// que se lean como una sola carcasa. Se pinta detrás de las piezas y es también
/// la región de la ventana.
export function outlinePath(l: SkinLayout): string {
  const F = SKIN.fillet;
  const { body, tab, pod, side } = l;
  const bx = body.x;
  const by = body.y;
  const bb = body.y + body.h;
  const right = body.x + body.w;
  const pr = pod.h / 2;
  const cy = pod.y + pr;
  const cl = pod.x + pr;
  const cr = pod.x + pod.w - pr;
  const fR = lineCircleFillet(bb, cr, cy, pr, F, 1);
  const fL = lineCircleFillet(bb, cl, cy, pr, F, -1);
  const n = (v: number) => +v.toFixed(2);
  const d: string[] = [];
  // Curva en S del cuerpo hacia la pestaña.
  const s0 = tab.x - SKIN.tabInset - SKIN.tabSlope;
  const s3 = tab.x;
  const sm = (s3 - s0) * 0.5;
  if (side) {
    const c = sideCurve(bx, by);
    d.push(`M 0 ${c.p0[1]}`, `C ${c.c1[0]} ${n(c.c1[1])} ${n(c.c2[0])} ${c.c2[1]} ${c.p3[0]} ${c.p3[1]}`);
  } else {
    d.push(`M 0 ${by + body.r[0]}`, `A ${body.r[0]} ${body.r[0]} 0 0 1 ${body.r[0]} ${by}`);
  }
  d.push(
    `L ${s0} ${by}`,
    `C ${s0 + sm} ${by} ${s3 - sm} 0 ${s3} 0`,
    `L ${right - tab.r[1]} 0`,
    `A ${tab.r[1]} ${tab.r[1]} 0 0 1 ${right} ${tab.r[1]}`,
    // Esquina inferior derecha recta (agarre de tamaño).
    `L ${right} ${bb}`,
    // Cuerpo → píldora (derecha), la píldora por debajo, píldora → cuerpo (izquierda).
    `L ${n(fR.onLine[0])} ${bb}`,
    `A ${F} ${F} 0 0 0 ${n(fR.onCircle[0])} ${n(fR.onCircle[1])}`,
    `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
    `L ${cl} ${pod.y + pod.h}`,
    `A ${pr} ${pr} 0 0 1 ${n(fL.onCircle[0])} ${n(fL.onCircle[1])}`,
    `A ${F} ${F} 0 0 0 ${n(fL.onLine[0])} ${bb}`,
  );
  if (side) {
    const r3 = side.r[3];
    d.push(`L ${r3} ${bb}`, `A ${r3} ${r3} 0 0 1 0 ${bb - r3}`);
  } else {
    d.push(`L ${bx} ${bb}`);
  }
  d.push("Z");
  return d.join(" ");
}

/// Estilo CSS de posición para un rectángulo.
export function place(r: Rect): string {
  return `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;
}

export function radii(p: Piece): string {
  return p.r.map((x) => `${x}px`).join(" ");
}
