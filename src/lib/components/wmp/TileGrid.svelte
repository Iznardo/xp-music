<script lang="ts">
  // Large XP Explorer-style icons (FR-004-07): cover + title + subtitle.
  import type { MediaItem } from "../../bindings";
  import { hintTrack, itemSubtitle, itemThumbs, itemTitle, openItem, playTracks, trackMenu } from "../../actions";
  import { ui, type MenuItem } from "../../stores/ui.svelte";
  import { thumbSrc } from "../../thumb";
  import { t } from "../../i18n.svelte";

  let {
    items,
    row = false,
    onopen,
  }: {
    items: MediaItem[];
    row?: boolean;
    /// Instead of opening the item on YouTube (local albums and artists).
    onopen?: (item: MediaItem, index: number) => void;
  } = $props();

  const open = (item: MediaItem, i: number) => (onopen ? onopen(item, i) : openItem(item));

  function menu(item: MediaItem, i: number, e: MouseEvent) {
    if (item.Track) return ui.openMenu(e, trackMenu([item.Track], { onPlay: () => open(item, i) }));
    const items: MenuItem[] = [{ kind: "item", label: t.menu.open, bold: true, action: () => open(item, i) }];
    ui.openMenu(e, items);
  }
  void playTracks;
</script>

<div class="grid" class:row>
  {#each items as item, i (i)}
    {@const src = thumbSrc(itemThumbs(item), 120)}
    <button class="tile" class:round={!!item.Artist} onmouseenter={() => item.Track && hintTrack(item.Track.id)} ondblclick={() => open(item, i)} onkeydown={(e) => e.key === "Enter" && open(item, i)} oncontextmenu={(e) => menu(item, i, e)} title={itemTitle(item)}>
      <span class="art">
        <span class="ph">♫</span>
        <!-- Some automatic lists come without a valid cover: the icon stays. -->
        {#if src}<img {src} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")} />{/if}
        {#if item.Track}<span class="badge">♪</span>{/if}
      </span>
      <span class="title">{itemTitle(item)}</span>
      <span class="sub">{itemSubtitle(item)}</span>
    </button>
  {/each}
</div>

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px;
  }
  .grid.row {
    flex-wrap: nowrap;
    overflow-x: auto;
    padding-bottom: 6px;
  }
  .tile {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 128px;
    padding: 4px;
    border: 1px solid transparent;
    border-radius: 2px;
    background: none;
    font: inherit;
    color: inherit;
    cursor: default;
  }
  .tile:hover {
    border-color: #c4d3ee;
    background: #eef3fc;
  }
  .tile:focus {
    outline: none;
    border-color: #316ac5;
    background: #dde7f8;
  }
  .art {
    position: relative;
    width: 104px;
    height: 104px;
    border: 1px solid #a6b2c8;
    background: linear-gradient(135deg, #e7ecf5, #c4cee2);
    box-shadow: 2px 2px 3px rgb(0 0 0 / 0.18);
  }
  .round .art {
    border-radius: 50%;
    overflow: hidden;
  }
  .ph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 40px;
    color: rgb(255 255 255 / 0.85);
    text-shadow: 0 1px 3px rgb(0 0 40 / 0.4);
  }
  .art img {
    position: relative;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .badge {
    position: absolute;
    right: 3px;
    bottom: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: radial-gradient(circle at 40% 30%, #8ec3ff, #1f5fd1);
    color: #fff;
    font-size: 11px;
    line-height: 18px;
  }
  .title,
  .sub {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }
  .title {
    margin-top: 5px;
  }
  .sub {
    color: #6d6d6d;
  }
</style>
