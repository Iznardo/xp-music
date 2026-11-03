<script lang="ts" generics="T">
  // Loading wrapper: XP "marquee" bar while loading, message and retry if it fails.
  import type { Snippet } from "svelte";
  import type { Resource } from "../../resource.svelte";
  import { t } from "../../i18n.svelte";

  let { res, children }: { res: Resource<T>; children: Snippet } = $props();
</script>

{#if res.loading && res.data === null}
  <div class="state">
    <div class="marquee"><span></span><span></span><span></span></div>
    <p>{t.views.loading}</p>
  </div>
{:else if res.error && res.data === null}
  <div class="state">
    <p class="err">{t.views.error}</p>
    <p>{res.error.message}</p>
    <button class="xp-btn" onclick={() => res.load()}>{t.views.retry}</button>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--text-muted);
  }
  .err {
    font-weight: bold;
    color: var(--error);
  }
  /* XP indeterminate progress bar: green blocks that advance. */
  .marquee {
    position: relative;
    width: 160px;
    height: 14px;
    border: 1px solid var(--input-border);
    border-radius: 3px;
    background: var(--view-bg);
    overflow: hidden;
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.2);
  }
  .marquee span {
    position: absolute;
    top: 2px;
    width: 8px;
    height: 8px;
    background: linear-gradient(to bottom, #aef0ae, #29b829 50%, #158f15);
    animation: slide 1.6s linear infinite;
  }
  .marquee span:nth-child(2) {
    animation-delay: 0.12s;
  }
  .marquee span:nth-child(3) {
    animation-delay: 0.24s;
  }
  @keyframes slide {
    from {
      left: -30px;
    }
    to {
      left: 170px;
    }
  }
</style>
