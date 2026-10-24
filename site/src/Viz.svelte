<script lang="ts">
  // The app's visualizations (src/lib/viz) following the home page music.
  // Until ▶ is pressed (the browser does not allow any sound before that) they run on
  // their own with a made-up rhythm: a 120 bpm pulse and undulating bands.
  import { onMount } from "svelte";
  import { Bars } from "../../src/lib/viz/canvas2d";
  import { EMPTY_FRAME, type VizData, type VizRenderer } from "../../src/lib/viz/types";
  import { player, vizFrame } from "./player.svelte";

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  let { make }: { make: (c: HTMLCanvasElement) => VizRenderer } = $props();
  let canvas: HTMLCanvasElement;

  function demo(t: number): VizData {
    const beat = Math.exp(-((t * 2) % 1) * 5);
    const bands = Array.from({ length: 64 }, (_, i) => {
      const f = i / 64;
      return Math.min(1, 0.55 * beat * (1 - f) ** 2 + 0.32 * Math.abs(Math.sin(t * 2.3 + i * 0.37)) * (1 - f * 0.6) + 0.06 * Math.random());
    });
    const wave = Array.from({ length: 256 }, (_, j) => Math.sin(j * 0.15 + t * 9) * Math.sin(j * 0.031 + t) * 0.7 * (0.5 + 0.5 * beat));
    return { bands, peaks: bands, wave, level: 0.35 + 0.4 * beat, beat, bass: beat };
  }

  onMount(() => {
    let r: VizRenderer;
    try {
      r = make(canvas);
    } catch {
      r = new Bars(canvas); // No WebGL2.
    }
    const scale = r.continuous ? 0.75 : Math.min(devicePixelRatio || 1, 1);
    const resize = () => {
      canvas.width = Math.max(1, Math.round(canvas.clientWidth * scale));
      canvas.height = Math.max(1, Math.round(canvas.clientHeight * scale));
      r.resize(canvas.width, canvas.height);
      r.draw(EMPTY_FRAME, 0);
    };
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    resize();
    const t0 = performance.now();
    let last = 0;
    // When paused, a few more frames are drawn (the trail fades out) and then it stops.
    let idle = 0;
    let raf = requestAnimationFrame(function loop(now) {
      raf = requestAnimationFrame(loop);
      if (now - last < 1000 / 30 - 2) return;
      last = now;
      const on = player.playing || (!player.started && !reduced);
      idle = on ? 0 : idle + 1;
      if (idle > 45) return;
      const t = (now - t0) / 1000;
      r.draw(vizFrame() ?? (on ? demo(t) : EMPTY_FRAME), t);
    });
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      r.dispose();
    };
  });
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
</style>
