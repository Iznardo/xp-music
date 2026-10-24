// Generates site/src/desktop.svg, the desktop background: an original illustration (sky,
// clouds and hill), not the Windows XP "Bliss" photo. Usage: node site/desktop.mjs site/src/desktop.svg
let seed = 7;
const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
const f = (n) => +n.toFixed(1);

// Cumulus: flat base and bumps that grow toward the center. Two layers: the shadow
// (blue-grey) and the white one on top, raised, so only the grey base shows.
function cumulus(cx, base, w, h, n = 14) {
  const pts = [];
  for (let i = 0; i < n; i++) {
    const t = rnd() * 2 - 1;
    const bell = 1 - t * t;
    const top = h * (0.3 + 0.7 * bell) * (0.7 + 0.3 * rnd());
    const r = Math.max(5, top * (0.5 + 0.25 * rnd()));
    pts.push([cx + (t * w) / 2, Math.min(base - top + r, base - r * 0.35), r]);
  }
  const c = (dy, k) => pts.map(([x, y, r]) => `<circle cx="${f(x)}" cy="${f(y + dy)}" r="${f(r * k)}"/>`).join("");
  const lift = Math.max(3, h * 0.16);
  return `<g><g fill="#9fb2d6" filter="url(#cshade)">${c(0, 1)}</g><g fill="url(#cwhite)" filter="url(#cumulus)">${c(-lift, 0.95)}</g></g>`;
}
function wisp(x, y, len, rot, op = 0.75) {
  let s = "";
  for (let i = 0; i < 9; i++) {
    const dx = (rnd() - 0.5) * len * 0.6, dy = (rnd() - 0.5) * 14;
    s += `<ellipse cx="${f(x + dx)}" cy="${f(y + dy)}" rx="${f(len * (0.25 + 0.3 * rnd()))}" ry="${f(2 + 4 * rnd())}"/>`;
  }
  return `<g transform="rotate(${rot} ${x} ${y})" opacity="${op}">${s}</g>`;
}

const clouds = [
  cumulus(170, 40, 320, 90, 16), cumulus(1330, 80, 460, 120, 20), cumulus(1560, 150, 160, 60, 8),
  cumulus(400, 205, 120, 30, 7), cumulus(250, 335, 130, 30, 7),
  cumulus(790, 300, 210, 60, 12), cumulus(1080, 325, 170, 50, 10), cumulus(1270, 345, 240, 60, 12),
  cumulus(1470, 255, 210, 50, 10), cumulus(1000, 432, 360, 34, 16), cumulus(1430, 438, 330, 36, 14),
  cumulus(640, 425, 160, 18, 8), cumulus(1500, 345, 110, 28, 6),
  ...Array.from({ length: 9 }, (_, i) => cumulus(560 + i * 120 + rnd() * 40, 442 + i * 2 + rnd() * 6, 110 + rnd() * 90, 16 + rnd() * 24, 11)),
].join("");
const wisps = [wisp(280, 160, 220, -8), wisp(760, 70, 320, 22, 0.6), wisp(1080, 385, 300, -4, 0.55), wisp(300, 285, 170, 12, 0.6), wisp(1450, 200, 200, -14, 0.5)].join("");

// Yellow flowers in the bottom strip.
// Top edge of the meadow: almost straight, a bit higher on the right.
const edge = (x) => { const t = x / 1600; return 744 - 34 * t - 7 * Math.sin(Math.PI * t); };
const xs = Array.from({ length: 161 }, (_, i) => i * 10);
const line = (pts) => pts.map(([x, y], i) => `${i ? "L" : "M"}${f(x)} ${f(y)}`).join(" ");

// Strip of soil between the hill and the meadow: wide on the left, thinning
// to a thread toward the right. Irregular top edge.
const wobble = (x) => 0.75 + 0.18 * Math.sin(x / 37) + 0.12 * Math.sin(x / 13 + 1) + 0.1 * Math.sin(x / 91 + 2);
const cut = (x) => 1.5 + 28 * Math.max(0, 1 - x / 700) ** 1.6 * wobble(x) + 2 * rnd();
const soil = line([...xs.map((x) => [x, edge(x) - cut(x)]), ...xs.slice().reverse().map((x) => [x, edge(x) + 3])]) + " Z";

// Dry grass over the wide stretch of the soil strip: seen from afar it is a brown,
// fuzzy mass, not individual stalks. Height in clumps (summed bell curves).
const clumps = [[60, 14], [150, 20], [230, 12], [310, 18], [400, 11], [480, 8], [560, 5]];
const scrubH = (x) => clumps.reduce((h, [c, a]) => h + a * Math.exp(-(((x - c) / 38) ** 2)), 0) * Math.max(0, 1 - x / 640);
const sx = Array.from({ length: 166 }, (_, i) => i * 4);
const scrubBase = (x) => edge(x) - cut(x) * 0.45;
const scrub = line([...sx.map((x) => [x, scrubBase(x) - scrubH(x) - 2 * rnd()]), ...sx.slice().reverse().map((x) => [x, scrubBase(x)])]) + " Z";

