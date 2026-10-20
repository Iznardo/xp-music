// Presets WebGL2 con realimentación (rastro): Ambiente y Batería.
// Inspirados en el espíritu de las visualizaciones de WMP, con shaders propios.

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
uniform float time;
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
  // Rastro: lo anterior gira, se aleja y se apaga poco a poco.
  float ang = 0.006 + level * 0.01;
  mat2 rot = mat2(cos(ang), -sin(ang), sin(ang), cos(ang));
  vec2 q = rot * c * (0.985 - level * 0.01);
  q.x /= res.x / res.y;
  vec3 trail = texture(prev, q + 0.5).rgb * 0.955;

  float r = length(c);
  float a = atan(c.y, c.x);
  float petals = abs(sin(a * 3.0 + time * 0.4));
  float radius = 0.16 + 0.22 * band(petals * 0.7) + 0.05 * sin(a * 6.0 - time);
  float ring = smoothstep(0.018, 0.0, abs(r - radius)) * (0.6 + level * 1.6);
  float core = smoothstep(0.09 + avg(0, 6) * 0.08, 0.0, r) * avg(0, 6);
  vec3 col = hsv(fract(time * 0.03 + r * 0.6 + petals * 0.1), 0.75, 1.0) * ring + vec3(0.6, 0.8, 1.0) * core;
  o = vec4(max(trail, col), 1.0);
}`;

const BATTERY = `${COMMON}
void main() {
  vec2 c = uv - 0.5;
  c.x *= res.x / res.y;
  // Caleidoscopio de 6 ejes.
  float r = length(c);
  float a = atan(c.y, c.x);
  float seg = 6.2831853 / 6.0;
  a = mod(a, seg);
  a = abs(a - seg * 0.5);
  vec2 k = vec2(cos(a), sin(a)) * r;
  // Túnel: lo anterior se acerca y gira.
  float zoom = 0.94 - avg(0, 8) * 0.06;
  float ang = 0.03 * sin(time * 0.3) + level * 0.04;
  mat2 rot = mat2(cos(ang), -sin(ang), sin(ang), cos(ang));
  vec2 q = rot * k * zoom;
  q.x /= res.x / res.y;
  vec3 trail = texture(prev, q + 0.5).rgb * (0.93 + level * 0.04);

  vec3 col = vec3(0.0);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float e = avg(i * 20, i * 20 + 20);
    // Dentro de la cuña: la simetría los multiplica en 12 destellos.
    float pa = (0.5 + 0.5 * sin(time * (0.7 + fi * 0.41) + fi * 2.1)) * seg * 0.5;
    float pr = 0.08 + fi * 0.09 + e * 0.18 + 0.03 * sin(time * (1.3 + fi));
    vec2 pos = vec2(cos(pa), sin(pa)) * pr;
    float d = length(k - pos);
    col += hsv(fract(time * 0.05 + fi * 0.33), 0.85, 1.0) * smoothstep(0.035 + e * 0.07, 0.0, d) * (0.6 + e * 2.0);
  }
  o = vec4(clamp(trail + col, 0.0, 1.0), 1.0);
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

  constructor(canvas: HTMLCanvasElement, frag: string) {
    const gl = canvas.getContext("webgl2", { antialias: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error("WebGL2 no disponible");
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
    // El rastro se calcula a media resolución: se nota poco y cuesta 4 veces menos.
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
    // 1) Nuevo fotograma a partir del anterior.
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo[next]!);
    gl.viewport(0, 0, this.w, this.h);
    gl.useProgram(this.draw_);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex[this.cur]!);
    gl.uniform1i(gl.getUniformLocation(this.draw_, "prev"), 0);
    gl.uniform1fv(gl.getUniformLocation(this.draw_, "bands"), f.bands);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "level"), f.level);
    gl.uniform1f(gl.getUniformLocation(this.draw_, "time"), t);
    gl.uniform2f(gl.getUniformLocation(this.draw_, "res"), this.w, this.h);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    // 2) A pantalla.
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
