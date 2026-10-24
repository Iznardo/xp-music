<script lang="ts">
  // Ventana de diálogo XP dentro de la app (modal), con barra de título Luna.
  import type { Snippet } from "svelte";
  import { t } from "../../i18n.svelte";

  let { title, onclose, children, width }: { title: string; onclose: () => void; children: Snippet; width?: number } = $props();
</script>

<div class="overlay" role="presentation" onkeydown={(e) => e.key === "Escape" && onclose()}>
  <div class="dialog" role="dialog" aria-label={title} style:width={width ? `${width}px` : undefined}>
    <header>
      <span>{title}</span>
      <button class="x" aria-label={t.window.close} title={t.window.close} onclick={onclose}>
        <svg viewBox="0 0 11 11"><path d="M2 2l7 7M9 2l-7 7" stroke="currentColor" stroke-width="1.8" /></svg>
      </button>
    </header>
    <div class="body">{@render children()}</div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.08);
  }
  .dialog {
    max-width: calc(100vw - 40px);
    max-height: calc(100vh - 40px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 3px solid var(--frame);
    border-top: 0;
    border-radius: 8px 8px 0 0;
    box-shadow: 3px 3px 10px rgb(0 0 0 / 0.45);
  }
  header {
    display: flex;
    align-items: center;
    height: 26px;
    padding: 0 4px 0 8px;
    margin: 0 -3px;
    border-radius: 7px 7px 0 0;
    background: var(--title-grad);
    color: var(--title-text);
    font: bold 12px var(--font-title);
    text-shadow: var(--title-text-shadow);
  }
  header span {
    flex: 1;
  }
  .x {
    display: grid;
    place-items: center;
    width: 21px;
    height: 21px;
    padding: 0;
    border: 1px solid #fff;
    border-radius: 3px;
    background: var(--close-grad);
    color: #fff;
  }
  .x svg {
    width: 11px;
    height: 11px;
  }
  .body {
    padding: 12px;
    overflow: auto;
  }
</style>
