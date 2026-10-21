<script lang="ts">
  // Barra lateral de WMP 9 (FR-004-02): siete tareas apiladas, búsqueda y emblema.
  // La pieza (forma y fondo) la dibuja App; aquí solo va el contenido.
  import { router } from "../../router.svelte";
  import { auth } from "../../stores/auth.svelte";
  import { ui } from "../../stores/ui.svelte";
  import { t } from "../../strings";

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
      go: () => router.go({ name: "library", section: "playlists" }),
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

  // Al llegar a una búsqueda por otra vía (menú, atajos), refleja el texto.
  $effect(() => {
    if (router.route.name === "search") {
      query = router.route.query;
      searching = true;
    }
  });
</script>

<nav class="side">
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
    <svg class="emblem" viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <radialGradient id="side-emblem" cx="40%" cy="30%" r="75%">
          <stop offset="0" stop-color="#d8ecff" />
          <stop offset="0.55" stop-color="#3d8be6" />
          <stop offset="1" stop-color="#0d3c9e" />
        </radialGradient>
      </defs>
      <circle cx="16" cy="16" r="13" fill="url(#side-emblem)" stroke="#fff" stroke-width="1.5" />
      <path d="M12.5 9.5v13l10-6.5z" fill="#ffa62a" stroke="#7a3300" stroke-width="0.8" />
    </svg>
  </div>
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
    padding: 5px 6px 6px;
    border: 0;
    background: none;
    color: var(--side-text);
    font: bold 12px/14px var(--font-ui);
    text-align: left;
    text-shadow: var(--side-text-shadow);
    cursor: pointer;
  }
  /* Separadores: línea clara con sombra, como los de WMP 9. */
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
  .foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0 0 6px;
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
  .emblem {
    width: 34px;
    height: 34px;
    filter: drop-shadow(0 1px 2px rgb(0 0 40 / 0.5));
  }
</style>
