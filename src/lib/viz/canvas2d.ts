// Presets en Canvas 2D: Barras y Ondas (inspirados en "Bars and Waves" de WMP, no copiados).

import type { VizData as VizFrame, VizRenderer } from "./types";

abstract class Canvas2D implements VizRenderer {
  protected ctx: CanvasRenderingContext2D;
  protected w = 1;
  protected h = 1;
  readonly continuous = false;

  constructor(protected canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext("2d", { alpha: false })!;
  }

  resize(width: number, height: number) {
    this.w = width;
    this.h = height;
  }

  dispose() {}

  abstract draw(frame: VizFrame, t: number): void;
}

/// Barras con tope de pico que cae y reflejo en el "suelo".
export class Bars extends Canvas2D {
  private grad: CanvasGradient | null = null;
  private gradH = 0;

  draw(f: VizFrame) {
    const { ctx, w, h } = this;
    const floor = h * 0.78;
    if (!this.grad || this.gradH !== h) {
      this.grad = ctx.createLinearGradient(0, floor, 0, h * 0.08);
      this.grad.addColorStop(0, "#1fbf2f");
      this.grad.addColorStop(0.55, "#d8e83a");
      this.grad.addColorStop(0.85, "#f39a1e");
      this.grad.addColorStop(1, "#e8321e");
      this.gradH = h;
    }
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, w, h);

    // 48 barras: se agrupan las 64 bandas recortando los agudos extremos.
    const n = 48;
    const gap = Math.max(1, Math.round(w / n / 6));
    const bw = (w - gap * (n + 1)) / n;
    const maxH = floor - h * 0.08;
    ctx.fillStyle = this.grad;
    for (let i = 0; i < n; i++) {
      const v = f.bands[i] ?? 0;
      const x = gap + i * (bw + gap);
      const bh = Math.max(1, v * maxH);
      // Barras "segmentadas" como un ecualizador de los 2000.
      ctx.fillRect(x, floor - bh, bw, bh);
    }
    ctx.fillStyle = "rgba(0,0,0,0.38)";
    for (let y = floor - 3; y > h * 0.08; y -= 4) ctx.fillRect(0, y, w, 1);

    ctx.fillStyle = "#e9f4ff";
    for (let i = 0; i < n; i++) {
      const p = f.peaks[i] ?? 0;
      if (p < 0.01) continue;
      const x = gap + i * (bw + gap);
      ctx.fillRect(x, floor - p * maxH - 3, bw, 2);
    }

    // Reflejo.
    ctx.save();
    ctx.globalAlpha = 0.22;
    ctx.translate(0, floor * 2 + 2);
    ctx.scale(1, -1);
    ctx.drawImage(this.canvas, 0, 0, w, floor, 0, 0, w, floor);
    ctx.restore();
    const fade = ctx.createLinearGradient(0, floor, 0, h);
    fade.addColorStop(0, "rgba(0,0,0,0.2)");
    fade.addColorStop(1, "rgba(0,0,0,1)");
    ctx.fillStyle = fade;
    ctx.fillRect(0, floor + 1, w, h - floor);
  }
}

/// Osciloscopio con brillo sobre un espectro tenue.
export class Waves extends Canvas2D {
  draw(f: VizFrame) {
    const { ctx, w, h } = this;
    ctx.fillStyle = "rgba(0,4,16,0.55)";
    ctx.fillRect(0, 0, w, h);

    // Espectro de fondo en puntos.
    const n = f.bands.length;
    for (let i = 0; i < n; i++) {
      const v = f.bands[i] ?? 0;
      ctx.fillStyle = `hsla(${200 + i * 2}, 90%, 60%, ${0.15 + v * 0.5})`;
      const x = (i + 0.5) * (w / n);
      ctx.beginPath();
      ctx.arc(x, h * 0.5, 1 + v * h * 0.06, 0, Math.PI * 2);
      ctx.fill();
    }

    const mid = h / 2;
    const amp = h * 0.42;
    ctx.lineJoin = "round";
    for (const [width, color] of [
      [6, "rgba(60,160,255,0.18)"],
      [3, "rgba(120,200,255,0.45)"],
      [1.2, "#e8f6ff"],
    ] as const) {
      ctx.lineWidth = width;
      ctx.strokeStyle = color;
      ctx.beginPath();
      f.wave.forEach((s, i) => {
        const x = (i / (f.wave.length - 1)) * w;
        const y = mid - s * amp * (0.8 + f.level);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    }
  }
}
