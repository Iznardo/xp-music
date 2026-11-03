<script lang="ts">
  // Skin picker, like WMP's: the list on the left and a preview on the right
  // (the real skin, small and untouchable).
  import { onMount } from "svelte";
  import { commands } from "../bindings";
  import DialogFrame from "../components/xp/DialogFrame.svelte";
  import { skins, SKIN_BASE } from "../stores/skins.svelte";
  import { isTauri } from "../platform";
  import { t } from "../i18n.svelte";

  const BOX = { w: 300, h: 230 };

  let selected = $state<string | null>(skins.current);
  let preview = $state<{ w: number; h: number; title: string } | null>(null);
  let failed = $state(false);
  let frame = $state<HTMLIFrameElement>();

  const scale = $derived(preview ? Math.min(1, BOX.w / preview.w, BOX.h / preview.h) : 1);

  function close() {
    skins.pickerOpen = false;
  }
  async function apply() {
    if (!selected) return;
    close();
    await skins.enter(selected);
  }

  function onMessage(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow) return;
    const m = e.data as Record<string, unknown> | null;
    if (m?.t === "ready") preview = { w: Number(m.w) || 1, h: Number(m.h) || 1, title: String(m.title ?? "") };
    else if (m?.t === "error") failed = true;
    // Everything else (commands, window…) is ignored: it is only a preview.
  }

  function load() {
    if (!selected) return;
    frame?.contentWindow?.postMessage({ t: "init", url: `${SKIN_BASE}/wmz/${encodeURIComponent(selected)}` }, "*");
  }

  onMount(() => {
    void skins.refresh().then(() => {
      if (!selected && skins.list.length) selected = skins.list[0]!.id;
    });
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  });

  function pick(id: string) {
    if (id === selected) return;
    selected = id;
    preview = null;
    failed = false;
  }
</script>

<DialogFrame title={t.skins.title} onclose={close} width={560}>
  <div class="picker">
    <div class="list" role="listbox" aria-label={t.skins.title}>
      {#each skins.list as s (s.id)}
        <button role="option" aria-selected={s.id === selected} class:sel={s.id === selected} onclick={() => pick(s.id)} ondblclick={apply} title={s.id}>
          {s.name}
        </button>
      {:else}
        <p class="empty">{t.skins.empty}</p>
      {/each}
    </div>
    <div class="side">
      <div class="preview" style:width="{BOX.w}px" style:height="{BOX.h}px">
        {#if selected}
          {#key selected}
            <iframe
              bind:this={frame}
              title={t.skins.title}
              src={`${SKIN_BASE}/frame/index.html`}
              sandbox={isTauri ? "allow-scripts" : "allow-scripts allow-same-origin"}
              onload={load}
              style:width="{preview?.w ?? BOX.w}px"
              style:height="{preview?.h ?? BOX.h}px"
              style:transform="scale({scale})"
              class:ready={!!preview}
            ></iframe>
          {/key}
          {#if failed}<p class="msg">{t.skins.noPreview}</p>{:else if !preview}<p class="msg">{t.skins.loading}</p>{/if}
        {/if}
      </div>
      <p class="name">{preview?.title || skins.list.find((s) => s.id === selected)?.name || ""}</p>
    </div>
  </div>
  <div class="buttons">
    <span class="count">{t.skins.count(skins.list.length)}</span>
    <button class="xp-btn" onclick={() => void commands.skinsOpenDir()}>{t.skins.openFolder}</button>
    <button class="xp-btn" onclick={() => void skins.refresh()}>{t.skins.reload}</button>
    <button class="xp-btn" disabled={!selected || failed} onclick={apply}>{t.skins.apply}</button>
  </div>
</DialogFrame>

<style>
  .picker {
    display: flex;
    gap: 10px;
    height: 280px;
  }
  .list {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    border: 1px solid var(--input-border);
    background: var(--view-bg);
  }
  .list button {
    display: block;
    width: 100%;
    padding: 2px 6px;
    border: 0;
    background: none;
    font: inherit;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: default;
  }
  .list button.sel {
    background: var(--selection);
    color: #fff;
  }
  .empty {
    margin: 8px;
    color: var(--text-muted);
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .preview {
    position: relative;
    overflow: hidden;
    border: 1px solid var(--input-border);
    background: repeating-conic-gradient(#e4e4e4 0 25%, #fff 0 50%) 0 0 / 12px 12px;
  }
  .preview iframe {
    position: absolute;
    left: 0;
    top: 0;
    border: 0;
    transform-origin: 0 0;
    pointer-events: none;
    visibility: hidden;
    color-scheme: light;
  }
  .preview iframe.ready {
    visibility: visible;
  }
  .msg {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--text-muted);
  }
  .name {
    margin: 0;
    font-weight: bold;
    text-align: center;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .count {
    flex: 1;
    color: var(--text-muted);
  }
</style>
