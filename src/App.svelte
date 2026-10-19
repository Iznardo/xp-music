<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import TitleBar from "./lib/components/xp/TitleBar.svelte";
  import DebugPlayer from "./lib/components/DebugPlayer.svelte";
  import { player } from "./lib/stores/player.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { t } from "./lib/strings";
  import type { Theme } from "./lib/bindings";

  const themes: Theme[] = ["Blue", "Olive", "Silver"];
  let maximized = $state(false);
  let active = $state(true);

  // El tema se aplica en <html> para que lo vean todas las variables CSS.
  $effect(() => {
    document.documentElement.dataset.theme = (settings.current?.theme ?? "Blue").toLowerCase();
  });

  onMount(() => {
    const win = getCurrentWindow();
    settings.init();
    player.init();
    const sync = async () => (maximized = await win.isMaximized());
    sync();
    const unlisteners = [win.onResized(sync), win.onFocusChanged((e) => (active = e.payload))];
    return () => unlisteners.forEach((p) => p.then((off) => off()));
  });
</script>

<div class="window" class:maximized class:inactive={!active}>
  <TitleBar title={t.appName} {maximized} {active} />

  <nav class="menubar">
    {#each Object.values(t.menu) as label}
      <span class="menu-item">{label}</span>
    {/each}
  </nav>

  <main class="content">
    <p class="placeholder">{t.placeholder}</p>

    <DebugPlayer />

    <fieldset>
      <legend>{t.themes.title}</legend>
      {#each themes as theme}
        <label class="radio">
          <input
            type="radio"
            name="theme"
            value={theme}
            checked={settings.current?.theme === theme}
            disabled={!settings.current}
            onchange={() => settings.update({ theme })}
          />
          {t.themes[theme]}
        </label>
      {/each}
    </fieldset>
  </main>

  <footer class="statusbar">
    {settings.error ? t.status.loadError : t.status.ready}
  </footer>
</div>

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
  .menubar {
    display: flex;
    padding: 1px 2px;
    border-bottom: 1px solid #fff;
    box-shadow: 0 1px var(--border-dark);
  }
  .menu-item {
    padding: 3px 7px;
  }
  .menu-item:hover {
    background: var(--selection);
    color: #fff;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .placeholder {
    margin: 0;
    color: var(--text-muted);
  }
  .radio {
    display: flex;
    align-items: center;
    padding: 2px 0;
  }
  .statusbar {
    height: 22px;
    padding: 4px 8px;
    border-top: 1px solid var(--border-dark);
    background: var(--surface);
    box-shadow: inset 0 1px #fff;
  }
</style>
