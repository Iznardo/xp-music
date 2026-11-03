<script lang="ts">
  // Tab that sticks out at the top right (WMP 9 skin): current list selector and
  // the window buttons, small and offset as in the original.
  import { commands } from "../../bindings";
  import { appWindow } from "../../platform";
  import { player } from "../../stores/player.svelte";
  import { library } from "../../stores/library.svelte";
  import { ui, type MenuItem } from "../../stores/ui.svelte";
  import { router } from "../../router.svelte";
  import { addToNewPlaylist, playPlaylist } from "../../actions";
  import { t } from "../../i18n.svelte";

  let { maximized = false }: { maximized?: boolean } = $props();
  const win = appWindow();
  const q = $derived(player.queue);
  const count = $derived(q?.tracks.length ?? 0);
  // If the queue came from one of your lists, its name is shown (as in WMP).
  const playingList = $derived.by(() => {
    const src = q?.source;
    return src && typeof src === "object" && src.Playlist ? src.Playlist : null;
  });
  const label = $derived(library.playlists.find((p) => p.id === playingList)?.title ?? t.views.queue);

  function open(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const lists: MenuItem[] = library.playlists.map((p) => ({
      kind: "check",
      label: p.title,
      checked: p.id === playingList,
      action: () => void playPlaylist(p.id),
    }));
    const items: MenuItem[] = [
      { kind: "item", label: t.tab.showQueue, bold: true, action: () => router.go({ name: "now" }) },
      { kind: "sep" },
      ...(lists.length ? lists : [{ kind: "item", label: t.tab.noLists, disabled: true, action: () => {} } as MenuItem]),
      { kind: "sep" },
      { kind: "check", label: t.menu.shuffleAll, checked: q?.shuffle ?? false, action: () => void commands.playerSetShuffle(!q?.shuffle) },
      { kind: "check", label: t.views.autoplay, checked: q?.autoplay ?? true, action: () => void commands.queueSetAutoplay(!(q?.autoplay ?? true)) },
      { kind: "item", label: t.views.clearQueue, disabled: !count, action: () => void commands.queueClear() },
      { kind: "sep" },
      { kind: "item", label: t.menu.newPlaylist, action: () => void addToNewPlaylist([]) },
      { kind: "item", label: t.tab.library, action: () => router.go({ name: "library", section: "playlists" }) },
    ];
    // Make the menu line up with the dropdown, like an XP combo box.
    ui.openMenu({ x: r.left, y: r.bottom + 1 }, items);
  }
</script>

<div class="tab" data-tauri-drag-region>
  <button class="combo" onclick={open} title={label} aria-haspopup="menu">
    <svg class="doc" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 1.5h7l3 3v10H3z" fill="#fff" stroke="var(--btn-border)" />
      <path d="M5 6h6M5 8.5h6M5 11h4" stroke="var(--btn-border)" />
    </svg>
    <span class="label">{label}</span>
    <span class="count">{count}</span>
    <span class="arrow"><svg viewBox="0 0 8 5"><path d="M0 0h8L4 5z" /></svg></span>
  </button>
  <div class="win">
    <button title={t.window.minimize} aria-label={t.window.minimize} onclick={() => win?.minimize()}>
      <svg viewBox="0 0 9 9"><path d="M1 7h7" stroke="currentColor" stroke-width="1.6" /></svg>
    </button>
    <button title={maximized ? t.window.restore : t.window.maximize} aria-label={maximized ? t.window.restore : t.window.maximize} onclick={() => win?.toggleMaximize()}>
      <svg viewBox="0 0 9 9"><rect x="1" y="1.5" width="7" height="6" fill="none" stroke="currentColor" stroke-width="1.3" /></svg>
    </button>
    <button class="close" title={t.window.close} aria-label={t.window.close} onclick={() => win?.close()}>
      <svg viewBox="0 0 9 9"><path d="M1.5 1.5l6 6M7.5 1.5l-6 6" stroke="currentColor" stroke-width="1.6" /></svg>
    </button>
  </div>
</div>

<style>
  .tab {
    height: 100%;
    display: flex;
    /* One row, centered in the tab: combo and window buttons share a midline. */
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 10px;
  }
  .combo {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 23px;
    padding: 0 0 0 4px;
    border: 1px solid var(--btn-soft-border);
    border-radius: 2px;
    background: var(--view-bg);
    color: var(--text);
    font: 12px var(--font-ui);
    text-align: left;
    box-shadow: inset 1px 1px 2px rgb(0 0 40 / 0.2);
    cursor: pointer;
  }
  .doc {
    width: 15px;
    height: 15px;
    flex: none;
  }
  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    color: var(--text-faint);
    font-size: 10px;
  }
  .arrow {
    display: grid;
    place-items: center;
    width: 19px;
    height: 100%;
    border-left: 1px solid var(--btn-soft-border);
    background: linear-gradient(to bottom, var(--btn-1), var(--btn-3));
  }
  .arrow svg {
    width: 8px;
    height: 5px;
    fill: var(--btn-ink);
  }
  .win {
    display: flex;
    gap: 1px;
  }
  .win button {
    display: grid;
    place-items: center;
    width: 15px;
    height: 13px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--shell-ink);
    cursor: pointer;
  }
  /* Over the dark frame the thin glyphs need full white and a cut-out shadow to read. */
  :global([data-mode="dark"]) .win button {
    color: #fff;
    filter: drop-shadow(0 1px 0 rgb(0 0 0 / 0.9));
  }
  .win button:hover {
    color: #fff;
    background: var(--skin-mid);
    border-radius: 2px;
  }
  .win .close:hover {
    background: #d75c32;
  }
  .win svg {
    width: 9px;
    height: 9px;
  }
</style>
