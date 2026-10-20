<script lang="ts">
  // Panel de tareas vertical de WMP 9 (FR-004-02): secciones, búsqueda, cuenta y
  // una pestaña para ocultarlo.
  import { router } from "../../router.svelte";
  import { auth } from "../../stores/auth.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { t } from "../../strings";

  let query = $state("");
  let searchEl = $state<HTMLInputElement>();
  const tasks = [
    { id: "now", label: t.tabs.now, go: () => router.go({ name: "now" }) },
    { id: "home", label: t.tabs.home, go: () => router.go({ name: "home" }) },
    { id: "library", label: t.tabs.library, go: () => router.go({ name: "library", section: "playlists" }) },
    { id: "search", label: t.tabs.search, go: () => searchEl?.focus() },
  ] as const;

  function search(e: SubmitEvent) {
    e.preventDefault();
    if (query.trim()) router.go({ name: "search", filter: "All", query: query.trim() });
  }

  // Al llegar a una búsqueda por otra vía (menú, atajos), refleja el texto.
  $effect(() => {
    if (router.route.name === "search") query = router.route.query;
  });
</script>

<nav class="side" class:collapsed={ui.sideCollapsed}>
  {#if !ui.sideCollapsed}
    <div class="tasks">
      {#each tasks as task}
        <button class="task" class:active={router.tab === task.id} onclick={task.go}>
          {#each task.label.split("\n") as line}<span>{line}</span>{/each}
        </button>
      {/each}
      <form class="search" onsubmit={search}>
        <input bind:this={searchEl} placeholder={t.views.searchPlaceholder} bind:value={query} aria-label={t.views.searchPlaceholder} />
      </form>
    </div>

    <div class="foot">
      {#if auth.accountName}
        <button
          class="account"
          class:expired={!auth.signedIn}
          title={auth.signedIn ? "YouTube Music" : t.login.expired(auth.accountName)}
          onclick={() => (ui.loginOpen = !auth.signedIn)}
        >
          <span class="dot"></span><span class="name">{auth.accountName}</span>
        </button>
      {:else}
        <button class="account expired" onclick={() => (ui.loginOpen = true)}><span class="dot"></span>{t.options.connect}</button>
      {/if}
    </div>
  {/if}

  <button
    class="handle"
    title={ui.sideCollapsed ? t.side.show : t.side.hide}
    aria-label={ui.sideCollapsed ? t.side.show : t.side.hide}
    onclick={() => ui.toggleSide()}
  >
    <svg viewBox="0 0 6 10"><path d={ui.sideCollapsed ? "M1 1l4 4-4 4" : "M5 1L1 5l4 4"} fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
  </button>
</nav>

<style>
  .side {
    flex: none;
    position: relative;
    display: flex;
    flex-direction: column;
    width: 104px;
    padding-right: 9px;
    background: linear-gradient(to bottom, var(--side-top), var(--side-bottom) 70%, var(--skin-low));
    border-right: 1px solid var(--skin-edge);
    box-shadow: inset -1px 0 rgb(255 255 255 / 0.25);
    color: #fff;
  }
  .side.collapsed {
    width: 10px;
    padding-right: 0;
  }
  .tasks {
    display: flex;
    flex-direction: column;
    padding-top: 6px;
  }
  .task {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 6px 8px;
    border: 0;
    border-bottom: 1px solid rgb(255 255 255 / 0.28);
    box-shadow: 0 1px rgb(0 0 0 / 0.15);
    background: none;
    color: #fff;
    font-weight: bold;
    font-size: 11px;
    line-height: 13px;
    text-align: left;
    text-shadow: 0 1px 1px rgb(0 0 0 / 0.4);
    cursor: pointer;
  }
  .task:hover {
    background: rgb(255 255 255 / 0.14);
  }
  .task.active {
    background: linear-gradient(to right, rgb(255 255 255 / 0.38), rgb(255 255 255 / 0.12));
    color: #fff;
  }
  .search {
    padding: 8px 6px 0;
  }
  .search input {
    width: 100%;
    height: 19px;
    padding: 0 5px;
    border: 1px solid var(--skin-edge);
    border-radius: 9px;
    background: #fff;
    font: inherit;
    outline: none;
  }
  .foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    /* Deja sitio al disco del ▶, que sobresale sobre el panel. */
    padding: 8px 4px 30px;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 2px 4px;
    border: 0;
    border-radius: 3px;
    background: rgb(0 0 0 / 0.2);
    color: #fff;
    font-size: 10px;
    cursor: pointer;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #c8ffc8, #2fb62f);
  }
  .account.expired .dot {
    background: radial-gradient(circle at 35% 35%, #ffd0c0, #d0401a);
  }
  .handle {
    position: absolute;
    top: 50%;
    right: 0;
    width: 10px;
    height: 46px;
    margin-top: -23px;
    padding: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--skin-edge);
    border-right: 0;
    border-radius: 5px 0 0 5px;
    background: linear-gradient(to right, var(--skin-hi), var(--skin-mid));
    color: #fff;
    cursor: pointer;
  }
  .collapsed .handle {
    border-radius: 0 5px 5px 0;
    border: 1px solid var(--skin-edge);
    border-left: 0;
  }
  .handle svg {
    width: 6px;
    height: 10px;
  }
</style>
