<script lang="ts">
  // The WMP 9 frame's painted layers (lib/skin.ts), shared by the app and the website:
  // the shell behind every piece (outline, side panel + pod, playlist tab layer) and the
  // raised edge of the side panel + pod, above the content. The pieces' contents (side
  // tasks, top strip, tab, content, transport) are laid out by the host.
  import { SKIN, outlinePath, sidePodEdge, sidePodPath, tabLayerPath, type SkinLayout } from "../../skin";

  let {
    L,
    w,
    h,
    outlineEl = $bindable(),
  }: { L: SkinLayout; w: number; h: number; outlineEl?: SVGPathElement } = $props();

  const outline = $derived(outlinePath(L));
  const sidePod = $derived(sidePodPath(L));
  const tabLayer = $derived(tabLayerPath(L));
  const podEdge = $derived(sidePodEdge(L));
</script>

<!-- Shell: the full outline with its joins, behind the pieces. -->
<svg class="shell" width={w} height={h} aria-hidden="true">
  <defs>
    <linearGradient id="shell-fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" class="sh-1" />
      <stop offset="0.25" class="sh-2" />
      <stop offset="0.75" class="sh-3" />
      <stop offset="1" class="sh-4" />
    </linearGradient>
    <!-- Side panel and pod: the same color end to end (they are one piece). -->
    <linearGradient id="side-fill" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2={SKIN.sideW} y2="0">
      <stop offset="0" class="sf-edge" />
      <stop offset="0.55" class="sf-mid" />
      <!-- Beyond the side panel (the pod) the light tone continues. -->
      <stop offset="1" class="sf-mid" />
    </linearGradient>
    <linearGradient id="side-shade" gradientUnits="userSpaceOnUse" x1="0" y1={L.pod.y - 30} x2="0" y2={h}>
      <!-- No effect above the pod; the highlight fades in over the 30px above it (no
           visible step), then turns into a shadow towards the bottom. -->
      <stop offset="0" stop-color="#fff" stop-opacity="0" />
      <stop offset={30 / (SKIN.pod.h + 30)} stop-color="#fff" stop-opacity="0.1" />
      <stop offset="1" stop-color="#000" stop-opacity="0.1" />
    </linearGradient>
    <!-- Playlist tab layer: raised control colors, fading into the shell at the bottom. -->
    <linearGradient id="tab-fill" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2={L.content.y}>
      <stop offset="0" class="tb-1" />
      <stop offset="0.6" class="tb-2" />
      <stop offset="1" class="tb-2" stop-opacity="0" />
    </linearGradient>
    <linearGradient id="tab-edge" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2={L.content.y}>
      <stop offset="0.4" class="tb-edge" />
      <stop offset="1" class="tb-edge" stop-opacity="0" />
    </linearGradient>
    <!-- Masks, not clip paths: masks are antialiased everywhere, while clip paths can be
         drawn hard-edged (jagged steps where they cut a line: the tab's S at the top
         edge, the raised edge's highlight). -->
    <mask id="outline-mask" maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
      <path d={outline} fill="#fff" />
    </mask>
    <mask id="side-pod-mask" maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
      <path d={sidePod} fill="#fff" />
    </mask>
  </defs>
  <path d={outline} bind:this={outlineEl} fill="url(#shell-fill)" />
  <!-- The same-colored stroke hides the light thread of the silver underneath at the edge. -->
  <path d={sidePod} fill="url(#side-fill)" stroke="url(#side-fill)" stroke-width="1.5" />
  <!-- Top-to-bottom highlight over the controls area, seamless. -->
  <path d={sidePod} fill="url(#side-shade)" />
  <!-- The tab is its own layer over the body: lighter, with a rim and a shadow on its S. -->
  <g mask="url(#outline-mask)">
    <path d={tabLayer.fill} fill="url(#tab-fill)" />
    <!-- Shadow only on the rim: under the faded fill it would show through. -->
    <path d={tabLayer.edge} fill="none" stroke="url(#tab-edge)" class="tab-rim" />
    <path d={tabLayer.edge} fill="none" stroke="#fff" stroke-opacity="0.5" transform="translate(1.5 0)" />
  </g>
</svg>

<!-- Raised edge of the side panel + pod, above the content so its rim lies on the
     content's halo. Same depth as the tab's S, as one line: a highlight on the piece's
     side (a wide stroke masked to the inside, whatever the edge's direction), then the
     rim on top, casting its shadow. -->
<svg class="shell-edge" width={w} height={h} aria-hidden="true">
  <defs>
    <!-- Inside the window outline, fading out before the body's bottom, where the pod's
         arc becomes the window's silhouette. -->
    <linearGradient id="edge-fade" gradientUnits="userSpaceOnUse" x1="0" y1={L.body.y + L.body.h - 16} x2="0" y2={L.body.y + L.body.h}>
      <stop offset="0" stop-color="#fff" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    <mask id="edge-mask" maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
      <path d={outline} fill="url(#edge-fade)" />
    </mask>
  </defs>
  <g mask="url(#edge-mask)">
    <path d={podEdge} fill="none" stroke="#fff" stroke-opacity="0.5" stroke-width="3" mask="url(#side-pod-mask)" />
    <path d={podEdge} fill="none" class="pod-rim" />
  </g>
</svg>

<style>
  .shell,
  .shell-edge {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .shell {
    z-index: 0;
  }
  /* Above the content (5), below the transport (20+). */
  .shell-edge {
    z-index: 6;
  }
  .sh-1 {
    stop-color: var(--shell-1);
  }
  .sh-2 {
    stop-color: var(--shell-2);
  }
  .sh-3 {
    stop-color: var(--shell-3);
  }
  .sh-4 {
    stop-color: var(--shell-4);
  }
  .sf-edge {
    stop-color: var(--side-bottom);
  }
  .sf-mid {
    stop-color: var(--side-top);
  }
  .tb-1 {
    stop-color: var(--btn-1);
  }
  .tb-2 {
    stop-color: var(--btn-2);
  }
  .tb-edge {
    stop-color: var(--btn-border);
  }
  .tab-rim {
    filter: drop-shadow(-2px 1px 2px rgb(0 0 0 / 0.35));
  }
  .pod-rim {
    stroke: var(--btn-border);
    filter: drop-shadow(2px 1px 2px rgb(0 0 0 / 0.35));
  }
</style>
