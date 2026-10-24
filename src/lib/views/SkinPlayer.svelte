<script lang="ts">
  // Skin mode: a WMP .wmz skin in an isolated iframe (src/skin-frame/). The
  // frame can't see the app; it asks for things via postMessage and here we
  // decide what gets done.
  import { onMount } from "svelte";
  import { commands, events, type RepeatMode } from "../bindings";
  import { player } from "../stores/player.svelte";
  import { skins, skinPrefs, saveSkinPref, SKIN_BASE } from "../stores/skins.svelte";
  import { ui, type MenuItem } from "../stores/ui.svelte";
  import { appWindow, isTauri } from "../platform";
  import { artistNames } from "../actions";
  import { thumbSrc } from "../thumb";
  import { handleShortcut } from "../menus";
  import { vizVisible } from "../viz/active";
  import { t } from "../i18n.svelte";

  let { id }: { id: string } = $props();

  let frame: HTMLIFrameElement;
  let size = $state<{ w: number; h: number } | null>(null);
  let logs = 0;

  /** What the frame needs to know about the player. */
  function skinState() {
    const s = player.state;
    const tr = s?.track ?? null;
    return {
      status: s?.status ?? "Idle",
      title: tr?.title ?? "",
      artist: tr ? artistNames(tr) : "",
      album: tr?.album?.name ?? "",
      durationMs: s?.durationMs ?? tr?.durationMs ?? 0,
      positionMs: s?.positionMs ?? 0,
      volume: s?.volume ?? 0.8,
      muted: !!s?.muted,
      shuffle: !!s?.shuffle,
      repeat: s?.repeat ?? "Off",
      cover: thumbSrc(tr?.thumbnails, 300),
      hasTrack: !!tr,
    };
  }
  function skinQueue() {
    const q = player.queue;
    return {
      items: (q?.tracks ?? []).map((x) => ({ title: x.title, artist: artistNames(x), album: x.album?.name ?? "", durationMs: x.durationMs })),
      current: q?.currentIndex ?? null,
      name: "",
    };
  }
  const post = (msg: unknown) => frame?.contentWindow?.postMessage(msg, "*");

  $effect(() => {
    const s = skinState();
    if (size) post({ t: "state", s });
  });
  $effect(() => {
    const q = skinQueue();
    if (size) post({ t: "queue", q });
  });

  /** What the frame requests, validated: nothing coming from it is ever executed. */
  function command(c: unknown, v: unknown) {
    const num = (x: unknown) => (typeof x === "number" && Number.isFinite(x) ? x : null);
    switch (c) {
      case "play":
        return void commands.playerPlay();
      case "pause":
        return void commands.playerPause();
      case "stop":
        return void commands.playerStop();
      case "next":
        return void commands.playerNext();
      case "previous":
        return void commands.playerPrevious();
      case "seek": {
        const ms = num(v);
        if (ms !== null) void commands.playerSeek(Math.max(0, Math.round(ms)));
        return;
      }
      case "volume": {
        const x = num(v);
        if (x !== null) void commands.playerSetVolume(Math.min(1, Math.max(0, x)));
        return;
      }
      case "mute":
        return void commands.playerSetMuted(!!v);
      case "shuffle":
        return void commands.playerSetShuffle(!!v);
      case "repeat":
        if (v === "Off" || v === "All" || v === "One") void commands.playerSetRepeat(v as RepeatMode);
        return;
      case "playIndex": {
        const i = num(v);
        if (i !== null && i >= 0) void commands.queuePlayIndex(Math.floor(i));
        return;
      }
    }
  }

  async function menu(x: number, y: number) {
    if (isTauri) {
      try {
        return await nativeMenu();
      } catch (e) {
        void commands.uiLog("error", `skin: native menu failed: ${String(e)}`);
      }
    }
    const items: MenuItem[] = [
      { kind: "item", label: t.skins.backToFull, bold: true, shortcut: "Ctrl+1", action: () => void skins.exit() },
      { kind: "item", label: t.skins.chooser, action: () => void openPicker() },
      { kind: "sep" },
      { kind: "item", label: t.menuBar.playPause, action: () => void commands.playerToggle() },
      { kind: "item", label: t.menuBar.next, action: () => void commands.playerNext() },
    ];
    ui.openMenu({ x, y }, items);
  }

  async function nativeMenu() {
    const st = player.state;
    const { Menu, MenuItem: Item, PredefinedMenuItem } = await import("@tauri-apps/api/menu");
    const sep = () => PredefinedMenuItem.new({ item: "Separator" });
    const m = await Menu.new({
      items: [
        await Item.new({ text: t.skins.backToFull, accelerator: "Ctrl+1", action: () => void skins.exit() }),
        await Item.new({ text: t.skins.chooser, action: () => void openPicker() }),
        await sep(),
        await Item.new({ text: t.menuBar.playPause, action: () => void commands.playerToggle() }),
        await Item.new({ text: t.menuBar.stop, action: () => void commands.playerStop() }),
        await Item.new({ text: t.menuBar.previous, action: () => void commands.playerPrevious() }),
        await Item.new({ text: t.menuBar.next, action: () => void commands.playerNext() }),
        await Item.new({ text: t.menuBar.mute, action: () => void commands.playerSetMuted(!st?.muted) }),
        await sep(),
        await Item.new({ text: t.skins.exit, action: () => void appWindow()?.close() }),
      ],
    });
    await m.popup();
  }

  /// The picker doesn't fit in a skin's window: go back to full mode.
  async function openPicker() {
    await skins.exit();
    skins.pickerOpen = true;
  }

  /** Frame rectangles (CSS px) → polygons in physical pixels. */
  function shape(rects: unknown) {
    if (!Array.isArray(rects) || rects.length > 5000) return void commands.windowSetShape(null);
    const d = devicePixelRatio || 1;
    const polys: [number, number][][] = [];
    for (const r of rects) {
      if (!Array.isArray(r) || r.length !== 4 || !r.every((n) => typeof n === "number")) continue;
      const [x, y, w, h] = r.map((n: number) => n * d) as [number, number, number, number];
      polys.push([
        [Math.round(x), Math.round(y)],
        [Math.round(x + w), Math.round(y)],
        [Math.round(x + w), Math.round(y + h)],
        [Math.round(x), Math.round(y + h)],
      ]);
    }
    void commands.windowSetShape(polys.length ? polys : null);
  }

  let vizOn = false;
  let unlistenViz: Promise<() => void> | null = null;

  async function onMessage(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow) return;
    const m = e.data as Record<string, unknown> | null;
    if (!m || typeof m !== "object") return;
    switch (m.t) {
      case "ready": {
        const w = Math.round(Number(m.w));
        const h = Math.round(Number(m.h));
        if (!(w > 0 && h > 0)) {
          ui.error(t.skins.failed(t.skins.noPreview));
          return void skins.exit();
        }
        size = { w, h };
        await commands.windowSetSkin([w, h]);
        shape(m.rects);
        if (m.effects && !vizOn) {
          vizOn = true;
          vizVisible(1);
          unlistenViz = events.vizFrame.listen((ev) => post({ t: "viz", f: ev.payload }));
        }
        return;
      }
      case "shape":
        return shape(m.rects);
      case "cmd":
        return command(m.c, m.v);
      case "win": {
        const win = appWindow();
        if (m.c === "drag") void win?.startDragging();
        else if (m.c === "minimize") void win?.minimize();
        else if (m.c === "close") void win?.close();
        else if (m.c === "full") void skins.exit();
        else if (m.c === "menu") void menu(Number(m.x) || 0, Number(m.y) || 0);
        return;
      }
      case "pref":
        if (typeof m.k === "string" && typeof m.v === "string") saveSkinPref(id, m.k, m.v);
        return;
      case "key": {
        const k = new KeyboardEvent("keydown", { key: String(m.key ?? ""), ctrlKey: !!m.ctrl, altKey: !!m.alt, shiftKey: !!m.shift });
        handleShortcut(k);
        return;
      }
      case "log":
        // The skin's messages go to the log (capped: some skins are very noisy).
        if (logs++ < 30) void commands.uiLog("info", `[skin ${id}] ${String(m.msg ?? "").slice(0, 300)}`);
        return;
      case "error":
        ui.error(t.skins.failed(String(m.msg ?? "")));
        return void skins.exit();
    }
  }

  function init() {
    post({ t: "init", url: `${SKIN_BASE}/wmz/${encodeURIComponent(id)}`, prefs: skinPrefs(id), state: skinState(), queue: skinQueue() });
  }

  onMount(() => {
    window.addEventListener("message", onMessage);
    return () => {
      window.removeEventListener("message", onMessage);
      if (vizOn) vizVisible(-1);
      void unlistenViz?.then((off) => off());
      void commands.windowSetShape(null);
    };
  });
</script>

<!-- The browser preview blocks origin-less frames; in the app it is fully isolated. -->
<iframe
  bind:this={frame}
  title={t.menuBar.skinMode}
  src={`${SKIN_BASE}/frame/index.html`}
  sandbox={isTauri ? "allow-scripts" : "allow-scripts allow-same-origin"}
  class:ready={!!size}
  style:width={size ? `${size.w}px` : "100%"}
  style:height={size ? `${size.h}px` : "100%"}
  onload={init}
></iframe>

<style>
  iframe {
    position: fixed;
    left: 0;
    top: 0;
    border: 0;
    background: transparent;
    color-scheme: light;
    visibility: hidden;
  }
  iframe.ready {
    visibility: visible;
  }
</style>
