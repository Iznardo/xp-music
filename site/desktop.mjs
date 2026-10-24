// Genera site/src/desktop.svg, el fondo del escritorio: ilustración original (cielo,
// nubes y colina), no la foto «Bliss» de Windows XP. Uso: node site/desktop.mjs site/src/desktop.svg
let seed = 7;
const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
const f = (n) => +n.toFixed(1);

// Cúmulos: base plana y bultos que crecen hacia el centro. Dos capas: la sombra
// (gris azulado) y encima la blanca, subida, para que solo asome la base gris.
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

// Flores amarillas en la franja de abajo.
// Borde de arriba del prado: casi recto, un poco más alto a la derecha.
const edge = (x) => { const t = x / 1600; return 744 - 34 * t - 7 * Math.sin(Math.PI * t); };
const xs = Array.from({ length: 161 }, (_, i) => i * 10);
const line = (pts) => pts.map(([x, y], i) => `${i ? "L" : "M"}${f(x)} ${f(y)}`).join(" ");

// Corte de tierra entre la colina y el prado: ancho a la izquierda, se afina
// hasta ser un hilo hacia la derecha. Borde de arriba irregular.
const wobble = (x) => 0.75 + 0.18 * Math.sin(x / 37) + 0.12 * Math.sin(x / 13 + 1) + 0.1 * Math.sin(x / 91 + 2);
const cut = (x) => 1.5 + 28 * Math.max(0, 1 - x / 700) ** 1.6 * wobble(x) + 2 * rnd();
const soil = line([...xs.map((x) => [x, edge(x) - cut(x)]), ...xs.slice().reverse().map((x) => [x, edge(x) + 3])]) + " Z";

// Hierbas secas encima del tramo ancho del corte: vistas de lejos son una masa
// parda y difusa, no tallos sueltos. Altura en matas (campanas sumadas).
const clumps = [[60, 14], [150, 20], [230, 12], [310, 18], [400, 11], [480, 8], [560, 5]];
const scrubH = (x) => clumps.reduce((h, [c, a]) => h + a * Math.exp(-(((x - c) / 38) ** 2)), 0) * Math.max(0, 1 - x / 640);
const sx = Array.from({ length: 166 }, (_, i) => i * 4);
const scrubBase = (x) => edge(x) - cut(x) * 0.45;
const scrub = line([...sx.map((x) => [x, scrubBase(x) - scrubH(x) - 2 * rnd()]), ...sx.slice().reverse().map((x) => [x, scrubBase(x)])]) + " Z";

// Briznas del prado que tapan el borde del corte.
let blades = "";
for (let x = 0; x < 1600; x += 1.6 + rnd() * 2) {
  const y = edge(x) + 2 + rnd() * 3;
  const h = 3 + rnd() * 9;
  blades += `<path d="M${f(x)} ${f(y)} l ${f((rnd() - 0.5) * 3)} ${f(-h)}" stroke="${rnd() < 0.5 ? "#3f8a1a" : "#5aa52a"}"/>`;
}

// Flores amarillas, más densas cerca del borde.
let flowers = "";
for (let i = 0; i < 140; i++) {
  const x = rnd() * 1600;
  flowers += `<circle cx="${f(x)}" cy="${f(edge(x) + 8 + rnd() ** 1.8 * 130)}" r="${f(0.8 + rnd() * 1.2)}"/>`;
}

