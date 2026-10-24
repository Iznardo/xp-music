<script lang="ts">
  // WMP 9 side bar tasks (like SideTasks in the app), here with the website sections.
  import { RELEASES, REPO, SECTIONS } from "./content";
  import { t } from "./i18n.svelte";
  import { release, site } from "./state.svelte";

  let { section, row = false }: { section: string; row?: boolean } = $props();

</script>

<nav class="side" class:row aria-label={t.nav.label}>
  {#if !row}
    <div class="user">
      <a class="account" href={release.data?.url ?? RELEASES} title={t.nav.latest}>
        <span class="dot"></span>{release.data?.tag ?? "XP Music"}
      </a>
    </div>
  {/if}
  {#each SECTIONS as s (s.id)}
    <a class="task" class:active={section === s.id} aria-current={section === s.id ? "page" : undefined} href="#/{s.id}">{t.sections[s.key]}</a>
  {/each}
  <a class="task" href={REPO}>{t.nav.source}</a>
  {#if !row}
    <button class="task" onclick={() => (site.optionsOpen = true)}>{t.nav.options}</button>
  {/if}
</nav>

<style>
  .side {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 4px 14px 0 4px;
    color: var(--side-text);
  }
  .task {
    position: relative;
    display: block;
    padding: 5px 4px 6px;
    border: 0;
    background: none;
    color: var(--side-text);
    font: bold 12px/14px var(--font-ui);
    text-align: left;
    text-decoration: none;
    text-shadow: var(--side-text-shadow);
    cursor: pointer;
  }
  .task::after {
    content: "";
    position: absolute;
    left: 2px;
    right: 0;
    bottom: 0;
    height: 1px;
    background: linear-gradient(to right, var(--side-line), transparent);
    box-shadow: 0 1px var(--side-line-shadow);
  }
  .task:hover {
    background: rgb(255 255 255 / 0.12);
  }
  .task.active {
    background: linear-gradient(to right, rgb(255 255 255 / 0.3), rgb(255 255 255 / 0.05));
    box-shadow: inset 0 2px rgb(255 255 255 / 0.85);
  }
  .user {
    display: flex;
    padding: 0 0 6px 2px;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 5px;
    border-radius: 3px;
    background: rgb(0 0 40 / 0.22);
    color: #fff;
    font-size: 10px;
    text-decoration: none;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #c8ffc8, #2fb52f 60%, #1a7a1a);
  }
  /* On mobile: in a row, scrolling horizontally. */
  .row {
    flex-direction: row;
    padding: 0 4px;
    overflow-x: auto;
    white-space: nowrap;
  }
  .row .task {
    padding: 8px 10px;
  }
  .row .task::after {
    display: none;
  }
</style>
