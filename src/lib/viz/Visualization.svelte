<script lang="ts" module>
  // Varias visualizaciones pueden convivir un instante (al cambiar de preset se
  // monta la nueva antes de desmontar la vieja): el analizador del backend se
  // enciende con la primera visible y se apaga cuando no queda ninguna.
  import { commands as cmds } from "../bindings";
  let visible = 0;
  export function vizVisible(delta: 1 | -1) {
    const before = visible;
    visible = Math.max(0, visible + delta);
    if (before === 0 && visible > 0) void cmds.vizSetActive(true);
    if (before > 0 && visible === 0) void cmds.vizSetActive(false);
  }
</script>

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

    // Cuenta como visible mientras la página lo está.
    let counted = false;
    const sync = () => {
      const now = document.visibilityState === "visible";
      if (now !== counted) {
        counted = now;
        vizVisible(now ? 1 : -1);
      }
    };
    sync();
    document.addEventListener("visibilitychange", sync);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener("visibilitychange", sync);
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
