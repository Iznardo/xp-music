<script lang="ts">
  // Barra de tareas superior de WMP 10: pestañas grandes, búsqueda rápida y cuenta (FR-004-02).
  import { router } from "../../router.svelte";
  import { auth } from "../../stores/auth.svelte";
  import { t } from "../../strings";

  let query = $state("");
  const tabs = [
    { id: "now", label: t.tabs.now, go: () => router.go({ name: "now" }) },
    { id: "library", label: t.tabs.library, go: () => router.go({ name: "library", section: "playlists" }) },
    { id: "home", label: t.tabs.home, go: () => router.go({ name: "home" }) },
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

<nav class="taskbar">
  {#each tabs as tab}
    <button class="tab" class:active={router.tab === tab.id} onclick={tab.go}>
      {#each tab.label.split("\n") as line}<span>{line}</span>{/each}
    </button>
  {/each}

  <form class="search" class:active={router.tab === "search"} onsubmit={search}>
    <input placeholder={t.views.searchPlaceholder} bind:value={query} aria-label={t.views.searchPlaceholder} />
    <button type="submit" aria-label={t.tabs.search} title={t.tabs.search}>
      <svg viewBox="0 0 16 16"><circle cx="6.5" cy="6.5" r="4.2" fill="none" stroke="currentColor" stroke-width="1.7" /><path d="M9.6 9.6 14 14" stroke="currentColor" stroke-width="2" /></svg>
    </button>
  </form>

  <span class="spacer"></span>
  {#if auth.accountName}
    <span class="account" class:expired={!auth.signedIn} title={auth.signedIn ? "YouTube Music" : t.login.expired(auth.accountName)}>
      <span class="dot"></span>{auth.accountName}
    </span>
  {/if}
</nav>

<style>
  .taskbar {
    flex: none;
    display: flex;
    align-items: stretch;
    gap: 2px;
    height: 38px;
    padding: 3px 6px 0;
    background: linear-gradient(to bottom, var(--wmp-top) 0%, var(--wmp-mid) 40%, var(--wmp-bottom) 100%);
    border-bottom: 1px solid #000;
    box-shadow: inset 0 1px var(--wmp-line);
  }
  .tab {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 78px;
    padding: 0 12px;
    margin-bottom: 3px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    color: var(--wmp-text);
    font-weight: bold;
    font-size: 11px;
    line-height: 12px;
    cursor: pointer;
  }
  .tab:hover {
    border-color: var(--wmp-line);
    background: rgb(255 255 255 / 0.07);
    color: #fff;
  }
  .tab.active {
    color: #fff;
    border-color: var(--wmp-active-top) var(--wmp-active-bottom) var(--wmp-active-bottom);
    background: linear-gradient(to bottom, var(--wmp-active-top) 0%, var(--wmp-active-bottom) 100%);
    box-shadow: inset 0 1px rgb(255 255 255 / 0.45), 0 0 6px var(--wmp-active-glow);
    text-shadow: 0 1px rgb(0 0 0 / 0.5);
  }
  .search {
    display: flex;
    align-items: center;
    margin: 0 0 3px 10px;
    padding: 0 2px 0 6px;
    border-radius: 11px;
    background: var(--wmp-deep);
    border: 1px solid var(--wmp-line);
    box-shadow: inset 0 1px 2px #000;
    align-self: center;
    height: 22px;
  }
  .search.active {
    border-color: var(--wmp-active-top);
  }
  .search input {
    width: 190px;
    border: 0;
    outline: 0;
    background: none;
    color: #fff;
    font: inherit;
  }
  .search input::placeholder {
    color: var(--wmp-dim);
  }
  .search button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 18px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--wmp-dim);
    cursor: pointer;
  }
  .search svg {
    width: 13px;
    height: 13px;
  }
  .spacer {
    flex: 1;
  }
  .account {
    align-self: center;
    margin-bottom: 3px;
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--wmp-text);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #c8ffc8, #22b522);
    box-shadow: 0 0 4px #3f3;
  }
  .account.expired .dot {
    background: radial-gradient(circle at 35% 35%, #ffd2c8, #c8371a);
    box-shadow: 0 0 4px #f53;
  }
</style>
