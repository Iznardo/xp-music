<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./lib/components/xp/TitleBar.svelte";
  import MenuBar from "./lib/components/xp/MenuBar.svelte";
  import ContextMenu from "./lib/components/xp/ContextMenu.svelte";
  import Dialogs from "./lib/components/xp/Dialogs.svelte";
  import DialogFrame from "./lib/components/xp/DialogFrame.svelte";
  import TaskBar from "./lib/components/wmp/TaskBar.svelte";
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
  import { router } from "./lib/router.svelte";
  import { handleShortcut } from "./lib/menus";
  import { appWindow } from "./lib/platform";
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

  onMount(() => {
    void router.restore();
    void commands.windowIsCompact().then((c) => (ui.compact = c));
    void settings.init();
    void player.init();
    void library.init();
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
<div class="window" class:maximized class:inactive={!active}>
  <TitleBar {title} {maximized} {active} />
  <MenuBar />
  <TaskBar />

  <main class="main">
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
  .window {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 3px solid var(--frame);
    border-top: none;
    border-radius: 8px 8px 0 0;
    overflow: hidden;
  }
  .window.inactive {
    border-color: var(--frame-inactive);
  }
  .window.maximized {
    border-width: 0;
    border-radius: 0;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
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
