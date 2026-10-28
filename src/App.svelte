<script lang="ts">
  import { onMount } from "svelte";
  import { MediaQuery } from "svelte/reactivity";
  import TitleBar from "./lib/components/xp/TitleBar.svelte";
  import ContextMenu from "./lib/components/xp/ContextMenu.svelte";
  import Dialogs from "./lib/components/xp/Dialogs.svelte";
  import DialogFrame from "./lib/components/xp/DialogFrame.svelte";
  import SideTasks from "./lib/components/wmp/SideTasks.svelte";
  import TransportBar from "./lib/components/wmp/TransportBar.svelte";
  import Lazy from "./lib/components/Lazy.svelte";
  import { commands } from "./lib/bindings";
  import { player } from "./lib/stores/player.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { auth } from "./lib/stores/auth.svelte";
  import { library } from "./lib/stores/library.svelte";
  import { local } from "./lib/stores/local.svelte";
  import { ui } from "./lib/stores/ui.svelte";
  import { updates } from "./lib/stores/updates.svelte";
  import { skins } from "./lib/stores/skins.svelte";
  import { router } from "./lib/router.svelte";
  import { handleShortcut } from "./lib/menus";
  import { appWindow } from "./lib/platform";
  import { SKIN, outlinePath, place, sidePodPath, skinLayout, tabLayerPath } from "./lib/skin";
  import PlaylistTab from "./lib/components/wmp/PlaylistTab.svelte";
  import { t } from "./lib/i18n.svelte";
  import logo from "./assets/logo.webp";
  import type { SearchFilter } from "./lib/bindings";

  let maximized = $state(false);
  let active = $state(true);

  // Like WMP: "Title - XP Music" while something is playing.
  const title = $derived(player.state?.track ? `${player.state.track.title} - ${t.appName}` : t.appName);
  $effect(() => {
    document.title = title;
  });

  // Theme and light/dark mode go on <html> so every CSS variable sees them.
  const systemDark = new MediaQuery("(prefers-color-scheme: dark)");
  $effect(() => {
    const appearance = settings.current?.appearance ?? "System";
    const dark = appearance === "Dark" || (appearance === "System" && systemDark.current);
    document.documentElement.dataset.mode = dark ? "dark" : "light";
  });
  $effect(() => {
    document.documentElement.dataset.theme = (settings.current?.theme ?? "Blue").toLowerCase();
  });

  // Piecewise skin (lib/skin.ts): side panel, body, playlist tab and pod.
  let winW = $state(0);
  let winH = $state(0);
  const L = $derived(winW && winH ? skinLayout(winW, winH, !ui.sideCollapsed) : null);
  const outline = $derived(L ? outlinePath(L) : "");
  const sidePod = $derived(L ? sidePodPath(L) : null);
  const tabLayer = $derived(L ? tabLayerPath(L) : null);
  let outlineEl = $state<SVGPathElement>();

  // The outline (with its joins) is the shape for clicks: clicks on transparent areas
  // pass through to whatever is behind. Maximized, it is rectangular; compact mode sets its own.
  $effect(() => {
    // A skin sets its own shape; when leaving it, this one is applied again.
    if (ui.compact || skins.active) return;
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

  // Views and dialogs live in separate bundle chunks (lazy loading). The main views
  // are preloaded in the background after startup: switching views never waits,
  // but the first paint only downloads what is essential.
  const views = {
    now: () => import("./lib/views/NowPlayingView.svelte"),
    home: () => import("./lib/views/HomeView.svelte"),
    search: () => import("./lib/views/SearchView.svelte"),
    library: () => import("./lib/views/LibraryView.svelte"),
    options: () => import("./lib/views/OptionsDialog.svelte"),
    login: () => import("./lib/views/LoginView.svelte"),
    compact: () => import("./lib/views/CompactPlayer.svelte"),
    skin: () => import("./lib/views/SkinPlayer.svelte"),
    skinPicker: () => import("./lib/views/SkinPicker.svelte"),
  };

  onMount(() => {
    const idle = setTimeout(() => {
      void views.now();
      void views.library();
      void views.home();
      void views.search();
    }, 1500);
    void router.restore();
    void commands.windowIsCompact().then((c) => (ui.compact = c));
    void settings.init();
    void player.init();
    void library.init();
    void local.init();
    void updates.init();
    void auth.init().then(() => {
      if (auth.signedIn) return void library.refresh();
      ui.loginOpen = true;
      // Without a session the YouTube library is empty: "My music" works on its own.
      if (router.tab === "library" && !router.isLocal) router.go({ name: "local", view: "songs" });
    });
    const win = appWindow();
    if (!win) return () => clearTimeout(idle);
    // "Back to full mode" from the tray icon (emergency exit).
    const offFull = import("@tauri-apps/api/event").then(({ listen }) =>
      listen("ui://full-mode", () => {
        if (skins.active) void skins.exit();
        else void ui.setCompact(false);
      }),
    );
    // Tiling window managers (Hyprland, sway…) flag any tiled window as maximized.
    // On Linux it is only treated as maximized (flat mode) if it also fills the whole
    // usable screen area; a half-screen window keeps the WMP outline.
    const linux = /Linux/.test(navigator.userAgent);
    const fillsScreen = () => innerWidth >= screen.availWidth - 2 && innerHeight >= screen.availHeight - 2;
    const sync = async () => (maximized = (await win.isMaximized()) && (!linux || fillsScreen()));
    void sync();
    const unlisteners = [win.onResized(sync), win.onFocusChanged((e) => (active = e.payload))];
    return () => {
      clearTimeout(idle);
      unlisteners.forEach((p) => p.then((off) => off()));
      void offFull.then((off) => off());
    };
  });

  // If the session expires and cannot renew itself, offer to reconnect.
  $effect(() => {
    if (auth.status && typeof auth.status === "object" && "Expired" in auth.status) ui.loginOpen = true;
  });
</script>

<svelte:window onkeydown={handleShortcut} />

{#if skins.active && skins.current}
  {#key skins.current}<Lazy load={views.skin} props={{ id: skins.current }} />{/key}
{:else if ui.compact}
  <Lazy load={views.compact} />
{:else}
<div class="skin" class:maximized class:inactive={!active} bind:clientWidth={winW} bind:clientHeight={winH}>
  {#if L}
    <!-- Shell: the full outline with its joins, behind the pieces. -->
    <svg class="shell" width={winW} height={winH} aria-hidden="true">
      <defs>
        <linearGradient id="shell-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" class="sh-1" />
          <stop offset="0.25" class="sh-2" />
          <stop offset="0.75" class="sh-3" />
          <stop offset="1" class="sh-4" />
        </linearGradient>
        <!-- Side panel and pod: the same color end to end (they are one piece). -->
        <linearGradient id="side-fill" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2={SKIN.sideW} y2="0">
          <stop offset="0" class="sf-edge" />
          <stop offset="0.55" class="sf-mid" />
          <!-- Beyond the side panel (the pod) the light tone continues. -->
          <stop offset="1" class="sf-mid" />
        </linearGradient>
        <linearGradient id="side-shade" gradientUnits="userSpaceOnUse" x1="0" y1={L.pod.y - 30} x2="0" y2={winH}>
          <!-- No effect above the pod; the highlight fades in over the 30px above it (no
               visible step), then turns into a shadow towards the bottom. -->
          <stop offset="0" stop-color="#fff" stop-opacity="0" />
          <stop offset={30 / (SKIN.pod.h + 30)} stop-color="#fff" stop-opacity="0.1" />
          <stop offset="1" stop-color="#000" stop-opacity="0.1" />
        </linearGradient>
        <!-- Playlist tab layer: raised control colors, fading into the shell at the bottom. -->
        <linearGradient id="tab-fill" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2={L.content.y}>
          <stop offset="0" class="tb-1" />
          <stop offset="0.6" class="tb-2" />
          <stop offset="1" class="tb-2" stop-opacity="0" />
        </linearGradient>
        <linearGradient id="tab-edge" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2={L.content.y}>
          <stop offset="0.4" class="tb-edge" />
          <stop offset="1" class="tb-edge" stop-opacity="0" />
        </linearGradient>
        <clipPath id="outline-clip"><path d={outline} /></clipPath>
      </defs>
      <path d={outline} bind:this={outlineEl} class="shell-fill" />
      {#if sidePod}
        <!-- The same-colored stroke hides the light thread of the silver underneath at the edge. -->
        <path d={sidePod} fill="url(#side-fill)" stroke="url(#side-fill)" stroke-width="1.5" />
        <!-- Top-to-bottom highlight over the controls area, seamless. -->
        <path d={sidePod} fill="url(#side-shade)" />
      {/if}
      {#if tabLayer}
        <!-- The tab is its own layer over the body: lighter, with a rim and a shadow on its S. -->
        <g clip-path="url(#outline-clip)">
          <path d={tabLayer.fill} fill="url(#tab-fill)" />
          <!-- Shadow only on the rim: under the faded fill it would show through. -->
          <path d={tabLayer.edge} fill="none" stroke="url(#tab-edge)" class="tab-rim" />
          <path d={tabLayer.edge} fill="none" stroke="#fff" stroke-opacity="0.5" transform="translate(1.5 0)" />
        </g>
      {/if}
    </svg>
    {#if L.side}
      <div
        class="piece side"
        style="{place(L.side)};clip-path:path('{L.sideClip}');padding-top:{SKIN.sideTop - L.side.y + 4}px;padding-right:{L.side.w - SKIN.sideW}px;padding-bottom:{SKIN.taper + SKIN.podCorner - SKIN.pod.h}px"
      >
        <SideTasks />
      </div>
    {/if}
    <div class="piece body" style={place(L.body)} data-tauri-drag-region></div>
    <div class="piece tab" style={place(L.tab)}><PlaylistTab {maximized} /></div>
    <div class="topbar" style={place(L.topBar)}><TitleBar /></div>
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
        <Lazy load={views.now} />
      {:else if router.route.name === "home"}
        <Lazy load={views.home} />
      {:else if router.route.name === "search"}
        {#key `${router.route.filter}|${router.route.query}`}
          <Lazy load={views.search} props={{ query: router.route.query, filter: router.route.filter as SearchFilter }} />
        {/key}
      {:else}
        <Lazy load={views.library} />
      {/if}
    </main>

    <TransportBar pod={L.pod} bar={L.bar} />
    {#if !maximized}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="grip" title={t.top.resize} style="left:{winW - 16}px;top:{L.body.y + L.body.h - 16}px" onpointerdown={resize}></div>
    {/if}
  {/if}
</div>
{/if}

<ContextMenu />
<Dialogs />
{#if ui.optionsOpen}<Lazy load={views.options} />{/if}
{#if ui.loginOpen}<Lazy load={views.login} />{/if}
{#if skins.pickerOpen}<Lazy load={views.skinPicker} />{/if}
{#if ui.aboutOpen}
  <DialogFrame title={t.menuBar.about} onclose={() => (ui.aboutOpen = false)}>
    <div class="about">
      <img src={logo} alt="" width="48" height="48" />
      <div>
        <b>{t.appName}</b> 0.1.0
        <p>{t.dialogs.aboutText}</p>
      </div>
    </div>
    <div class="about-buttons"><button class="xp-btn" onclick={() => (ui.aboutOpen = false)}>{t.dialogs.ok}</button></div>
  </DialogFrame>
{/if}

<style>
  /* Piecewise WMP 9 skin (see lib/skin.ts). The window is transparent: whatever no
     piece covers is not visible (and the Windows region does not include it either). */
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
  /* Side panel: blue, darker, below the body. */
  /* The side panel paints no background: the shell paints it together with the pod. */
  .side {
    z-index: 1;
    border: 0;
  }
  :global(.sh-1) {
    stop-color: var(--shell-1);
  }
  :global(.sh-2) {
    stop-color: var(--shell-2);
  }
  :global(.sh-3) {
    stop-color: var(--shell-3);
  }
  :global(.sh-4) {
    stop-color: var(--shell-4);
  }
  :global(.sf-edge) {
    stop-color: var(--side-bottom);
  }
  :global(.tb-1) {
    stop-color: var(--btn-1);
  }
  :global(.tb-2) {
    stop-color: var(--btn-2);
  }
  :global(.tb-edge) {
    stop-color: var(--btn-border);
  }
  .tab-rim {
    filter: drop-shadow(-2px 1px 2px rgb(0 0 0 / 0.35));
  }
  :global(.sf-mid) {
    stop-color: var(--side-top);
  }
  /* Body and tab paint nothing of their own: the silver comes from the shell, so
     there are no seams between them. */
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
    border: 1px solid var(--content-border);
    box-shadow: 0 0 0 1px var(--content-halo);
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
    background: linear-gradient(to right, var(--shell-1), var(--shell-3));
    color: var(--shell-ink);
    cursor: pointer;
  }
  .handle svg {
    width: 6px;
    height: 10px;
  }
  /* Square bottom-right corner with a resize grip. */
  .grip {
    position: absolute;
    z-index: 40;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
    background:
      linear-gradient(135deg, transparent 0 55%, var(--wmp-line) 55% 62%, transparent 62% 70%, var(--wmp-line) 70% 77%, transparent 77% 85%, var(--wmp-line) 85% 92%, transparent 92%);
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
