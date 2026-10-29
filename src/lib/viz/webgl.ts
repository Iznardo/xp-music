// WebGL2 presets with feedback (trails): Ambience, Battery, Water, Alchemy and Spiral.
// Inspired by the spirit of WMP's visualizations, with our own shaders.

import type { VizData as VizFrame, VizRenderer } from "./types";

const VERT = `#version 300 es
in vec2 p;
out vec2 uv;
void main() { uv = p * 0.5 + 0.5; gl_Position = vec4(p, 0.0, 1.0); }`;

const COPY = `#version 300 es
precision mediump float;
in vec2 uv;
uniform sampler2D src;
out vec4 o;
void main() { o = texture(src, uv); }`;

const COMMON = `#version 300 es
precision highp float;
in vec2 uv;
out vec4 o;
uniform sampler2D prev;
uniform float bands[64];
uniform float level;
// Beat pulse (1 on each bass hit) and bass normalized to the song.
uniform float beat;
uniform float bass;
uniform float time;
// 60 fps frames that this one represents (2 to 30 fps): the trail advances the
// same at any draw rate.
uniform float step;
uniform vec2 res;
vec3 hsv(float h, float s, float v) {
  vec3 k = clamp(abs(mod(h * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
  return v * mix(vec3(1.0), k, s);
}
float band(float x) { int i = int(clamp(x, 0.0, 1.0) * 63.0); return bands[i]; }
float avg(int a, int b) { float s = 0.0; for (int i = a; i < b; i++) s += bands[i]; return s / float(b - a); }
`;

const AMBIENCE = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  c.x *= res.x / res.y;
  // Trail: the previous frame rotates, moves away and fades out gradually.
  float ang = (0.006 + level * 0.01) * step;
  mat2 rot = mat2(cos(ang), -sin(ang), sin(ang), cos(ang));
  vec2 q = rot * c * pow(0.985 - level * 0.01, step);
  q.x /= res.x / res.y;
  vec3 trail = texture(prev, q + 0.5).rgb * pow(0.955, step);

  float r = length(c);
  float a = atan(c.y, c.x);
  float petals = abs(sin(a * 3.0 + time * 0.4));
  float radius = 0.16 + 0.22 * band(petals * 0.7) + 0.05 * sin(a * 6.0 - time) + beat * 0.08;
  float ring = smoothstep(0.018 + beat * 0.01, 0.0, abs(r - radius)) * (0.6 + level * 1.6 + beat);
  float core = smoothstep(0.09 + avg(0, 6) * 0.08, 0.0, r) * avg(0, 6);
  vec3 col = hsv(fract(time * 0.03 + r * 0.6 + petals * 0.1), 0.75, 1.0) * ring + vec3(0.6, 0.8, 1.0) * core;
  o = vec4(max(trail, col), 1.0);
}`;

const BATTERY = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  c.x *= res.x / res.y;
  // 6-axis kaleidoscope.
  float r = length(c);
  float a = atan(c.y, c.x);
  float seg = 6.2831853 / 6.0;
  a = mod(a, seg);
  a = abs(a - seg * 0.5);
  vec2 k = vec2(cos(a), sin(a)) * r;
  // Tunnel: the previous frame zooms in and rotates.
  float zoom = pow(0.94 - bass * 0.04 - beat * 0.06, step);
  float ang = (0.03 * sin(time * 0.3) + level * 0.04) * step;
  mat2 rot = mat2(cos(ang), -sin(ang), sin(ang), cos(ang));
  vec2 q = rot * k * zoom;
  q.x /= res.x / res.y;
  vec3 trail = texture(prev, q + 0.5).rgb * pow(0.93 + level * 0.04, step);

  vec3 col = vec3(0.0);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float e = avg(i * 20, i * 20 + 20);
    // Inside the wedge: symmetry multiplies them into 12 sparkles.
    float pa = (0.5 + 0.5 * sin(time * (0.7 + fi * 0.41) + fi * 2.1)) * seg * 0.5;
    float pr = 0.08 + fi * 0.09 + e * 0.18 + 0.03 * sin(time * (1.3 + fi));
    vec2 pos = vec2(cos(pa), sin(pa)) * pr;
    float d = length(k - pos);
    col += hsv(fract(time * 0.05 + fi * 0.33), 0.85, 1.0) * smoothstep(0.035 + e * 0.07, 0.0, d) * (0.6 + e * 2.0);
  }
  o = vec4(clamp(trail + col, 0.0, 1.0), 1.0);
}`;

