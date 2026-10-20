<script lang="ts">
  // Lienzo de visualización (specs/005). El analizador del backend solo corre
  // mientras este componente está montado y la página es visible (AC-005-02).
  import { onMount } from "svelte";
  import { commands, events, type VizPreset } from "../bindings";
  import { EMPTY_FRAME, toData, type VizData, type VizRenderer } from "./types";
  import { Bars, Waves } from "./canvas2d";
  import { Alchemy, Ambience, Battery, Spiral, Water } from "./webgl";

  let { preset }: { preset: VizPreset } = $props();
  let canvas: HTMLCanvasElement;

  function create(c: HTMLCanvasElement): VizRenderer {
    try {
      if (preset === "Ambience") return new Ambience(c);
      if (preset === "Battery") return new Battery(c);
      if (preset === "Water") return new Water(c);
      if (preset === "Alchemy") return new Alchemy(c);
      if (preset === "Spiral") return new Spiral(c);
    } catch (e) {
      void commands.uiLog("warn", `visualización WebGL no disponible: ${String(e)}`);
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

    const resize = () => {
      const dpr = Math.min(devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.round(canvas.clientWidth * dpr));
      canvas.height = Math.max(1, Math.round(canvas.clientHeight * dpr));
      renderer.resize(canvas.width, canvas.height);
      dirty = true;
    };
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    resize();

    const loop = () => {
      if (renderer.continuous || dirty) {
        renderer.draw(frame, (performance.now() - t0) / 1000);
        dirty = false;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);

    const unlisten = events.vizFrame.listen((e) => {
      frame = toData(e.payload);
      dirty = true;
    });

    const sync = () => void commands.vizSetActive(document.visibilityState === "visible");
    sync();
    document.addEventListener("visibilitychange", sync);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener("visibilitychange", sync);
      void unlisten.then((off) => off());
      void commands.vizSetActive(false);
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
