// Classic menu bar menus and WMP keyboard shortcuts (FR-004-03, T004-17).

import { commands } from "./bindings";
import { router } from "./router.svelte";
import { player } from "./stores/player.svelte";
import { ui, type MenuItem } from "./stores/ui.svelte";
import { auth } from "./stores/auth.svelte";
import { skins } from "./stores/skins.svelte";
import { appWindow } from "./platform";
import { t } from "./i18n.svelte";

const vol = (d: number) => {
  const v = player.state?.volume ?? 0.8;
  void commands.playerSetVolume(Math.min(1, Math.max(0, Math.round((v + d) * 20) / 20)));
};

export const shortcuts: { keys: string; run: () => void }[] = [
  { keys: "Ctrl+P", run: () => void commands.playerToggle() },
  { keys: "Ctrl+S", run: () => void commands.playerStop() },
  { keys: "Ctrl+B", run: () => void commands.playerPrevious() },
  { keys: "Ctrl+F", run: () => void commands.playerNext() },
  { keys: "Ctrl+H", run: () => void commands.playerSetShuffle(!player.state?.shuffle) },
  { keys: "Ctrl+T", run: () => void commands.playerSetRepeat(player.state?.repeat === "Off" ? "All" : "Off") },
  { keys: "F8", run: () => void commands.playerSetMuted(!player.state?.muted) },
  { keys: "F9", run: () => vol(-0.05) },
  { keys: "F10", run: () => vol(0.05) },
  { keys: "Ctrl+1", run: () => void (skins.active ? skins.exit() : ui.setCompact(false)) },
  { keys: "Ctrl+2", run: () => void ui.setCompact(true) },
  { keys: "Ctrl+E", run: () => document.querySelector<HTMLInputElement>(".taskbar .search input")?.focus() },
];

function keyName(e: KeyboardEvent): string {
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return `${e.ctrlKey ? "Ctrl+" : ""}${e.altKey ? "Alt+" : ""}${e.shiftKey ? "Shift+" : ""}${k}`;
}

/// Global shortcuts. They do nothing while typing in a text field (except F-keys).
export function handleShortcut(e: KeyboardEvent): boolean {
  const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
  const name = keyName(e);
  if (typing && !/^F\d+$/.test(name)) return false;
  // Shift+F10 is the context menu; don't confuse it with F10 (volume up).
  const s = shortcuts.find((x) => x.keys === name);
  if (!s) return false;
  e.preventDefault();
  s.run();
  return true;
}

export function menuBar(): { label: string; items: () => MenuItem[] }[] {
  const st = player.state;
  return [
    {
      label: t.menuBar.file,
      items: () => [
        { kind: "item", label: t.menuBar.signOut, disabled: !auth.signedIn, action: () => void commands.authLogout() },
        { kind: "sep" },
        { kind: "item", label: t.menuBar.exit, action: () => void appWindow()?.close() },
      ],
    },
    {
      label: t.menuBar.view,
      items: () => [
        { kind: "check", label: t.menuBar.nowPlaying, checked: router.tab === "now", action: () => router.go({ name: "now" }) },
        { kind: "check", label: t.menuBar.library, checked: router.tab === "library", action: () => router.go({ name: "library", section: "playlists" }) },
        { kind: "check", label: t.menuBar.home, checked: router.tab === "home", action: () => router.go({ name: "home" }) },
        { kind: "item", label: t.menuBar.search, shortcut: "Ctrl+E", action: () => shortcuts.find((s) => s.keys === "Ctrl+E")?.run() },
        { kind: "sep" },
        { kind: "check", label: t.menuBar.fullMode, checked: !ui.compact && !skins.active, action: () => void ui.setCompact(false) },
        { kind: "check", label: t.menuBar.compactMode, checked: ui.compact, action: () => void ui.setCompact(true) },
        { kind: "check", label: t.menuBar.skinMode, checked: skins.active, action: () => void skins.enter() },
        { kind: "item", label: t.menuBar.skinChooser, action: () => (skins.pickerOpen = true) },
      ],
    },
    {
      label: t.menuBar.play,
      items: () => [
        { kind: "item", label: t.menuBar.playPause, shortcut: "Ctrl+P", action: () => void commands.playerToggle() },
        { kind: "item", label: t.menuBar.stop, shortcut: "Ctrl+S", action: () => void commands.playerStop() },
        { kind: "sep" },
        { kind: "item", label: t.menuBar.previous, shortcut: "Ctrl+B", action: () => void commands.playerPrevious() },
        { kind: "item", label: t.menuBar.next, shortcut: "Ctrl+F", action: () => void commands.playerNext() },
        { kind: "sep" },
        { kind: "check", label: t.menuBar.shuffle, checked: !!st?.shuffle, action: () => void commands.playerSetShuffle(!st?.shuffle) },
        { kind: "check", label: t.menuBar.repeat, checked: st?.repeat === "All", action: () => void commands.playerSetRepeat(st?.repeat === "All" ? "Off" : "All") },
        { kind: "check", label: t.menuBar.repeatOne, checked: st?.repeat === "One", action: () => void commands.playerSetRepeat(st?.repeat === "One" ? "Off" : "One") },
        { kind: "check", label: t.menuBar.autoplay, checked: player.queue?.autoplay ?? true, action: () => void commands.queueSetAutoplay(!(player.queue?.autoplay ?? true)) },
        { kind: "sep" },
        { kind: "item", label: t.menuBar.volumeUp, shortcut: "F10", action: () => vol(0.05) },
        { kind: "item", label: t.menuBar.volumeDown, shortcut: "F9", action: () => vol(-0.05) },
        { kind: "check", label: t.menuBar.mute, checked: !!st?.muted, action: () => void commands.playerSetMuted(!st?.muted) },
      ],
    },
    {
      label: t.menuBar.tools,
      items: () => [{ kind: "item", label: t.menuBar.options, action: () => (ui.optionsOpen = true) }],
    },
    {
      label: t.menuBar.help,
      items: () => [{ kind: "item", label: t.menuBar.about, action: () => { ui.optionsAbout = true; ui.optionsOpen = true; } }],
    },
  ];
}