const WATER = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  float asp = res.x / res.y;
  c.x *= asp;
  float mid = avg(8, 30);
  // Trail: the previous frame expands from the center, rotates slowly and ripples.
  float ang = (0.004 + 0.008 * sin(time * 0.21)) * step;
  mat2 rot = mat2(cos(ang), -sin(ang), sin(ang), cos(ang));
  vec2 q = rot * c * pow(0.96 - bass * 0.025 - beat * 0.06, step);
  q += 0.0035 * step * vec2(sin(c.y * 28.0 + time * 2.1), cos(c.x * 28.0 + time * 1.7));
  q.x /= asp;
  // It also fades by subtracting a little: without this the background ends up as gray haze.
  vec3 trail = max(texture(prev, q + 0.5).rgb * pow(0.93 + mid * 0.03, step) - vec3(0.008, 0.005, 0.007) * step, 0.0);

  float r = length(c);
  float a = atan(c.y, c.x);
  // Rays: their length comes from the frequency band at each angle (symmetric).
  float fa = abs(fract(a / 6.2831853 + 0.5) * 2.0 - 1.0);
  float b = band(fa);
  float spike = pow(abs(cos(a * 14.0 + time * 0.35)), 24.0);
  float len = 0.06 + b * 0.45 + beat * 0.2;
  float ray = spike * smoothstep(len, 0.0, r) * (0.4 + b * 2.2);
  float core = smoothstep(0.03 + bass * 0.05 + beat * 0.08, 0.0, r);
  vec3 pal = hsv(0.33 + 0.1 * sin(time * 0.05) + r * 0.25, 0.85, 1.0);
  vec3 col = pal * ray * (1.0 + beat) + vec3(0.8, 1.0, 0.85) * core * (0.3 + bass + beat);
  o = vec4(clamp(trail + col, 0.0, 1.0), 1.0);
}`;

const ALCHEMY = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  float asp = res.x / res.y;
  c.x *= asp;
  float mid = avg(6, 24);
  // Plasma with domain warping: the bass twists, the mids speed it up.
  vec2 p = c * 3.2;
  float t = time * (0.3 + mid * 0.4);
  for (int i = 1; i < 6; i++) {
    float fi = float(i);
    p += vec2(0.7 / fi * sin(fi * p.y + t + 0.3 * fi) + bass * 0.2 + beat * 0.35, 0.7 / fi * cos(fi * p.x + t * 1.13 + 0.5 * fi));
  }
  float v = 0.5 + 0.5 * sin(p.x + p.y + time * 0.2);
  // Thin contour stripes: the psychedelic "lava lamp" touch.
  float bands2 = 0.5 + 0.5 * sin(v * 18.0 - time * 2.0 - bass * 4.0 - beat * 3.0);
  vec3 col = hsv(fract(v * 0.7 + time * 0.035 + bass * 0.25), 0.85, 0.15 + 0.85 * bands2 * smoothstep(0.05, 0.9, v));
  // Some rotating trail so it "flows".
  float ang = (0.012 + level * 0.03) * step;
  vec2 q = mat2(cos(ang), -sin(ang), sin(ang), cos(ang)) * c * pow(0.99, step);
  q.x /= asp;
  vec3 trail = texture(prev, q + 0.5).rgb;
  o = vec4(clamp(mix(col, trail, 0.5 - beat * 0.3) * (0.85 + level * 0.4 + beat * 0.6), 0.0, 1.0), 1.0);
}`;

