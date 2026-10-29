// WMP 9 skin geometry (specs/008). The window is not a rectangle: it is the
// union of overlapping pieces, as in the original player:
//
//            ╭──────── list tab (sticks out on top) ───────────╮
//   ╭────────┴─ silver body ───────────────────────────────────┤
//  ╭┴╮ side panel                                              │
//  │ │ (starts lower,       [ visualization ]  [ list ]        │
//  │ │  rounded corners)                                       │
//  ╰┬╯                                                         │
//   ╰─(  controls pill  )── bottom bar ────────────────────── ◢┘ ← square corner with grip
//      (sticks out below)
//
// All in CSS pixels, derived from the window size.

export type Rect = { x: number; y: number; w: number; h: number };
/// Radii per corner: top-left, top-right, bottom-right, bottom-left.
export type Piece = Rect & { r: [number, number, number, number] };

export const SKIN = {
  sideW: 118,
  /// Top edge of the side panel: runs straight at this height and then rises in an S
  /// (like the tab's) up to the body's top edge.
  sideTop: 48,
  sideCorner: 12,
  /// Span of the side panel's S rise (start and end x).
  sideSlope: { from: 40, to: 160 },
  /// The body covers the side panel's right edge.
  sideOverlap: 10,
  /// The list tab sticks out this much above the body.
  tabRise: 22,
  tabW: 286,
  tabH: 44,
  /// The tab's S: one curve from the content's top edge up to the window's top edge,
  /// spanning this width. The body's outline follows its upper part, so the tab layer
  /// and the window edge share a single cut.
  tabSlope: 100,
  tabInset: 34,
  /// Body's top strip (menu button and round buttons).
  topBar: 28,
  /// Gap below the body: only the pill reaches the very bottom.
  bottomGap: 26,
  pod: { dx: 22, w: 360, h: 88 },
  /// With a side panel, its lower part narrows in an S over this height and
  /// ends directly at the pill's bottom-left corner (side panel and controls,
  /// one piece).
  taper: 150,
  /// Radius of the bottom-left corner of the pill joined to the side panel.
  podCorner: 40,
  /// Fillet joining the pill to the body, on the right.
  podFillet: 18,
  frame: 7,
} as const;

export type SkinLayout = {
  side: Piece | null;
  /// Side panel clip (local coordinates) with its S curve.
  sideClip: string | null;
  body: Piece;
  tab: Piece;
  pod: Piece;
  /// Black content area (views).
  content: Rect;
  /// Body's top strip, to the left of the tab.
  topBar: Rect;
  /// Area of the bottom bar to the right of the pill.
  bar: Rect;
};

