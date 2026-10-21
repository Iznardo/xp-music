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
  import { SKIN, outlinePath, place, skinLayout } from "./lib/skin";
  import PlaylistTab from "./lib/components/wmp/PlaylistTab.svelte";
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

  // Skin por piezas (lib/skin.ts): lateral, cuerpo, pestaña de la lista y píldora.
  let winW = $state(0);
  let winH = $state(0);
  const L = $derived(winW && winH ? skinLayout(winW, winH, !ui.sideCollapsed) : null);
  const outline = $derived(L ? outlinePath(L) : "");
  let outlineEl = $state<SVGPathElement>();

  // El contorno (con sus empalmes) es la región de la ventana (Windows): fuera no
  // se pinta nada ni se reciben clics. Maximizada o compacta, rectangular.
  $effect(() => {
    const path = outlineEl;
    const shaped = !ui.compact && !maximized && !!outline && path !== undefined;
    void outline;
    const id = setTimeout(() => {
      if (!shaped || !path) return void commands.windowSetShape(null);
      const dpr = devicePixelRatio || 1;
      const len = path.getTotalLength();
      const n = Math.min(1200, Math.max(200, Math.round(len / 3)));
      const pts: [number, number][] = [];
      for (let i = 0; i < n; i++) {
        const q = path.getPointAtLength((i / n) * len);
        pts.push([Math.round(q.x * dpr), Math.round(q.y * dpr)]);
      }
      void commands.windowSetShape([pts]);
    }, 40);
    return () => clearTimeout(id);
  });

  function resize(e: PointerEvent) {
    e.preventDefault();
    void appWindow()?.startResizeDragging("SouthEast");
  }

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
<div class="skin" class:maximized class:inactive={!active} bind:clientWidth={winW} bind:clientHeight={winH}>
  {#if L}
    <!-- Carcasa: el contorno completo con los empalmes, detrás de las piezas. -->
    <svg class="shell" width={winW} height={winH} aria-hidden="true">
      <defs>
        <linearGradient id="shell-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#eef2fc" />
          <stop offset="0.25" stop-color="#cdd7f0" />
          <stop offset="0.75" stop-color="#b4c3e8" />
          <stop offset="1" stop-color="#9eb0dc" />
        </linearGradient>
      </defs>
      <path d={outline} bind:this={outlineEl} class="shell-fill" />
    </svg>
    {#if L.side}
      <div
        class="piece side"
        style="{place(L.side)};clip-path:path('{L.sideClip}');padding-top:{SKIN.sideTop - L.side.y + 4}px"
      >
        <SideTasks />
      </div>
    {/if}
    <div class="piece body" style={place(L.body)} data-tauri-drag-region></div>
    <div class="piece tab" style={place(L.tab)}><PlaylistTab {maximized} /></div>
    <div class="topbar" style={place(L.topBar)}><TitleBar {title} /></div>
    <button
      class="handle"
      style="left:{Math.max(0, L.content.x - 9)}px;top:{L.content.y + L.content.h / 2 - 22}px"
      title={ui.sideCollapsed ? t.side.show : t.side.hide}
      aria-label={ui.sideCollapsed ? t.side.show : t.side.hide}
      onclick={() => ui.toggleSide()}
    >
      <svg viewBox="0 0 6 10"><path d={ui.sideCollapsed ? "M1 1l4 4-4 4" : "M5 1L1 5l4 4"} fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
    </button>

    <main class="content" class:dark={router.route.name === "now"} style={place(L.content)}>
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

    <TransportBar pod={L.pod} bar={L.bar} />
    <!-- Borde exterior por encima de todo: la carcasa se lee como una sola pieza. -->
    {#if !maximized}
      <svg class="edge" width={winW} height={winH} aria-hidden="true"><path d={outline} /></svg>
    {/if}
    {#if !maximized}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="grip" title={t.top.resize} style="left:{winW - 16}px;top:{L.body.y + L.body.h - 16}px" onpointerdown={resize}></div>
    {/if}
  {/if}
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
  /* Skin WMP 9 por piezas (ver lib/skin.ts). La ventana es transparente: lo que no
     cubre ninguna pieza no se ve (y la región de Windows tampoco lo incluye). */
  .skin {
    position: relative;
    height: 100%;
  }
  .skin.maximized {
    background: var(--skin-low);
  }
  .skin.inactive .piece {
    filter: saturate(0.8);
  }
  .shell {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
  }
  .skin.maximized .shell {
    display: none;
  }
  .shell-fill {
    fill: url(#shell-fill);
  }
  .piece {
    position: absolute;
    border: 1px solid var(--skin-edge);
  }
  /* Lateral: azul, más oscura, por debajo del cuerpo. */
  .side {
    z-index: 1;
    border: 0;
    background: linear-gradient(to right, var(--side-bottom), var(--side-top) 55%, var(--side-bottom));
    box-shadow: inset 2px 2px 1px rgb(255 255 255 / 0.45), inset -2px -2px 2px rgb(0 0 50 / 0.3);
  }
  /* Cuerpo y pestaña no pintan nada propio: el plateado es el de la carcasa, así
     no hay costuras entre ellos. */
  .body,
  .tab {
    border: 0;
  }
  .body {
    z-index: 2;
  }
  .tab {
    z-index: 3;
  }
  .edge {
    position: absolute;
    inset: 0;
    z-index: 50;
    pointer-events: none;
    fill: none;
    stroke: var(--skin-edge);
    stroke-width: 2;
  }
  .skin.maximized .edge {
    display: none;
  }
  .topbar {
    position: absolute;
    z-index: 4;
  }
  .content {
    position: absolute;
    z-index: 5;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface);
    border: 1px solid #5b6fa6;
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.6);
    overflow: hidden;
  }
  .content.dark {
    background: var(--np-bg);
    border-color: #000;
  }
  .handle {
    position: absolute;
    z-index: 6;
    width: 9px;
    height: 44px;
    padding: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--skin-edge);
    border-radius: 5px 0 0 5px;
    background: linear-gradient(to right, #fff, #b4c3e8);
    color: var(--skin-deep);
    cursor: pointer;
  }
  .handle svg {
    width: 6px;
    height: 10px;
  }
  /* Esquina inferior derecha recta con agarre para cambiar el tamaño. */
  .grip {
    position: absolute;
    z-index: 40;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
    background:
      linear-gradient(135deg, transparent 0 55%, #6d82b8 55% 62%, transparent 62% 70%, #6d82b8 70% 77%, transparent 77% 85%, #6d82b8 85% 92%, transparent 92%);
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
