<script lang="ts">
  // Playlist / album / artist header: cover, titles and buttons (FR-004-05).
  import type { Snippet } from "svelte";
  import type { Thumbnail } from "../../bindings";
  import { thumbSrc } from "../../thumb";

  let {
    kind,
    title,
    subtitle = null,
    stats = null,
    thumbs = [],
    round = false,
    actions,
  }: {
    kind: string;
    title: string;
    subtitle?: string | null;
    stats?: string | null;
    thumbs?: Thumbnail[];
    round?: boolean;
    actions?: Snippet;
  } = $props();
  const src = $derived(thumbSrc(thumbs, 120));
</script>

<header class="detail">
  <div class="art" class:round>{#if src}<img {src} alt="" />{/if}</div>
  <div class="info">
    <span class="kind">{kind}</span>
    <h1>{title}</h1>
    {#if subtitle}<span class="sub">{subtitle}</span>{/if}
    {#if stats}<span class="stats">{stats}</span>{/if}
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
</header>

<style>
  .detail {
    flex: none;
    display: flex;
    gap: 14px;
    padding: 10px 12px;
    background: linear-gradient(to bottom, var(--pane-1), var(--pane-2));
    border: 1px solid var(--rule);
    margin-bottom: 6px;
  }
  .art {
    flex: none;
    width: 100px;
    height: 100px;
    border: 1px solid var(--input-border);
    background: var(--art-bg);
    box-shadow: 2px 2px 4px rgb(0 0 0 / 0.25);
  }
  .art.round {
    border-radius: 50%;
    overflow: hidden;
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
  }
  .kind {
    color: var(--text-faint);
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.5px;
  }
  h1 {
    margin: 0;
    font: bold 18px var(--font-title);
    color: var(--heading);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    color: var(--text-muted);
  }
  .stats {
    color: var(--text-faint);
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
</style>