// Meadow blades that cover the edge of the soil strip.
let blades = "";
for (let x = 0; x < 1600; x += 1.6 + rnd() * 2) {
  const y = edge(x) + 2 + rnd() * 3;
  const h = 3 + rnd() * 9;
  blades += `<path d="M${f(x)} ${f(y)} l ${f((rnd() - 0.5) * 3)} ${f(-h)}" stroke="${rnd() < 0.5 ? "#3f8a1a" : "#5aa52a"}"/>`;
}

// Yellow flowers, denser near the edge.
let flowers = "";
for (let i = 0; i < 140; i++) {
  const x = rnd() * 1600;
  flowers += `<circle cx="${f(x)}" cy="${f(edge(x) + 8 + rnd() ** 1.8 * 130)}" r="${f(0.8 + rnd() * 1.2)}"/>`;
}

// Grass texture: fine grain (stretched horizontally, visible from afar) and large soft
// patches, in grey with mean 0.5 and blended with "soft light" over the color
// underneath (lightens and darkens without changing the hue). k: contrast.
const grass = (id, fine, patch, k) => {
  const ch = (c) => `<feFunc${c} type="linear" slope="${k}" intercept="${f(0.5 - k / 2)}"/>`;
  return `  <filter id="${id}" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="${fine}" numOctaves="3" seed="3" result="fine"/>
    <feTurbulence type="fractalNoise" baseFrequency="${patch}" numOctaves="4" seed="8" result="patch"/>
    <feComposite in="fine" in2="patch" operator="arithmetic" k2=".75" k3=".35" k4="-.05" result="n"/>
    <feColorMatrix in="n" type="matrix" values=".33 .33 .33 0 0  .33 .33 .33 0 0  .33 .33 .33 0 0  0 0 0 0 1" result="g"/>
    <feComponentTransfer in="g" result="c">${ch("R")}${ch("G")}${ch("B")}</feComponentTransfer>
    <feBlend in="c" in2="SourceGraphic" mode="soft-light" result="b"/>
    <feComposite in="b" in2="SourceAlpha" operator="in"/>
  </filter>
`;
};

const hill = "M0 515 C 180 470 400 436 620 438 C 900 441 1200 500 1600 562 L 1600 900 L 0 900 Z";
const field = `${line(xs.map((x) => [x, edge(x)]))} L 1600 900 L 0 900 Z`;

const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 900" width="1600" height="900" preserveAspectRatio="xMidYMid slice">
<!-- Website desktop: original illustration (not the Windows XP "Bliss" photo). Generated by site/desktop.mjs. -->
<defs>
  <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#0a47d1"/><stop offset=".3" stop-color="#1c63e2"/><stop offset=".5" stop-color="#3f86ec"/><stop offset=".6" stop-color="#8bb8f2"/><stop offset=".68" stop-color="#c9def6"/>
  </linearGradient>
  <radialGradient id="glow" cx=".25" cy=".1" r=".7"><stop offset="0" stop-color="#fff" stop-opacity=".18"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
  <!-- Hill: sunlit lime on the crest, intense green in the middle and dark at the bottom. -->
  <linearGradient id="hill" gradientUnits="userSpaceOnUse" x1="0" y1="436" x2="0" y2="800">
    <stop offset="0" stop-color="#a9d84c"/><stop offset=".07" stop-color="#8fca3c"/><stop offset=".3" stop-color="#63af27"/><stop offset=".62" stop-color="#43951b"/><stop offset="1" stop-color="#2d7911"/>
  </linearGradient>
  <!-- Light from the left; the right slope is more in shadow. -->
  <linearGradient id="light" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="1600" y2="0">
    <stop offset="0" stop-color="#d9f27a" stop-opacity=".1"/><stop offset=".35" stop-color="#d9f27a" stop-opacity=".16"/><stop offset=".6" stop-color="#0d3a04" stop-opacity="0"/><stop offset="1" stop-color="#0d3a04" stop-opacity=".4"/>
  </linearGradient>
  <linearGradient id="field" gradientUnits="userSpaceOnUse" x1="0" y1="705" x2="0" y2="900"><stop offset="0" stop-color="#4f9c21"/><stop offset="1" stop-color="#36821a"/></linearGradient>
  <linearGradient id="ridge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#7d8fb8"/><stop offset="1" stop-color="#a9b9d6"/></linearGradient>

  <!-- Cumulus: frayed edges and the bottom part in blue-grey. -->
  <filter id="cumulus" x="-30%" y="-60%" width="160%" height="220%">
    <feTurbulence type="fractalNoise" baseFrequency=".035" numOctaves="4" seed="4" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="16" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="2.2"/>
  </filter>
  <filter id="cshade" x="-30%" y="-60%" width="160%" height="220%">
    <feTurbulence type="fractalNoise" baseFrequency=".035" numOctaves="4" seed="4" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="16" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="3.2"/>
  </filter>
  <radialGradient id="cwhite" cx=".45" cy=".25" r=".85"><stop offset=".45" stop-color="#fff"/><stop offset="1" stop-color="#dfe8f6"/></radialGradient>
  <filter id="wisp" x="-40%" y="-200%" width="180%" height="500%">
    <feTurbulence type="fractalNoise" baseFrequency=".006 .08" numOctaves="3" seed="9" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="34" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="5"/>
  </filter>
  <!-- Grass: fine grain (stretched horizontally: visible from afar) and large patches,
       blended with soft light over the color underneath. In front, coarser. -->
