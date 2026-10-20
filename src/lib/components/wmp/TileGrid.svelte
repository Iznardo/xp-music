<script lang="ts">
  // Iconos grandes estilo explorador de XP (FR-004-07): portada + título + subtítulo.
  import type { MediaItem } from "../../bindings";
  import { hintTrack, itemSubtitle, itemThumbs, itemTitle, openItem, playTracks, trackMenu } from "../../actions";
  import { ui, type MenuItem } from "../../stores/ui.svelte";
  import { thumbSrc } from "../../thumb";
  import { t } from "../../strings";

  let { items, row = false }: { items: MediaItem[]; row?: boolean } = $props();

  function menu(item: MediaItem, e: MouseEvent) {
    if (item.Track) return ui.openMenu(e, trackMenu([item.Track], { onPlay: () => openItem(item) }));
    const items: MenuItem[] = [{ kind: "item", label: t.menu.open, bold: true, action: () => openItem(item) }];
    ui.openMenu(e, items);
  }
  void playTracks;
</script>

<div class="grid" class:row>
  {#each items as item, i (i)}
    {@const src = thumbSrc(itemThumbs(item), 120)}
    <button class="tile" class:round={!!item.Artist} onmouseenter={() => item.Track && hintTrack(item.Track.id)} ondblclick={() => openItem(item)} onkeydown={(e) => e.key === "Enter" && openItem(item)} oncontextmenu={(e) => menu(item, e)} title={itemTitle(item)}>
      <span class="art">
        {#if src}<img {src} alt="" loading="lazy" />{/if}
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
  .art img {
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