export function skinLayout(w: number, h: number, sideOpen: boolean): SkinLayout {
  const s = SKIN;
  const bodyX = sideOpen ? s.sideW - s.sideOverlap : 0;
  const bodyY = s.tabRise;
  const bodyBottom = h - s.bottomGap;
  const podY = h - s.pod.h;
  // The pill starts at the body's edge: its left side is the continuation of
  // the side panel (or, without it, of the window's left edge).
  const podX = bodyX;
  const podRight = bodyX + s.pod.dx + s.pod.w;
  // The side panel spans from the top of the body; its silhouette comes from `sideClip`.
  const side: Piece | null = sideOpen
    ? { x: 0, y: bodyY, w: s.sideSlope.to, h: podY - bodyY, r: [0, 0, 0, 0] }
    : null;
  const body: Piece = { x: bodyX, y: bodyY, w: w - bodyX, h: bodyBottom - bodyY, r: [sideOpen ? 0 : 16, 0, 0, 0] };
  // The tab starts where the curve has already reached the top.
  const tab: Piece = { x: w - s.tabW + s.tabInset, y: 0, w: s.tabW - s.tabInset, h: s.tabH, r: [0, 12, 0, 0] };
  const pr = s.pod.h / 2;
  const pod: Piece = {
    x: podX,
    y: podY,
    w: podRight - podX,
    h: s.pod.h,
    r: [0, pr, pr, s.podCorner],
  };
  const contentTop = bodyY + s.topBar;
  // The content ends where the pill starts: the pill's top edge is the content's bottom
  // edge, so the raised line runs along it. The bottom bar (right of the pill) spans the
  // same height.
  const contentBottom = podY;
  let sideClip: string | null = null;
  if (side) {
    const c = sideCurve(bodyY);
    const t = (y: number) => y - bodyY;
    const k = s.sideCorner;
    const tp = taperCurve(pod);
    sideClip = [
      `M ${tp.p3[0]} ${t(tp.p3[1])}`,
      `C ${tp.c2[0]} ${t(tp.c2[1])} ${tp.c1[0]} ${t(tp.c1[1])} ${tp.p0[0]} ${t(tp.p0[1])}`,
      `L 0 ${t(s.sideTop + k)}`,
      `A ${k} ${k} 0 0 1 ${k} ${t(s.sideTop)}`,
      `L ${c.p0[0]} ${t(c.p0[1])}`,
      `C ${c.c1[0]} ${t(c.c1[1])} ${c.c2[0]} ${t(c.c2[1])} ${c.p3[0]} ${t(c.p3[1])}`,
      // Beyond its width, the side panel only occupies the top strip (the rise).
      shoulder(c.p3[0], 0),
      `L ${s.sideW} ${t(tp.p3[1])}`,
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
    // With a side panel, the strip starts where its rise ends.
    topBar: (() => {
      const x = sideOpen ? s.sideSlope.to : bodyX;
      return { x, y: bodyY, w: tabCurve(tab, contentTop).p0[0] - x, h: s.topBar };
    })(),
    bar: { x: podRight, y: contentBottom, w: w - podRight, h: bodyBottom - contentBottom },
  };
}

/// S rise of the side panel: from the straight stretch of its top edge to the
/// body's top edge, with horizontal tangents at both ends (the same shape as
/// the body's rise towards the tab).
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

/// Where the rise ends, the side panel's color curves down in an S to its right edge
/// across the top strip (as in WMP 9) instead of being cut vertically.
function shoulder(x: number, top: number, end: number = SKIN.sideW): string {
  const mid = top + SKIN.topBar / 2;
  return `C ${x} ${mid} ${end} ${mid} ${end} ${top + SKIN.topBar}`;
}

/// S narrowing of the side panel's lower part: from the left edge (vertical)
/// to the start of the pill's bottom-left corner.
function taperCurve(pod: Piece) {
  const end = pod.y + pod.h - pod.r[3];
  const y0 = end - SKIN.taper;
  const m = SKIN.taper / 2;
  return {
    p0: [0, y0] as const,
    c1: [0, y0 + m] as const,
    c2: [pod.x, end - m] as const,
    p3: [pod.x, end] as const,
  };
}

/// Side panel and pill as a single piece (painted with one fill, no seam):
/// top edge with its rise, S narrowing and pill.
export function sidePodPath(l: SkinLayout): string {
  const { side, pod } = l;
  const pr = pod.h / 2;
  const cr = pod.x + pod.w - pr;
  const bk = pod.r[3];
  // Without a side panel, just the pill (flush with the left edge).
  if (!side) return pillPath(pod);
  const c = sideCurve(side.y);
  const tp = taperCurve(pod);
  const k = SKIN.sideCorner;
  return [
    `M 0 ${SKIN.sideTop + k}`,
    `A ${k} ${k} 0 0 1 ${k} ${SKIN.sideTop}`,
    `L ${c.p0[0]} ${c.p0[1]}`,
    `C ${c.c1[0]} ${c.c1[1]} ${c.c2[0]} ${c.c2[1]} ${c.p3[0]} ${c.p3[1]}`,
    shoulder(c.p3[0], side.y),
    `L ${SKIN.sideW} ${pod.y}`,
    `L ${cr} ${pod.y}`,
    `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
    `L ${pod.x + bk} ${pod.y + pod.h}`,
    `A ${bk} ${bk} 0 0 1 ${pod.x} ${pod.y + pod.h - bk}`,
    `C ${tp.c2[0]} ${tp.c2[1]} ${tp.c1[0]} ${tp.c1[1]} ${tp.p0[0]} ${tp.p0[1]}`,
    "Z",
  ].join(" ");
}

/// The pill alone: square top-left, round right end, bottom-left corner.
function pillPath(pod: Piece): string {
  const pr = pod.h / 2;
  const cr = pod.x + pod.w - pr;
  const bk = pod.r[3];
  return [
    `M ${pod.x} ${pod.y}`,
    `L ${cr} ${pod.y}`,
    `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
    `L ${pod.x + bk} ${pod.y + pod.h}`,
    `A ${bk} ${bk} 0 0 1 ${pod.x} ${pod.y + pod.h - bk}`,
    "Z",
  ].join(" ");
}

/// Raised edge of the side panel + pod, for the same rim/highlight/shadow as the tab's S.
/// Drawn above the content, on its halo: the rim replaces it, so each side of the content
/// reads highlight, rim, the content's black border. With the side panel, one line: the
/// shoulder over the top strip (mirror of the tab's cut), down beside the content, along
/// the pod's top (the content's bottom) and around its right arc. Collapsed, the pod
/// alone: its top edge from the window's left edge. The outer silhouette gets none:
/// there is nothing behind it to cast on.
export function sidePodEdge(l: SkinLayout): string {
  const { side, pod, content } = l;
  const pr = pod.h / 2;
  const cr = pod.x + pod.w - pr;
  // Half a pixel in, on the content's bottom halo; the arc keeps the same inset.
  const y = pod.y + 0.5;
  const r = pr - 0.5;
  // The arc ends where it meets the body's bottom (the fillet's start): below, it is the
  // window's silhouette, which gets no line (it would end clipped and jagged).
  const cy = pod.y + pr;
  const [fx, fy] = lineCircleFillet(l.body.y + l.body.h, cr, cy, pr, SKIN.podFillet, 1).onCircle;
  const end = (v: number, c: number) => +(c + ((v - c) * r) / pr).toFixed(2);
  const top = `L ${cr} ${y} A ${r} ${r} 0 0 1 ${end(fx, cr)} ${end(fy, cy)}`;
  if (!side) return `M 0 ${y} ${top}`;
  const c = sideCurve(side.y);
  const x = content.x - 0.5;
  // The shoulder lands on that line, so the S flows straight into it (no jog at the corner).
  return `M ${c.p3[0]} ${c.p3[1]} ${shoulder(c.p3[0], side.y, x)} L ${x} ${y} ${top}`;
}

/// Playlist tab as its own layer over the body (as in WMP 9): from the end of the top
/// strip it rises in an S to the top edge and covers the rest of the strip. `edge` is
/// just the S, for its rim. Clip it with the outline (its top-right corner is square).
export function tabLayerPath(l: SkinLayout): { fill: string; edge: string } {
  const c = tabCurve(l.tab, l.content.y);
  const edge = `M ${c.p0[0]} ${c.p0[1]} C ${c.c1[0]} ${c.c1[1]} ${c.c2[0]} ${c.c2[1]} ${c.p3[0]} 0`;
  const right = l.tab.x + l.tab.w;
  return { edge, fill: `${edge} L ${right} 0 L ${right} ${c.p0[1]} Z` };
}

type Pt = readonly [number, number];

/// The tab's S, from the content's top edge (`bottom`) up to the window's top edge,
/// with horizontal tangents at both ends. It ends just inside the tab, so the combo
/// sits where the curve has already flattened out.
function tabCurve(tab: Piece, bottom: number) {
  const x1 = tab.x + 12;
  const x0 = x1 - SKIN.tabSlope;
  const m = SKIN.tabSlope / 2;
  return { p0: [x0, bottom] as Pt, c1: [x0 + m, bottom] as Pt, c2: [x1 - m, 0] as Pt, p3: [x1, 0] as Pt };
}

/// Part of a cubic (y decreasing along it) from where it crosses `y` to its end
/// (de Casteljau split).
function cubicFromY(c: { p0: Pt; c1: Pt; c2: Pt; p3: Pt }, y: number) {
  const lerp = (a: Pt, b: Pt, t: number): Pt => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  let lo = 0;
  let hi = 1;
  for (let i = 0; i < 40; i++) {
    const t = (lo + hi) / 2;
    const u = 1 - t;
    const yt = u ** 3 * c.p0[1] + 3 * u * u * t * c.c1[1] + 3 * u * t * t * c.c2[1] + t ** 3 * c.p3[1];
    if (yt > y) lo = t;
    else hi = t;
  }
  const t = (lo + hi) / 2;
  const b = lerp(c.c1, c.c2, t);
  const e = lerp(b, lerp(c.c2, c.p3, t), t);
  const start = lerp(lerp(lerp(c.p0, c.c1, t), b, t), e, t);
  return { start, c1: e, c2: lerp(c.c2, c.p3, t), end: c.p3 };
}

/// Concave fillet between the horizontal line `y = line` (the shape lies above) and
/// a circle that crosses it and sticks out below: returns the tangent point on the
/// line and on the circle. `side` = -1 for the left.
function lineCircleFillet(line: number, cx: number, cy: number, r: number, f: number, side: 1 | -1) {
  const dy = line + f - cy;
  const fx = cx + side * Math.sqrt((r + f) ** 2 - dy ** 2);
  const k = r / (r + f);
  return { onLine: [fx, line] as const, onCircle: [cx + (fx - cx) * k, cy + (line + f - cy) * k] as const };
}

/// Complete outer outline: the union of the pieces with concave fillets at each
/// joint (side panel–body, body–tab and on both sides of the pill), so they read
/// as a single shell. It is painted behind the pieces and is also the window
/// region.
export function outlinePath(l: SkinLayout): string {
  const { body, tab, pod, side } = l;
  const by = body.y;
  const bb = body.y + body.h;
  const right = body.x + body.w;
  const pr = pod.h / 2;
  const cy = pod.y + pr;
  const cr = pod.x + pod.w - pr;
  const fR = lineCircleFillet(bb, cr, cy, pr, SKIN.podFillet, 1);
  const n = (v: number) => +v.toFixed(2);
  const d: string[] = [];
  // Body → tab: the upper part of the tab's own S (one cut for both).
  const ts = cubicFromY(tabCurve(tab, l.content.y), by);
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
    `L ${n(ts.start[0])} ${by}`,
    `C ${n(ts.c1[0])} ${n(ts.c1[1])} ${n(ts.c2[0])} ${n(ts.c2[1])} ${ts.end[0]} 0`,
    `L ${right - tab.r[1]} 0`,
    `A ${tab.r[1]} ${tab.r[1]} 0 0 1 ${right} ${tab.r[1]}`,
    // Square bottom-right corner (resize grip).
    `L ${right} ${bb}`,
    // Body → pill (right), the pill underneath, pill → body (left).
    `L ${n(fR.onLine[0])} ${bb}`,
    `A ${SKIN.podFillet} ${SKIN.podFillet} 0 0 0 ${n(fR.onCircle[0])} ${n(fR.onCircle[1])}`,
    `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
  );
  if (side) {
    // The pill ends on the left in a round corner and goes up along the side
    // panel, which narrows in an S down to its left edge.
    const k = pod.r[3];
    const tp = taperCurve(pod);
    d.push(
      `L ${pod.x + k} ${pod.y + pod.h}`,
      `A ${k} ${k} 0 0 1 ${pod.x} ${pod.y + pod.h - k}`,
      `C ${tp.c2[0]} ${tp.c2[1]} ${tp.c1[0]} ${tp.c1[1]} ${tp.p0[0]} ${tp.p0[1]}`,
    );
  } else {
    // Without a side panel: the left edge goes straight down to the pill's corner.
    const k = pod.r[3];
    d.push(`L ${pod.x + k} ${pod.y + pod.h}`, `A ${k} ${k} 0 0 1 ${pod.x} ${pod.y + pod.h - k}`);
  }
  d.push("Z");
  return d.join(" ");
}

/// CSS position style for a rectangle.
export function place(r: Rect): string {
  return `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;
}

export function radii(p: Piece): string {
  return p.r.map((x) => `${x}px`).join(" ");
}
