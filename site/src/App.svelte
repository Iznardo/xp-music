<script lang="ts">
  // The website is an XP Music window on an XP desktop: the same WMP 9 shell the app
  // draws (src/lib/skin.ts), with the sections as tasks.
  import { SKIN, place, skinLayout } from "../../src/lib/skin";
  import Shell from "../../src/lib/components/wmp/Shell.svelte";
  import DialogFrame from "../../src/lib/components/xp/DialogFrame.svelte";
  import logo from "../../src/assets/logo.webp";
  import Side from "./Side.svelte";
  import Transport from "./Transport.svelte";
  import Home from "./views/Home.svelte";
  import Download from "./views/Download.svelte";
  import Screens from "./views/Screens.svelte";
  import Docs from "./views/Docs.svelte";
  import { REPO, SECTIONS } from "./content";
  import { route, setAppearance, setTheme, site, type Appearance, type Theme } from "./state.svelte";
  import { i18n, language, locale, setLanguage, t } from "./i18n.svelte";
  import type { Language } from "../../src/lib/locale";
  import { current, next, pause, player, toggle } from "./player.svelte";

  const r = $derived(route(site.hash));
  const label = $derived(t.sections[SECTIONS.find((s) => s.id === r.section)?.key ?? "home"]);
  // Like WMP: "Title - XP Music" while something plays.
  $effect(() => {
    document.title = player.playing
      ? `${current().title} - XP Music`
      : r.section
        ? `${label} - XP Music`
        : t.title;
  });

  let deskW = $state(0);
  let deskH = $state(0);
  let winW = $state(0);
  let winH = $state(0);
  let win = $state<"normal" | "max" | "min" | "closed">("normal");
  // Small screens: no desktop or shell, a single column.
  const mobile = $derived(deskW > 0 && (deskW < 760 || deskH < 500));
  const shown = $derived(mobile || win === "normal" || win === "max");
  const L = $derived(!mobile && winW && winH ? skinLayout(winW, winH, !site.sideCollapsed) : null);

  let comboOpen = $state(false);
  let comboEl = $state<HTMLElement>();

  let clock = $state(new Date());
  $effect(() => {
    const id = setInterval(() => (clock = new Date()), 15_000);
    return () => clearInterval(id);
  });

  const themes = $derived(Object.entries(t.options.themes) as [Theme, string][]);
  const appearances = $derived(Object.entries(t.options.appearances) as [Appearance, string][]);
  // Each language is named in itself, so it can be found whatever the current one is.
  const languages: [Language, string][] = [["en", "English"], ["es", "Español"]];
</script>

<svelte:window
  onclick={(e) => {
    if (comboOpen && !comboEl?.contains(e.target as Node)) comboOpen = false;
  }}
/>

