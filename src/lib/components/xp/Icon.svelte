<script lang="ts">
  // 16 px icons drawn for the app (not Microsoft assets).
  import type { IconName } from "./types";
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
  // Unique ids per instance: if repeated, when the first one unmounts the others lose their gradient.
  const u = $props.id();
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  <defs>
    <linearGradient id="{u}-b" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#8fc0ff" /><stop offset="1" stop-color="#1f5fd0" /></linearGradient>
    <linearGradient id="{u}-y" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffe9a3" /><stop offset="1" stop-color="#e2a929" /></linearGradient>
    <linearGradient id="{u}-r" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ff9b8f" /><stop offset="1" stop-color="#c8261c" /></linearGradient>
    <linearGradient id="{u}-g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e9eef7" /><stop offset="1" stop-color="#8d9bb3" /></linearGradient>
  </defs>
  {#if name === "library"}
    <rect x="2" y="2" width="3" height="12" fill="url(#{u}-b)" stroke="#173c86" stroke-width=".6" />
    <rect x="6" y="3" width="3" height="11" fill="url(#{u}-y)" stroke="#8a5c08" stroke-width=".6" />
    <path d="M10 4.5l2.6-.8 2.4 9.6-2.6.8z" fill="url(#{u}-r)" stroke="#7d140e" stroke-width=".6" />
  {:else if name === "folder"}
    <path d="M1.5 4h5l1 1.5h7v8h-13z" fill="url(#{u}-y)" stroke="#9a6b0f" stroke-width=".7" />
  {:else if name === "playlist"}
    <rect x="1.5" y="2.5" width="11" height="11" rx="1" fill="#fff" stroke="#5a6f96" stroke-width=".8" />
    <path d="M3.5 5.5h7M3.5 8h7M3.5 10.5h4" stroke="#3a5aa0" stroke-width="1" />
    <circle cx="12.5" cy="12" r="2.3" fill="url(#{u}-b)" stroke="#173c86" stroke-width=".6" />
  {:else if name === "note"}
    <path d="M6 3l7-1.5v9" fill="none" stroke="#1e4fb3" stroke-width="1.4" />
    <path d="M6 3v9" stroke="#1e4fb3" stroke-width="1.4" />
    <ellipse cx="4.4" cy="12.3" rx="2.3" ry="1.8" fill="url(#{u}-b)" stroke="#173c86" stroke-width=".6" />
    <ellipse cx="11.4" cy="10.8" rx="2.3" ry="1.8" fill="url(#{u}-b)" stroke="#173c86" stroke-width=".6" />
  {:else if name === "heart"}
    <path d="M8 14S1.5 9.6 1.5 5.4A3.2 3.2 0 0 1 8 4a3.2 3.2 0 0 1 6.5 1.4C14.5 9.6 8 14 8 14z" fill="url(#{u}-r)" stroke="#7d140e" stroke-width=".7" />
  {:else if name === "disc"}
    <circle cx="8" cy="8" r="6.5" fill="url(#{u}-g)" stroke="#55627a" stroke-width=".7" />
    <circle cx="8" cy="8" r="2.2" fill="#fff" stroke="#55627a" stroke-width=".6" />
    <path d="M4 5a5 5 0 0 1 3-2" stroke="#fff" stroke-width="1" fill="none" />
  {:else if name === "person"}
    <circle cx="8" cy="5" r="3" fill="url(#{u}-y)" stroke="#8a5c08" stroke-width=".6" />
    <path d="M2.5 14.5c0-3.4 2.5-5.3 5.5-5.3s5.5 1.9 5.5 5.3z" fill="url(#{u}-b)" stroke="#173c86" stroke-width=".6" />
  {:else if name === "clock"}
    <circle cx="8" cy="8" r="6.5" fill="#fff" stroke="#1f5fd0" stroke-width="1.2" />
    <path d="M8 4v4l2.5 1.8" stroke="#000" stroke-width="1.1" fill="none" />
  {:else if name === "home"}
    <path d="M2 8l6-5.5L14 8" fill="none" stroke="#9a2a1c" stroke-width="1.3" />
    <path d="M3.5 7.5V14h9V7.5L8 3.6z" fill="url(#{u}-y)" stroke="#8a5c08" stroke-width=".6" />
    <rect x="6.6" y="10" width="2.8" height="4" fill="#3a5aa0" />
  {:else if name === "search"}
    <circle cx="6.5" cy="6.5" r="4.3" fill="#d8e9ff" stroke="#1e4fb3" stroke-width="1.4" />
    <path d="M9.8 9.8 14.5 14.5" stroke="#5a3b10" stroke-width="2.2" />
  {:else if name === "radio"}
    <rect x="1.5" y="5" width="13" height="9" rx="1.5" fill="url(#{u}-g)" stroke="#55627a" stroke-width=".7" />
    <circle cx="5.5" cy="9.5" r="2.4" fill="#3a4a66" />
    <path d="M10 8h3M10 10h3M10 12h3M4 5l7-3.5" stroke="#3a4a66" stroke-width=".9" />
  {/if}
</svg>

<style>
  .icon {
    flex: none;
    vertical-align: middle;
  }
</style>
