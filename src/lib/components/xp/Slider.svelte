<script lang="ts">
  // XP-style horizontal slider (sunken track + pointed "thumb").
  let {
    value,
    min = 0,
    max = 100,
    label,
    dark = false,
    onchange,
  }: { value: number; min?: number; max?: number; label: string; dark?: boolean; onchange: (v: number) => void } = $props();

  let track: HTMLDivElement;
  let drag = $state<number | null>(null);
  const shown = $derived(drag ?? value);
  const pct = $derived(((shown - min) / (max - min || 1)) * 100);

  function at(e: PointerEvent) {
    const r = track.getBoundingClientRect();
    return Math.round(min + Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * (max - min));
  }
  function down(e: PointerEvent) {
    track.setPointerCapture(e.pointerId);
    drag = at(e);
  }
  function move(e: PointerEvent) {
    if (drag !== null) drag = at(e);
  }
  function up() {
    if (drag !== null) {
      onchange(drag);
      drag = null;
    }
  }
  function key(e: KeyboardEvent) {
    const step = (max - min) / 20;
    if (e.key === "ArrowRight" || e.key === "ArrowUp") onchange(Math.min(max, value + step));
    else if (e.key === "ArrowLeft" || e.key === "ArrowDown") onchange(Math.max(min, value - step));
    else return;
    e.preventDefault();
  }
</script>

<div
  class="slider"
  class:dark
  bind:this={track}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={shown}
  title={`${label}: ${Math.round(pct)}%`}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onkeydown={key}
>
  <div class="rail"><div class="fill" style:width="{pct}%"></div></div>
  <div class="knob" style:left="{pct}%"></div>
</div>

<style>
  .slider {
    position: relative;
    height: 18px;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }
  .rail {
    position: absolute;
    left: 0;
    right: 0;
    top: 7px;
    height: 4px;
    border: 1px solid;
    border-color: var(--rail-shadow) var(--rail-light) var(--rail-light) var(--rail-shadow);
    background: var(--rail-bg);
  }
  .dark .rail {
    border-color: #050a16 #3d5384 #3d5384 #050a16;
    background: #0b1428;
  }
  .fill {
    height: 100%;
    background: var(--slider-fill);
  }
  .dark .fill {
    background: linear-gradient(to bottom, #9fc7ff, #3d7ee0);
  }
  .knob {
    position: absolute;
    top: 2px;
    width: 10px;
    height: 14px;
    margin-left: -5px;
    border: 1px solid var(--btn-border);
    border-radius: 2px 2px 5px 5px;
    background: linear-gradient(to right, var(--btn-1), var(--btn-2) 50%, var(--btn-3));
    box-shadow: 0 1px 1px rgb(0 0 0 / 0.4);
  }
  .slider:focus-visible .knob {
    outline: 1px dotted var(--text);
    outline-offset: 1px;
  }
</style>
