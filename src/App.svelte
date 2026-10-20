<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./lib/components/xp/TitleBar.svelte";
  import ContextMenu from "./lib/components/xp/ContextMenu.svelte";
  import Dialogs from "./lib/components/xp/Dialogs.svelte";
  import DialogFrame from "./lib/components/xp/DialogFrame.svelte";
  import SideTasks from "./lib/components/wmp/SideTasks.svelte";
  import TransportBar from "./lib/components/wmp/TransportBar.svelte";
  import LibraryView from "./lib/views/LibraryView.svelte";
  import HomeView from "./lib/views/HomeView.svelte";
  import SearchView from "./lib/views/SearchView.svelte";
  import NowPlayingView from "./lib/views/NowPlayingView.svelte";
  import OptionsDialog from "./lib/views/OptionsDialog.svelte";
  import LoginView from "./lib/views/LoginView.svelte";
  import CompactPlayer from "./lib/views/CompactPlayer.svelte";
  import { commands } from "./lib/bindings";
  import { player } from "./lib/stores/player.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { auth } from "./lib/stores/auth.svelte";
  import { library } from "./lib/stores/library.svelte";
  import { ui } from "./lib/stores/ui.svelte";
  import { updates } from "./lib/stores/updates.svelte";
  import { router } from "./lib/router.svelte";
  import { handleShortcut } from "./lib/menus";
  import { appWindow } from "./lib/platform";
  import { SHAPE, shellPath } from "./lib/shape";
  import { t } from "./lib/strings";
  import type { SearchFilter } from "./lib/bindings";

  let maximized = $state(false);
  let active = $state(true);

  // Como WMP: "Título - XP Music" mientras suena algo.
  const title = $derived(player.state?.track ? `${player.state.track.title} - ${t.appName}` : t.appName);

  // El tema se aplica en <html> para que lo vean todas las variables CSS.
  $effect(() => {
    document.documentElement.dataset.theme = (settings.current?.theme ?? "Blue").toLowerCase();
  });

  // Silueta de la ventana: se recalcula con el tamaño (sin forma al maximizar).
  let winW = $state(0);
  let winH = $state(0);
  const outline = $derived(winW && winH && !maximized ? shellPath(winW, winH) : null);
  let edgeEl = $state<SVGPathElement>();

  // La misma silueta como región de la ventana (Windows): fuera de ella no se pinta
  // nada ni se reciben clics. Se muestrea el trazado en píxeles físicos.
  $effect(() => {
    const path = edgeEl;
    const shaped = !ui.compact && outline !== null && path !== undefined;
    const id = setTimeout(() => {
      if (!shaped || !path) return void commands.windowSetShape(null);
      const dpr = devicePixelRatio || 1;
      const len = path.getTotalLength();
      const n = Math.min(600, Math.max(120, Math.round(len / 6)));
      const pts: [number, number][] = [];
      for (let i = 0; i < n; i++) {
        const p = path.getPointAtLength((i / n) * len);
        pts.push([Math.round(p.x * dpr), Math.round(p.y * dpr)]);
      }
      void commands.windowSetShape(pts);
    }, 60);
    return () => clearTimeout(id);
  });

  onMount(() => {
    void router.restore();
    void commands.windowIsCompact().then((c) => (ui.compact = c));
    void settings.init();
    void player.init();
    void library.init();
    void updates.init();
    void auth.init().then(() => {
      if (!auth.signedIn) ui.loginOpen = true;
      else void library.refresh();
    });
    const win = appWindow();
    if (!win) return;
    const sync = async () => (maximized = await win.isMaximized());
    void sync();
    const unlisteners = [win.onResized(sync), win.onFocusChanged((e) => (active = e.payload))];
    return () => unlisteners.forEach((p) => p.then((off) => off()));
  });

  // Si la sesión caduca y no se puede renovar sola, se ofrece reconectar.
  $effect(() => {
    if (auth.status && typeof auth.status === "object" && "Expired" in auth.status) ui.loginOpen = true;
  });
</script>

<svelte:window onkeydown={handleShortcut} />

{#if ui.compact}
  <CompactPlayer />
{:else}
<div
  class="window"
  class:maximized
  class:inactive={!active}
  bind:clientWidth={winW}
  bind:clientHeight={winH}
  style:clip-path={outline ? `path("${outline}")` : undefined}
  style:--shape-top="{maximized ? 0 : SHAPE.top}px"
  style:--shape-left="{maximized ? 0 : SHAPE.left}px"
  style:--shape-bottom="{maximized ? 0 : SHAPE.bottom}px"
>
  {#if outline}
    <svg class="outline" width={winW} height={winH} aria-hidden="true">
      <path d={outline} class="edge" bind:this={edgeEl} />
      <path d={outline} class="hi" />
    </svg>
  {/if}
  <TitleBar {title} {maximized} {active} />

  <div class="body">
  <SideTasks />
  <main class="main" class:dark={router.route.name === "now"}>
    {#if router.route.name === "now"}
      <NowPlayingView />
    {:else if router.route.name === "home"}
      <HomeView />
    {:else if router.route.name === "search"}
      {#key `${router.route.filter}|${router.route.query}`}
        <SearchView query={router.route.query} filter={router.route.filter as SearchFilter} />
      {/key}
    {:else}
      <LibraryView />
    {/if}
  </main>
  </div>

  <TransportBar />
</div>
{/if}

<ContextMenu />
<Dialogs />
{#if ui.optionsOpen}<OptionsDialog />{/if}
{#if ui.loginOpen}<LoginView />{/if}
{#if ui.aboutOpen}
  <DialogFrame title={t.menuBar.about} onclose={() => (ui.aboutOpen = false)}>
    <div class="about">
      <svg viewBox="0 0 16 16" width="40" height="40"><circle cx="8" cy="8" r="7" fill="#f4f4f4" stroke="#1d4ec4" /><path d="M6 4.5v7l5.5-3.5z" fill="#1d9b2c" /></svg>
      <div>
        <b>{t.appName}</b> 0.1.0
        <p>{t.dialogs.aboutText}</p>
      </div>
    </div>
    <div class="about-buttons"><button class="xp-btn" onclick={() => (ui.aboutOpen = false)}>{t.dialogs.ok}</button></div>
  </DialogFrame>
{/if}

<style>
  /* Marco asimétrico del skin WMP 9: la ventana es transparente y la forma la dan
     los radios (grande arriba a la izquierda, siguiendo el pod de ▶ abajo). */
  /* Silueta asimétrica (ver lib/shape.ts): el fondo del skin rellena la forma y los
     márgenes dejan sitio a la oreja, al borde abombado y al disco del ▶. */
  .window {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    padding-top: var(--shape-top);
    background: linear-gradient(to right, var(--skin-mid), var(--skin-low) 30%);
  }
  .window.inactive {
    filter: saturate(0.85);
  }
  .outline {
    position: absolute;
    inset: 0;
    z-index: 950;
    pointer-events: none;
    fill: none;
  }
  .outline .edge {
    stroke: var(--skin-edge);
    stroke-width: 4;
  }
  .outline .hi {
    stroke: var(--skin-hi);
    stroke-width: 1;
    transform: translate(1px, 1px);
    opacity: 0.6;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
    margin-left: var(--shape-left);
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--surface);
  }
  .main.dark {
    background: var(--np-bg);
  }
  .about {
    display: flex;
    gap: 12px;
    width: 300px;
  }
  .about p {
    white-space: pre-line;
    margin: 6px 0 0;
  }
  .about-buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 10px;
  }
</style>
