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
  /// Borde superior de la lateral: va recto a esta altura y luego sube en S
  /// (como el de la pestaña) hasta el borde superior del cuerpo.
  sideTop: 48,
  sideCorner: 12,
  /// Tramo de la subida en S de la lateral (x de inicio y de llegada).
  sideSlope: { from: 40, to: 160 },
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
  /// Con lateral, su parte de abajo se estrecha en S durante esta altura hasta
  /// la esquina inferior izquierda de la píldora (lateral y controles, una pieza).
  taper: 110,
  /// Radio de la esquina inferior izquierda de la píldora unida a la lateral.
  podCorner: 40,
  /// Empalme de la píldora con el cuerpo, por la derecha.
  podFillet: 18,
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
  // Con lateral, la píldora arranca en el borde del cuerpo: su lado izquierdo es
  // la continuación de la lateral.
  const podX = sideOpen ? bodyX : bodyX + s.pod.dx;
  const podRight = bodyX + s.pod.dx + s.pod.w;
  // La lateral ocupa desde arriba del cuerpo; su silueta la da `sideClip`.
  const side: Piece | null = sideOpen
    ? { x: 0, y: bodyY, w: s.sideSlope.to, h: podY - bodyY, r: [0, 0, 0, 0] }
    : null;
  const body: Piece = { x: bodyX, y: bodyY, w: w - bodyX, h: bodyBottom - bodyY, r: [sideOpen ? 0 : 16, 0, 0, 0] };
  // La pestaña empieza donde la curva ya ha llegado arriba.
  const tab: Piece = { x: w - s.tabW + s.tabInset, y: 0, w: s.tabW - s.tabInset, h: s.tabH, r: [0, 12, 0, 0] };
  const pr = s.pod.h / 2;
  const pod: Piece = {
    x: podX,
    y: podY,
    w: podRight - podX,
    h: s.pod.h,
    r: sideOpen ? [0, pr, pr, s.podCorner] : [pr, pr, pr, pr],
  };
  const contentTop = bodyY + s.topBar;
  const contentBottom = bodyBottom - s.bottomBar;
  let sideClip: string | null = null;
  if (side) {
    const c = sideCurve(bodyY);
    const t = (y: number) => y - bodyY;
    const k = s.sideCorner;
    const tp = taperCurve(bodyX, podY);
    sideClip = [
      `M ${tp.p3[0]} ${side.h}`,
      `C ${tp.c2[0]} ${t(tp.c2[1])} ${tp.c1[0]} ${t(tp.c1[1])} ${tp.p0[0]} ${t(tp.p0[1])}`,
      `L 0 ${t(s.sideTop + k)}`,
      `A ${k} ${k} 0 0 1 ${k} ${t(s.sideTop)}`,
      `L ${c.p0[0]} ${t(c.p0[1])}`,
      `C ${c.c1[0]} ${t(c.c1[1])} ${c.c2[0]} ${t(c.c2[1])} ${c.p3[0]} ${t(c.p3[1])}`,
      // Más allá de su ancho, la lateral solo ocupa la franja superior (la subida).
      `L ${c.p3[0]} ${s.topBar}`,
      `L ${s.sideW} ${s.topBar}`,
      `L ${s.sideW} ${side.h}`,
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
    // Con lateral, la franja empieza donde termina su subida.
    topBar: (() => {
      const x = sideOpen ? s.sideSlope.to : bodyX;
      return { x, y: bodyY, w: w - s.tabW - x, h: s.topBar };
    })(),
    bar: { x: podRight, y: contentBottom, w: w - podRight, h: bodyBottom - contentBottom },
  };
}

/// Subida en S de la lateral: del tramo recto de su borde superior al borde
/// superior del cuerpo, con tangentes horizontales en ambos extremos (la misma
/// figura que la subida del cuerpo hacia la pestaña).
function sideCurve(bodyY: number) {
  const { from, to } = SKIN.sideSlope;
  const m = (to - from) / 2;
  return {
    p0: [from, SKIN.sideTop] as const,
    c1: [from + m, SKIN.sideTop] as const,
    c2: [to - m, bodyY] as const,
    p3: [to, bodyY] as const,
  };
}

/// Estrechamiento en S de la parte de abajo de la lateral: del borde izquierdo
/// (vertical) al lado izquierdo de la píldora (vertical), que empieza en `x`.
function taperCurve(x: number, podY: number) {
  const y0 = podY - SKIN.taper;
  const m = SKIN.taper / 2;
  return {
    p0: [0, y0] as const,
    c1: [0, y0 + m] as const,
    c2: [x, podY - m] as const,
    p3: [x, podY] as const,
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
  const fR = lineCircleFillet(bb, cr, cy, pr, SKIN.podFillet, 1);
  const fL = lineCircleFillet(bb, cl, cy, pr, F, -1);
  const n = (v: number) => +v.toFixed(2);
  const d: string[] = [];
  // Curva en S del cuerpo hacia la pestaña.
  const s0 = tab.x - SKIN.tabInset - SKIN.tabSlope;
  const s3 = tab.x;
  const sm = (s3 - s0) * 0.5;
  if (side) {
    const c = sideCurve(by);
    const k = SKIN.sideCorner;
    d.push(
      `M 0 ${c.p0[1] + k}`,
      `A ${k} ${k} 0 0 1 ${k} ${c.p0[1]}`,
      `L ${c.p0[0]} ${c.p0[1]}`,
      `C ${c.c1[0]} ${c.c1[1]} ${c.c2[0]} ${c.c2[1]} ${c.p3[0]} ${c.p3[1]}`,
    );
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
    `A ${SKIN.podFillet} ${SKIN.podFillet} 0 0 0 ${n(fR.onCircle[0])} ${n(fR.onCircle[1])}`,
    `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
  );
  if (side) {
    // La píldora termina por la izquierda en una esquina redonda y sube por la
    // lateral, que se estrecha en S hasta su borde izquierdo.
    const k = pod.r[3];
    const tp = taperCurve(pod.x, pod.y);
    d.push(
      `L ${pod.x + k} ${pod.y + pod.h}`,
      `A ${k} ${k} 0 0 1 ${pod.x} ${pod.y + pod.h - k}`,
      `L ${tp.p3[0]} ${tp.p3[1]}`,
      `C ${tp.c2[0]} ${tp.c2[1]} ${tp.c1[0]} ${tp.c1[1]} ${tp.p0[0]} ${tp.p0[1]}`,
    );
  } else {
    d.push(
      `L ${cl} ${pod.y + pod.h}`,
      `A ${pr} ${pr} 0 0 1 ${n(fL.onCircle[0])} ${n(fL.onCircle[1])}`,
      `A ${F} ${F} 0 0 0 ${n(fL.onLine[0])} ${bb}`,
    );
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