{#snippet view()}
  {#if r.section === "download"}
    <Download />
  {:else if r.section === "screenshots"}
    <Screens />
  {:else if r.section === "docs"}
    <Docs slug={r.doc} />
  {:else}
    <Home />
  {/if}
{/snippet}

<div class="desktop" class:mobile bind:clientWidth={deskW} bind:clientHeight={deskH}>
  {#if !mobile}
    <button class="desk-icon" ondblclick={() => (win = "normal")} onkeydown={(e) => e.key === "Enter" && (win = "normal")} title={t.window.open}>
      <img src={logo} alt="" width="32" height="32" />
      <span>XP Music</span>
    </button>
  {/if}

  {#if shown}
    <div class="window" class:max={win === "max"} bind:clientWidth={winW} bind:clientHeight={winH}>
      {#if mobile}
        <header class="m-top">
          <img src={logo} alt="" width="22" height="22" />
          <b>XP Music</b>
          <span class="grow"></span>
          <button class="round" title={t.nav.options} aria-label={t.nav.options} onclick={() => (site.optionsOpen = true)}>
            <svg viewBox="0 0 16 16"><path d="M3 4h10M3 8h10M3 12h10" stroke="currentColor" stroke-width="1.6" /><circle cx="6" cy="4" r="1.6" /><circle cx="10" cy="8" r="1.6" /><circle cx="6" cy="12" r="1.6" /></svg>
          </button>
        </header>
        <div class="m-side"><Side row section={r.section} /></div>
        <main class="content m-content" class:dark={r.section === ""}>{@render view()}</main>
        <footer class="m-bar">
          <button class="round m-play" aria-label={player.playing ? t.player.pause : t.player.play} onclick={toggle}>
            {#if player.playing}
              <svg viewBox="0 0 16 16"><rect x="4" y="3" width="3" height="10" /><rect x="9" y="3" width="3" height="10" /></svg>
            {:else}
              <svg viewBox="0 0 16 16"><path d="M5 3v10l8-5z" /></svg>
            {/if}
          </button>
          <span class="m-lcd">{player.playing ? `${current().artist}: ${current().title}` : t.player.pressPlay}</span>
          <button class="round" aria-label={t.player.next} onclick={() => next(1)}>
            <svg viewBox="0 0 16 16"><path d="M3 4v8l7-4z" /><rect x="11" y="4" width="2" height="8" /></svg>
          </button>
        </footer>
      {:else if L}
        <Shell {L} w={winW} h={winH} />

        {#if L.side}
          <div
            class="piece side"
            style="{place(L.side)};clip-path:path('{L.sideClip}');padding-top:{SKIN.sideTop - L.side.y + 4}px;padding-right:{L.side.w - SKIN.sideW}px"
          >
            <Side section={r.section} />
          </div>
        {/if}

        <header class="topbar" style={place(L.topBar)}>
          <a class="menu" href="#/" title={t.nav.home} aria-label={t.nav.home}>
            <svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="4.2" fill="none" stroke="currentColor" stroke-width="1.8" /><circle cx="8" cy="8" r="1.4" /></svg>
          </a>
          <div class="rounds">
            <button
              title={site.sideCollapsed ? t.nav.showSide : t.nav.hideSide}
              aria-label={site.sideCollapsed ? t.nav.showSide : t.nav.hideSide}
              onclick={() => (site.sideCollapsed = !site.sideCollapsed)}
            >
              <svg viewBox="0 0 16 16"><rect x="2.5" y="3" width="11" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.4" /><rect x="3" y="3.5" width="3.5" height="9" /></svg>
            </button>
            <button title={t.nav.options} aria-label={t.nav.options} onclick={() => (site.optionsOpen = true)}>
              <svg viewBox="0 0 16 16"><path d="M3 4h10M3 8h10M3 12h10" stroke="currentColor" stroke-width="1.6" /><circle cx="6" cy="4" r="1.6" /><circle cx="10" cy="8" r="1.6" /><circle cx="6" cy="12" r="1.6" /></svg>
            </button>
          </div>
          <span class="grow"></span>
        </header>

        <div class="tab" style={place(L.tab)}>
          <div class="combo-wrap" bind:this={comboEl}>
            <button class="combo" aria-haspopup="menu" aria-expanded={comboOpen} onclick={() => (comboOpen = !comboOpen)}>
              <svg class="doc" viewBox="0 0 16 16" aria-hidden="true">
                <path d="M3 1.5h7l3 3v10H3z" fill="#fff" stroke="var(--btn-border)" />
                <path d="M5 6h6M5 8.5h6M5 11h4" stroke="var(--btn-border)" />
              </svg>
              <span class="label">{label}</span>
              <span class="count">{SECTIONS.length}</span>
              <span class="arrow"><svg viewBox="0 0 8 5"><path d="M0 0h8L4 5z" /></svg></span>
            </button>
            {#if comboOpen}
              <ul class="menu-list" role="menu">
                {#each SECTIONS as s (s.id)}
                  <li role="none">
                    <a role="menuitemradio" aria-checked={s.id === r.section} href="#/{s.id}" onclick={() => (comboOpen = false)}>{t.sections[s.key]}</a>
                  </li>
                {/each}
                <li class="sep" role="separator"></li>
                <li role="none"><a role="menuitem" href={REPO}>{t.nav.viewOnGithub}</a></li>
              </ul>
            {/if}
          </div>
          <div class="win">
            <button title={t.window.minimize} aria-label={t.window.minimize} onclick={() => (win = "min")}>
              <svg viewBox="0 0 9 9"><path d="M1 7h7" stroke="currentColor" stroke-width="1.6" /></svg>
            </button>
            <button
              title={win === "max" ? t.window.restore : t.window.maximize}
              aria-label={win === "max" ? t.window.restore : t.window.maximize}
              onclick={() => (win = win === "max" ? "normal" : "max")}
            >
              <svg viewBox="0 0 9 9"><rect x="1" y="1.5" width="7" height="6" fill="none" stroke="currentColor" stroke-width="1.3" /></svg>
            </button>
            <button class="close" title={t.window.close} aria-label={t.window.close} onclick={() => {
                win = "closed";
                pause();
              }}
            >
              <svg viewBox="0 0 9 9"><path d="M1.5 1.5l6 6M7.5 1.5l-6 6" stroke="currentColor" stroke-width="1.6" /></svg>
            </button>
          </div>
        </div>

        <button
          class="handle"
          style="left:{Math.max(0, L.content.x - 9)}px;top:{L.content.y + L.content.h / 2 - 22}px"
          title={site.sideCollapsed ? t.nav.showSide : t.nav.hideSide}
          aria-label={site.sideCollapsed ? t.nav.showSide : t.nav.hideSide}
          onclick={() => (site.sideCollapsed = !site.sideCollapsed)}
        >
          <svg viewBox="0 0 6 10"><path d={site.sideCollapsed ? "M1 1l4 4-4 4" : "M5 1L1 5l4 4"} fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
        </button>

        <main class="content" class:dark={r.section === ""} style={place(L.content)}>{@render view()}</main>

        <Transport pod={L.pod} bar={L.bar} />
      {/if}
    </div>
  {/if}
</div>

{#if !mobile}
  <footer class="taskbar">
    <button class="start" onclick={() => (win = win === "max" ? "max" : "normal")}>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M1 3.2l6-.9v5.4H1z" fill="#f65314" /><path d="M8 2.2l7-1v6.5H8z" fill="#7cbb00" />
        <path d="M1 8.6h6V14l-6-.9z" fill="#00a1f1" /><path d="M8 8.6h7v6.4l-7-1z" fill="#ffbb00" />
      </svg>
      {t.window.start}
    </button>
    {#if win !== "closed"}
      <button class="task" class:active={win !== "min"} onclick={() => (win = win === "min" ? "normal" : "min")}>
        <img src={logo} alt="" width="16" height="16" />
        XP Music
      </button>
    {/if}
    <span class="grow"></span>
    <div class="tray">
      <a href={REPO} title={t.window.github} aria-label={t.window.github}>
        <svg viewBox="0 0 16 16"><path fill="currentColor" d="M8 0a8 8 0 0 0-2.5 15.6c.4 0 .5-.2.5-.4v-1.5c-2.2.5-2.7-1-2.7-1-.4-.9-.9-1.2-.9-1.2-.7-.5.1-.5.1-.5.8.1 1.2.8 1.2.8.7 1.3 1.9.9 2.4.7 0-.5.3-.9.5-1.1-1.8-.2-3.6-.9-3.6-4 0-.9.3-1.6.8-2.1-.1-.2-.4-1 .1-2.1 0 0 .7-.2 2.2.8a7.5 7.5 0 0 1 4 0c1.5-1 2.2-.8 2.2-.8.4 1.1.2 1.9.1 2.1.5.6.8 1.3.8 2.1 0 3.1-1.9 3.8-3.6 4 .3.3.6.8.6 1.5v2.2c0 .2.1.5.6.4A8 8 0 0 0 8 0z" /></svg>
      </a>
      <time>{clock.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" })}</time>
    </div>
  </footer>
{/if}

{#if site.optionsOpen}
  <DialogFrame title={t.options.title} onclose={() => (site.optionsOpen = false)} width={300}>
    <fieldset>
      <legend>{t.options.theme}</legend>
      {#each themes as [id, name] (id)}
        <label class="radio"><input type="radio" name="theme" checked={site.theme === id} onchange={() => setTheme(id)} /> {name}</label>
      {/each}
    </fieldset>
    <fieldset>
      <legend>{t.options.appearance}</legend>
      {#each appearances as [id, name] (id)}
        <label class="radio"><input type="radio" name="appearance" checked={site.appearance === id} onchange={() => setAppearance(id)} /> {name}</label>
      {/each}
    </fieldset>
    <fieldset>
      <legend>{t.options.language}</legend>
      <select class="xp-input" value={i18n.choice ?? ""} onchange={(e) => setLanguage((e.currentTarget.value || null) as Language | null)}>
        <option value="">{t.options.languageAuto}</option>
        {#each languages as [id, name] (id)}<option value={id}>{name}</option>{/each}
      </select>
    </fieldset>
    <div class="dialog-buttons"><button class="xp-btn" onclick={() => (site.optionsOpen = false)}>{t.options.ok}</button></div>
  </DialogFrame>
{/if}

<style>
  .desktop {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .desk-icon {
    position: absolute;
    left: 14px;
    top: 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 76px;
    padding: 4px;
    border: 1px solid transparent;
    background: none;
    color: #fff;
    font-size: 11px;
    text-shadow: 1px 1px 2px #000;
    cursor: default;
  }
  .desk-icon:focus {
    outline: 1px dotted #fff;
  }
  .desk-icon:focus span {
    background: var(--selection);
  }
  .window {
    position: absolute;
    top: max(12px, calc((100% - 820px) / 2));
    bottom: max(12px, calc((100% - 820px) / 2));
    left: max(110px, calc((100% - 1240px) / 2));
    right: max(16px, calc((100% - 1240px) / 2));
  }
  .window.max {
    inset: 0;
  }
  /* The window's drop shadow on the desktop. */
  .window :global(.shell) {
    filter: drop-shadow(0 6px 14px rgb(0 0 0 / 0.45));
  }
  .piece,
  .topbar,
  .tab {
    position: absolute;
  }
  .side {
    z-index: 1;
  }
  .topbar {
    z-index: 4;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 12px;
  }
  .grow {
    flex: 1;
  }
  .menu,
  .rounds button,
  .round {
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--skin-edge);
    color: var(--shell-ink);
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 30%, var(--btn-1) 0%, var(--btn-2) 40%, var(--btn-3) 80%, var(--btn-4) 100%);
    box-shadow: inset 0 -2px 3px rgb(0 0 40 / 0.25), 0 1px 1px rgb(0 0 40 / 0.35);
    cursor: pointer;
  }
  .menu {
    width: 21px;
    height: 21px;
    color: #fff;
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 30%, var(--accent-1) 0%, var(--accent-2) 45%, var(--accent-3) 100%);
  }
  .menu svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }
  .rounds {
    display: flex;
    gap: 4px;
  }
  .rounds button,
  .round {
    width: 19px;
    height: 19px;
  }
  .rounds svg,
  .round svg {
    width: 11px;
    height: 11px;
    fill: currentColor;
  }
  .menu:hover,
  .rounds button:hover,
  .round:hover {
    filter: brightness(1.1);
  }

  /* Tab at the top right: sections dropdown and window buttons. */
  .tab {
    z-index: 6;
    display: flex;
    /* One row, centered in the tab: combo and window buttons share a midline. */
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 10px;
  }
  .combo-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .combo {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 23px;
    padding: 0 0 0 4px;
    border: 1px solid var(--btn-soft-border);
    border-radius: 2px;
    background: var(--view-bg);
    color: var(--text);
    font: 12px var(--font-ui);
    text-align: left;
    box-shadow: inset 1px 1px 2px rgb(0 0 40 / 0.2);
    cursor: pointer;
  }
  .doc {
    width: 15px;
    height: 15px;
    flex: none;
  }
  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    color: var(--text-faint);
    font-size: 10px;
  }
  .arrow {
    display: grid;
    place-items: center;
    width: 19px;
    height: 100%;
    border-left: 1px solid var(--btn-soft-border);
    background: linear-gradient(to bottom, var(--btn-1), var(--btn-3));
  }
  .arrow svg {
    width: 8px;
    height: 5px;
    fill: var(--btn-ink);
  }
  /* XP menu. */
  .menu-list {
    position: absolute;
    top: 24px;
    left: 0;
    right: 0;
    margin: 0;
    padding: 2px;
    list-style: none;
    background: var(--view-bg);
    border: 1px solid var(--border-dark);
    box-shadow: 2px 2px 3px rgb(0 0 0 / 0.35);
  }
  .menu-list a {
    display: block;
    padding: 3px 8px 3px 20px;
    color: var(--text);
    text-decoration: none;
  }
  .menu-list a[aria-checked="true"] {
    font-weight: bold;
  }
  .menu-list a:hover,
  .menu-list a:focus {
    background: var(--selection);
    color: #fff;
    outline: none;
  }
  .sep {
    height: 1px;
    margin: 3px 2px;
    background: var(--border-dark);
  }
  .win {
    display: flex;
    gap: 1px;
  }
  .win button {
    display: grid;
    place-items: center;
    width: 15px;
    height: 13px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--shell-ink);
    cursor: pointer;
  }
  /* Over the dark frame the thin glyphs need full white and a cut-out shadow to read. */
  :global([data-mode="dark"]) .win button {
    color: #fff;
    filter: drop-shadow(0 1px 0 rgb(0 0 0 / 0.9));
  }
  .win button:hover {
    color: #fff;
    background: var(--skin-mid);
    border-radius: 2px;
  }
  .win .close:hover {
    background: #d75c32;
  }
  .win svg {
    width: 9px;
    height: 9px;
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

  /* Mobile: a single column with the skin header and the tasks in a row. */
  .mobile .window {
    inset: 0;
    display: flex;
    flex-direction: column;
  }
  .m-top {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 10px;
    background: linear-gradient(to bottom, var(--shell-1), var(--shell-3));
    border-bottom: 1px solid var(--skin-edge);
    color: var(--shell-ink);
    font: bold 14px var(--font-title);
  }
  .m-side {
    background: linear-gradient(to bottom, var(--side-top), var(--side-bottom));
    border-bottom: 1px solid var(--skin-edge);
  }
  .m-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    padding: 0 10px;
    background: linear-gradient(to bottom, var(--shell-2), var(--shell-4));
    border-top: 1px solid var(--skin-edge);
  }
  .m-bar .round {
    width: 26px;
    height: 26px;
  }
  .m-bar .m-play {
    width: 34px;
    height: 34px;
    color: #fff;
    background: var(--btn-gloss, none), radial-gradient(circle at 50% 28%, var(--accent-1) 0%, var(--accent-2) 35%, var(--accent-3) 100%);
  }
  .m-bar svg {
    width: 13px;
    height: 13px;
  }
  .m-lcd {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--lcd-bg);
    color: var(--lcd-text);
    font-weight: bold;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .m-content {
    position: static;
    flex: 1;
    min-height: 0;
    border: 0;
    box-shadow: none;
  }

  /* XP taskbar. */
  .taskbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    background: var(--tb-grad);
    border-top: 1px solid var(--tb-line);
    color: #fff;
  }
  .start {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 100%;
    padding: 0 18px 2px 10px;
    border: 0;
    border-radius: 0 10px 10px 0;
    background: linear-gradient(to bottom, #3c8f3c 0%, #5eb35e 10%, #3a9a3a 30%, #2f8b2f 70%, #2a7a2a 95%, #1e5c1e 100%);
    box-shadow: inset -2px 0 3px rgb(0 0 0 / 0.3), 2px 0 3px rgb(0 0 0 / 0.25);
    color: #fff;
    font: italic bold 18px var(--font-title);
    text-shadow: 1px 1px 2px rgb(0 0 0 / 0.6);
    cursor: pointer;
  }
  .start:hover {
    filter: brightness(1.1);
  }
  .start svg {
    width: 20px;
    height: 20px;
    filter: drop-shadow(1px 1px 1px rgb(0 0 0 / 0.5));
  }
  .task {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 170px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid rgb(0 0 0 / 0.3);
    border-radius: 3px;
    background: var(--tb-task);
    color: #fff;
    text-align: left;
    box-shadow: inset 1px 1px rgb(255 255 255 / 0.3);
    cursor: pointer;
  }
  .task.active {
    background: var(--tb-task-active);
    box-shadow: inset 1px 1px 2px rgb(0 0 0 / 0.5);
  }
  .tray {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 100%;
    padding: 0 12px;
    background: var(--tb-tray);
    border-left: 1px solid var(--tb-line);
    box-shadow: inset 1px 0 rgb(255 255 255 / 0.2);
  }
  .tray a {
    display: grid;
    color: #fff;
  }
  .tray svg {
    width: 15px;
    height: 15px;
  }

  .radio {
    display: block;
    padding: 3px 0;
  }
  .dialog-buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 10px;
  }
</style>
