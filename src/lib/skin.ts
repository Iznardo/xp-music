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
  /// The body's top edge rises in an S up to the tab: it starts this far before
  /// the tab and reaches the top this far inside it.
  tabSlope: 90,
  tabInset: 34,
  /// Body's top strip (menu button and round buttons).
  topBar: 28,
  /// Gap below the body: only the pill reaches the very bottom.
  bottomGap: 26,
  /// Body's bottom bar (controls to the right of the pill).
  bottomBar: 58,
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
  const contentBottom = bodyBottom - s.bottomBar;
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
      return { x, y: bodyY, w: w - s.tabW - x, h: s.topBar };
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
function shoulder(x: number, top: number): string {
  const { sideW, topBar } = SKIN;
  const mid = top + topBar / 2;
  return `C ${x} ${mid} ${sideW} ${mid} ${sideW} ${top + topBar}`;
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
  if (!side) {
    // Without a side panel, just the pill (flush with the left edge).
    return [
      `M ${pod.x} ${pod.y}`,
      `L ${cr} ${pod.y}`,
      `A ${pr} ${pr} 0 0 1 ${cr} ${pod.y + pod.h}`,
      `L ${pod.x + bk} ${pod.y + pod.h}`,
      `A ${bk} ${bk} 0 0 1 ${pod.x} ${pod.y + pod.h - bk}`,
      "Z",
    ].join(" ");
  }
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

/// Playlist tab as its own layer over the body (as in WMP 9): from the end of the top
/// strip it rises in an S to the top edge and covers the rest of the strip. `edge` is
/// just the S, for its rim. Clip it with the outline (its top-right corner is square).
export function tabLayerPath(l: SkinLayout): { fill: string; edge: string } {
  const x0 = l.topBar.x + l.topBar.w;
  const y0 = l.content.y;
  const x1 = l.tab.x + 12;
  const m = (x1 - x0) / 2;
  const edge = `M ${x0} ${y0} C ${x0 + m} ${y0} ${x1 - m} 0 ${x1} 0`;
  const right = l.tab.x + l.tab.w;
  return { edge, fill: `${edge} L ${right} 0 L ${right} ${y0} Z` };
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
  // S curve from the body to the tab.
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