${grass("grass-far", "1.2 .25", ".003 .03", 1.7)}${grass("grass-near", ".5 .1", ".006 .05", 2)}  <!-- Cloud shadows on the grass: patches with an irregular edge. -->
  <filter id="cloud-shadow" x="-30%" y="-80%" width="160%" height="260%">
    <feTurbulence type="fractalNoise" baseFrequency=".006 .02" numOctaves="3" seed="12" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="70" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="24"/>
  </filter>
  <linearGradient id="soil" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#4d3127"/><stop offset=".5" stop-color="#3a231c"/><stop offset="1" stop-color="#1f1510"/>
  </linearGradient>
  <!-- Dry grass: blends into the hill at the top, darkens toward the soil at the bottom. -->
  <linearGradient id="scrub" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#58603a" stop-opacity=".15"/><stop offset=".45" stop-color="#5a4a35" stop-opacity=".75"/><stop offset="1" stop-color="#3d2c22"/>
  </linearGradient>
  <!-- Top edge in vertical streaks (stalks from afar) and slightly blurred. -->
  <filter id="scrub-edge" x="-2%" y="-60%" width="104%" height="220%">
    <feTurbulence type="fractalNoise" baseFrequency=".5 .03" numOctaves="2" seed="31" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="10" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation=".8"/>
  </filter>
  <filter id="furrow" x="-2%" y="-200%" width="104%" height="500%">
    <feTurbulence type="fractalNoise" baseFrequency=".03 .3" numOctaves="2" seed="21" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="4" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation=".7"/>
  </filter>
  <filter id="soft" x="-20%" y="-50%" width="140%" height="200%"><feGaussianBlur stdDeviation="28"/></filter>
  <filter id="edge" x="-5%" y="-50%" width="110%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
</defs>

<rect width="1600" height="900" fill="url(#sky)"/>
<rect width="1600" height="900" fill="url(#glow)"/>
<g fill="#fff" filter="url(#wisp)">${wisps}</g>
<g opacity=".97">${clouds}</g>

<path d="M1150 552 C 1260 518 1360 505 1440 514 C 1510 521 1560 512 1600 516 L 1600 620 L 1150 620 Z" fill="url(#ridge)"/>

<g filter="url(#grass-far)">
  <path d="${hill}" fill="url(#hill)"/>
  <path d="${hill}" fill="url(#light)"/>
  <g fill="#cdeb6a" opacity=".22" filter="url(#soft)">
    <ellipse cx="380" cy="545" rx="440" ry="34" transform="rotate(-5 380 545)"/><ellipse cx="560" cy="700" rx="420" ry="30"/>
  </g>
  <g fill="#0f3d05" opacity=".38" filter="url(#cloud-shadow)">
    <ellipse cx="1130" cy="590" rx="300" ry="30"/><ellipse cx="1300" cy="715" rx="480" ry="62"/><ellipse cx="640" cy="760" rx="420" ry="32"/>
  </g>
</g>
<!-- Shine on the crest. -->
<path d="M0 515 C 180 470 400 436 620 438 C 900 441 1200 500 1600 562" fill="none" stroke="#d2f08a" stroke-width="2.5" opacity=".5" filter="url(#edge)"/>

<!-- Rut running down the right slope. -->
<path d="M1405 646 C 1470 662 1540 676 1600 690" fill="none" stroke="#174d08" stroke-width="2.5" stroke-dasharray="14 6 30 9" opacity=".6" filter="url(#furrow)"/>
<!-- Foreground: the meadow with flowers and, between it and the hill, the soil strip. -->
<g filter="url(#grass-near)">
  <path d="${field}" fill="url(#field)"/>
  <g fill="#0f3d05" opacity=".3" filter="url(#cloud-shadow)"><ellipse cx="1250" cy="870" rx="420" ry="40"/></g>
</g>
<!-- The grass darkens as it reaches the soil strip. -->
<path d="${line(xs.map((x) => [x, edge(x) - cut(x) * 0.7 - 2]))}" fill="none" stroke="#123f06" stroke-width="7" opacity=".35" filter="url(#edge)"/>
<g filter="url(#grass-near)"><path d="${soil}" fill="url(#soil)" filter="url(#furrow)"/></g>
<path d="${scrub}" fill="url(#scrub)" filter="url(#scrub-edge)"/>
<g fill="none" stroke-width="1" opacity=".9">${blades}</g>
<g fill="#f2e45a" opacity=".85">${flowers}</g>
</svg>
`;
import { writeFileSync } from "node:fs";
writeFileSync(process.argv[2], svg);
console.log(svg.length, "bytes");
