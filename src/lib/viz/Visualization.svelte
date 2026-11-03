<script lang="ts">
  // Visualization canvas (specs/005). The backend analyzer only runs while this
  // component is mounted and the page is visible (AC-005-02).
  import { onMount } from "svelte";
  import { vizVisible } from "./active";
  import { appWindow } from "../platform";
  import { commands, events, type VizPreset } from "../bindings";
  import { EMPTY_FRAME, toData, type VizData, type VizRenderer } from "./types";
  import { Bars, Waves } from "./canvas2d";
  import { Alchemy, Ambience, Battery, Custom, Spiral, Water } from "./webgl";

  // `source`: GLSL of a user visualization (wins over `preset`).
  let { preset, source }: { preset: VizPreset; source?: string } = $props();
  let canvas: HTMLCanvasElement;

  function create(c: HTMLCanvasElement): VizRenderer {
    try {
      if (source) return new Custom(c, source);
      if (preset === "Ambience") return new Ambience(c);
      if (preset === "Battery") return new Battery(c);
      if (preset === "Water") return new Water(c);
      if (preset === "Alchemy") return new Alchemy(c);
      if (preset === "Spiral") return new Spiral(c);
    } catch (e) {
      void commands.uiLog("warn", `WebGL visualization unavailable: ${String(e)}`);
      return new Bars(c);
    }
    return preset === "Waves" ? new Waves(c) : new Bars(c);
  }

  onMount(() => {
    const renderer = create(canvas);
    let frame: VizData = EMPTY_FRAME;
    let dirty = true;
    let raf = 0;
    const t0 = performance.now();

    // Savings (008): WebGL effects are drawn at 75 % of the screen resolution
    // (stretched without it showing: they are soft) and the bars at 1:1.
    const scale = renderer.continuous ? 0.75 : Math.min(devicePixelRatio || 1, 1);
    const resize = () => {
      canvas.width = Math.max(1, Math.round(canvas.clientWidth * scale));
      canvas.height = Math.max(1, Math.round(canvas.clientHeight * scale));
      renderer.resize(canvas.width, canvas.height);
      dirty = true;
    };
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    resize();

    // At most 30 fps: that is the rate the audio analysis arrives at, so drawing
    // more often only wastes CPU/GPU.
    const MIN_FRAME_MS = 1000 / 30 - 2;
    let last = 0;
    const loop = (now: number) => {
      raf = counted ? requestAnimationFrame(loop) : 0;
      if (now - last < MIN_FRAME_MS) return;
      last = now;
      if (renderer.continuous || dirty) {
        renderer.draw(frame, (now - t0) / 1000);
        dirty = false;
      }
    };
    const unlisten = events.vizFrame.listen((e) => {
      frame = toData(e.payload);
      dirty = true;
    });

    // Counts as visible while the page is. WebView2 doesn't report it as hidden
    // when the window is minimized: that is asked separately (otherwise it keeps
    // drawing and analyzing audio with the window in the taskbar).
    const win = appWindow();
    let counted = false;
    let minimized = false;
    let disposed = false;
    const sync = () => {
      if (disposed) return;
      const now = document.visibilityState === "visible" && !minimized;
      if (now !== counted) {
        counted = now;
        vizVisible(now ? 1 : -1);
        if (now && !raf) raf = requestAnimationFrame(loop);
      }
    };
    const syncMinimized = async () => {
      minimized = (await win?.isMinimized()) ?? false;
      sync();
    };
    sync();
    void syncMinimized();
    document.addEventListener("visibilitychange", sync);
    const offResized = win?.onResized(() => void syncMinimized());

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener("visibilitychange", sync);
      void offResized?.then((off) => off());
      void unlisten.then((off) => off());
      if (counted) vizVisible(-1);
      renderer.dispose();
    };
  });
</script>

<canvas bind:this={canvas} class="viz"></canvas>

<style>
  .viz {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    background: #000;
  }
</style>
