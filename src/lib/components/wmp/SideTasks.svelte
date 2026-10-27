<script lang="ts">
  // WMP 9 side bar (FR-004-02): the account on top and seven stacked tasks with search.
  // The piece (shape and background) is drawn by App; only the content goes here.
  import { router } from "../../router.svelte";
  import { auth } from "../../stores/auth.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { t } from "../../i18n.svelte";

  let query = $state("");
  let searchEl = $state<HTMLInputElement>();
  let searching = $state(false);

  const isLib = (s: string) => router.route.name === "library" && router.route.section === s;
  const tasks = $derived([
    { label: t.side.now, active: router.tab === "now", go: () => router.go({ name: "now" }) },
    { label: t.side.guide, active: router.tab === "home", go: () => router.go({ name: "home" }) },
    {
      label: t.side.library,
      active: router.tab === "library" && !isLib("liked") && !isLib("history"),
      // Without a YouTube session, the library is the music on this computer.
      go: () => router.go(auth.signedIn ? { name: "library", section: "playlists" } : { name: "local", view: "songs" }),
    },
    { label: t.side.liked, active: isLib("liked"), go: () => router.go({ name: "library", section: "liked" }) },
    { label: t.side.history, active: isLib("history"), go: () => router.go({ name: "library", section: "history" }) },
    {
      label: t.side.search,
      active: router.tab === "search" || searching,
      go: () => {
        searching = true;
        queueMicrotask(() => searchEl?.focus());
      },
    },
    { label: t.side.options, active: ui.optionsOpen, go: () => (ui.optionsOpen = true) },
  ]);

  function search(e: SubmitEvent) {
    e.preventDefault();
    if (query.trim()) router.go({ name: "search", filter: "All", query: query.trim() });
  }

  // When a search is reached another way (menu, shortcuts), mirror its text.
  $effect(() => {
    if (router.route.name === "search") {
      query = router.route.query;
      searching = true;
    }
  });
</script>

<nav class="side">
  <!-- The account, at the very top. -->
  <div class="user">
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
  {#each tasks as task, i}
    <button class="task" class:active={task.active} onclick={task.go}>
      {#each task.label.split("\n") as line}<span>{line}</span>{/each}
    </button>
    {#if i === 5 && searching}
      <form class="search" onsubmit={search}>
        <input bind:this={searchEl} placeholder={t.views.searchPlaceholder} bind:value={query} aria-label={t.views.searchPlaceholder} />
      </form>
    {/if}
  {/each}

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
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 5px 4px 6px;
    border: 0;
    background: none;
    color: var(--side-text);
    font: bold 12px/14px var(--font-ui);
    text-align: left;
    text-shadow: var(--side-text-shadow);
    cursor: pointer;
  }
  /* Separators: light line with a shadow, like WMP 9's. */
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
  .search {
    padding: 4px 0 6px 2px;
  }
  .search input {
    width: 100%;
    height: 19px;
    padding: 0 6px;
    border: 1px solid var(--skin-edge);
    border-radius: 9px;
    background: #fff;
    font: inherit;
    outline: none;
  }
  .user {
    display: flex;
    padding: 0 0 6px 2px;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 2px 5px;
    border: 0;
    border-radius: 3px;
    background: rgb(0 0 40 / 0.22);
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
</style>
