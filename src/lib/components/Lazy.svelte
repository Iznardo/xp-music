<script lang="ts" generics="P extends Record<string, unknown>">
  // Lazy loading of a component: its code goes in a separate bundle chunk and is
  // only downloaded the first time it is shown (then it stays cached).
  import type { Component } from "svelte";

  let { load, props }: { load: () => Promise<{ default: Component<P> }>; props?: P } = $props();
</script>

{#await load() then mod}
  <mod.default {...(props ?? ({} as P))} />
{/await}
