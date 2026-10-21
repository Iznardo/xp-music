<script lang="ts" generics="P extends Record<string, unknown>">
  // Carga diferida de un componente: su código va en un trozo aparte del bundle y
  // solo se descarga la primera vez que se muestra (después queda en caché).
  import type { Component } from "svelte";

  let { load, props }: { load: () => Promise<{ default: Component<P> }>; props?: P } = $props();
</script>

{#await load() then mod}
  <mod.default {...(props ?? ({} as P))} />
{/await}