// Textura de hierba: grano fino (estirado en horizontal, se ve de lejos) y manchas
// grandes suaves, en gris con media 0,5 y mezclado en "luz suave" sobre el color de
// debajo (aclara y oscurece sin cambiar el tono). k: contraste.
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
<!-- Escritorio de la web: ilustración original (no es la foto «Bliss» de Windows XP). La genera site/desktop.mjs. -->
<defs>
  <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#0a47d1"/><stop offset=".3" stop-color="#1c63e2"/><stop offset=".5" stop-color="#3f86ec"/><stop offset=".6" stop-color="#8bb8f2"/><stop offset=".68" stop-color="#c9def6"/>
  </linearGradient>
  <radialGradient id="glow" cx=".25" cy=".1" r=".7"><stop offset="0" stop-color="#fff" stop-opacity=".18"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
  <!-- Colina: lima al sol en la cresta, verde intenso en medio y oscuro abajo. -->
  <linearGradient id="hill" gradientUnits="userSpaceOnUse" x1="0" y1="436" x2="0" y2="800">
    <stop offset="0" stop-color="#a9d84c"/><stop offset=".07" stop-color="#8fca3c"/><stop offset=".3" stop-color="#63af27"/><stop offset=".62" stop-color="#43951b"/><stop offset="1" stop-color="#2d7911"/>
  </linearGradient>
  <!-- Luz desde la izquierda; la ladera derecha, más en sombra. -->
  <linearGradient id="light" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="1600" y2="0">
    <stop offset="0" stop-color="#d9f27a" stop-opacity=".1"/><stop offset=".35" stop-color="#d9f27a" stop-opacity=".16"/><stop offset=".6" stop-color="#0d3a04" stop-opacity="0"/><stop offset="1" stop-color="#0d3a04" stop-opacity=".4"/>
  </linearGradient>
  <linearGradient id="field" gradientUnits="userSpaceOnUse" x1="0" y1="705" x2="0" y2="900"><stop offset="0" stop-color="#4f9c21"/><stop offset="1" stop-color="#36821a"/></linearGradient>
  <linearGradient id="ridge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#7d8fb8"/><stop offset="1" stop-color="#a9b9d6"/></linearGradient>

  <!-- Cúmulos: bordes deshilachados y la parte de abajo en gris azulado. -->
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
  <!-- Hierba: grano fino (estirado en horizontal: se ve de lejos) y manchas grandes,
       mezclados en luz suave sobre el color de debajo. Delante, más grueso. -->
${grass("grass-far", "1.2 .25", ".003 .03", 1.7)}${grass("grass-near", ".5 .1", ".006 .05", 2)}  <!-- Sombras de nubes sobre la hierba: manchas con borde irregular. -->
  <filter id="cloud-shadow" x="-30%" y="-80%" width="160%" height="260%">
    <feTurbulence type="fractalNoise" baseFrequency=".006 .02" numOctaves="3" seed="12" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="70" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="24"/>
  </filter>
  <linearGradient id="soil" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#4d3127"/><stop offset=".5" stop-color="#3a231c"/><stop offset="1" stop-color="#1f1510"/>
  </linearGradient>
  <!-- Hierba seca: arriba se funde con la colina, abajo se oscurece hacia la tierra. -->
  <linearGradient id="scrub" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#58603a" stop-opacity=".15"/><stop offset=".45" stop-color="#5a4a35" stop-opacity=".75"/><stop offset="1" stop-color="#3d2c22"/>
  </linearGradient>
  <!-- Borde de arriba en vetas verticales (tallos de lejos) y algo difuminado. -->
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
<!-- Brillo en la cresta. -->
<path d="M0 515 C 180 470 400 436 620 438 C 900 441 1200 500 1600 562" fill="none" stroke="#d2f08a" stroke-width="2.5" opacity=".5" filter="url(#edge)"/>

<!-- Rodera que baja por la ladera derecha. -->
<path d="M1405 646 C 1470 662 1540 676 1600 690" fill="none" stroke="#174d08" stroke-width="2.5" stroke-dasharray="14 6 30 9" opacity=".6" filter="url(#furrow)"/>
<!-- Primer plano: el prado con flores y, entre él y la colina, el corte de tierra. -->
<g filter="url(#grass-near)">
  <path d="${field}" fill="url(#field)"/>
  <g fill="#0f3d05" opacity=".3" filter="url(#cloud-shadow)"><ellipse cx="1250" cy="870" rx="420" ry="40"/></g>
</g>
<!-- La hierba se oscurece al llegar al corte. -->
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
