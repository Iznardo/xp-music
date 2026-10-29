<script lang="ts">
  // Home: YouTube Music recommendations in rows of large icons (FR-004-07).
  import { commands, type Page, type Section } from "../bindings";
  import { Resource } from "../resource.svelte";
  import { auth } from "../stores/auth.svelte";
  import { t } from "../i18n.svelte";
  import TileGrid from "../components/wmp/TileGrid.svelte";
  import Loading from "../components/xp/Loading.svelte";

  const res = new Resource<Page<Section>>(() => commands.ytHome(null), () => "home");
  $effect(() => {
    if (auth.signedIn && !res.data) void res.load();
  });

  let loadingMore = $state(false);
  async function more() {
    const d = res.data;
    if (!d?.continuation || loadingMore) return;
    loadingMore = true;
    const r = await commands.ytHome(d.continuation);
    loadingMore = false;
    if (r.status === "ok") res.data = { items: [...d.items, ...r.data.items], continuation: r.data.continuation };
  }

  // Loads more sections when reaching the end, like the web app.
  function scroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 200) void more();
  }
</script>

<div class="home" onscroll={scroll}>
  {#if !auth.signedIn}
    <p class="hint">{t.views.signInFirst}</p>
  {:else}
    <Loading {res}>
      {#each res.data?.items ?? [] as s}
        <section>
          <h2>{s.title}</h2>
          <TileGrid items={s.items} row />
        </section>
      {/each}
      {#if res.data?.continuation}
        <div class="more"><button class="xp-btn" disabled={loadingMore} onclick={more}>{loadingMore ? t.views.loading : t.views.more}</button></div>
      {/if}
    </Loading>
  {/if}
</div>

<style>
  .home {
    flex: 1;
    overflow: auto;
    padding: 8px 14px;
    background: linear-gradient(to bottom, var(--view-bg), var(--view-alt) 300px);
  }
  section {
    margin-bottom: 8px;
  }
  h2 {
    margin: 6px 0 2px;
    font: bold 14px var(--font-title);
    color: var(--heading);
    border-bottom: 1px solid var(--rule);
    padding-bottom: 2px;
  }
  .more {
    padding: 10px;
    text-align: center;
  }
  .hint {
    color: var(--text-muted);
    padding: 20px;
    text-align: center;
  }
</style>