const SPIRAL = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  float asp = res.x / res.y;
  c.x *= asp;
  float mid = avg(6, 24);
  float r = length(c) + 1e-4;
  float a = atan(c.y, c.x);
  float lr = log(r);
  // Logarithmic spiral arms that advance with the beat.
  float arms = sin(a * 5.0 + lr * 9.0 - time * 2.2 - bass * 2.0 - beat * 2.5);
  float glow = smoothstep(-0.1, 0.9, arms) * (0.25 + mid * 1.2 + beat * 0.9) * smoothstep(0.0, 0.12, r);
  vec3 col = hsv(fract(lr * 0.35 + time * 0.06 + a / 6.2831853), 0.9, 1.0) * glow;
  // Inward trail: tunnel feeling.
  float ang = (-0.02 - level * 0.03) * step;
  vec2 q = mat2(cos(ang), -sin(ang), sin(ang), cos(ang)) * c * pow(1.025 + beat * 0.04, step);
  q.x /= asp;
  vec3 trail = texture(prev, q + 0.5).rgb * pow(0.86, step);
  o = vec4(clamp(max(trail, col) + col * 0.3, 0.0, 1.0), 1.0);
}`;

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader");
  return s;
}

function program(gl: WebGL2RenderingContext, frag: string): WebGLProgram {
  const p = gl.createProgram()!;
  gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, frag));
  gl.bindAttribLocation(p, 0, "p");
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
  return p;
}

class Feedback implements VizRenderer {
  readonly continuous = true;
  private gl: WebGL2RenderingContext;
  private draw_: WebGLProgram;
  private copy: WebGLProgram;
  private tex: WebGLTexture[] = [];
  private fbo: WebGLFramebuffer[] = [];
  private cur = 0;
  private w = 1;
  private h = 1;
  private buf: WebGLBuffer;
  private lastT: number | null = null;

  constructor(canvas: HTMLCanvasElement, frag: string) {
    const gl = canvas.getContext("webgl2", { antialias: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error("WebGL2 unavailable");
    this.gl = gl;
    this.draw_ = program(gl, frag);
    this.copy = program(gl, COPY);
    this.buf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  }

  resize(width: number, height: number) {
    const gl = this.gl;
    // The trail is computed at half resolution: barely noticeable and 4 times cheaper.
    this.w = Math.max(1, Math.round(width / 2));
    this.h = Math.max(1, Math.round(height / 2));
    this.tex.forEach((t) => gl.deleteTexture(t));
    this.fbo.forEach((f) => gl.deleteFramebuffer(f));
    this.tex = [];
    this.fbo = [];
    for (let i = 0; i < 2; i++) {
      const t = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, this.w, this.h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      const f = gl.createFramebuffer()!;
      gl.bindFramebuffer(gl.FRAMEBUFFER, f);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t, 0);
      this.tex.push(t);
      this.fbo.push(f);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  draw(f: VizFrame, t: number) {
    const gl = this.gl;
    const next = 1 - this.cur;
    // 1) New frame from the previous one.
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo[next]!);
    gl.viewport(0, 0, this.w, this.h);
    gl.useProgram(this.draw_);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex[this.cur]!);
    gl.uniform1i(gl.getUniformLocation(this.draw_, "prev"), 0);
    gl.uniform1fv(gl.getUniformLocation(this.draw_, "bands"), f.bands);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "level"), f.level);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "beat"), f.beat);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "bass"), f.bass);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "time"), t);
    // Time since the previous frame in "60 fps frames" (clamped after pauses).
    const step = this.lastT === null ? 1 : Math.min(4, Math.max(0.5, (t - this.lastT) * 60));
    this.lastT = t;
    gl.uniform1f(gl.getUniformLocation(this.draw_, "step"), step);
    gl.uniform2f(gl.getUniformLocation(this.draw_, "res"), this.w, this.h);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    // 2) To screen.
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.useProgram(this.copy);
    gl.bindTexture(gl.TEXTURE_2D, this.tex[next]!);
    gl.uniform1i(gl.getUniformLocation(this.copy, "src"), 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    this.cur = next;
  }

  dispose() {
    const gl = this.gl;
    this.tex.forEach((t) => gl.deleteTexture(t));
    this.fbo.forEach((f) => gl.deleteFramebuffer(f));
    gl.deleteBuffer(this.buf);
    gl.deleteProgram(this.draw_);
    gl.deleteProgram(this.copy);
  }
}

/// User shader: the file is the body that follows the shared header (helpers and
/// uniforms of `COMMON`), so it only writes `void main()`.
export class Custom extends Feedback {
  constructor(canvas: HTMLCanvasElement, source: string) {
    super(canvas, compiles(`${COMMON}\n${source}`));
  }
}

/// Compiles on a scratch canvas and returns `frag`, or throws. A canvas that got a
/// WebGL context can no longer be 2D, and the fallback to Bars needs it.
function compiles(frag: string): string {
  const gl = document.createElement("canvas").getContext("webgl2");
  if (!gl) throw new Error("WebGL2 unavailable");
  try {
    program(gl, frag);
  } finally {
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
  return frag;
}

export class Ambience extends Feedback {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, AMBIENCE);
  }
}

export class Battery extends Feedback {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, BATTERY);
  }
}

export class Water extends Feedback {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, WATER);
  }
}

export class Alchemy extends Feedback {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, ALCHEMY);
  }
}

export class Spiral extends Feedback {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, SPIRAL);
  }
}
