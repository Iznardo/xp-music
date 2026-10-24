// The user's playlists (for the tree and the "Add to playlist" submenu).

import { commands, events, type PlaylistSummary } from "../bindings";
import { shared } from "../resource.svelte";

class LibraryStore {
  playlists = $state<PlaylistSummary[]>([]);
  loaded = $state(false);

  async init(): Promise<void> {
    await events.ytCacheUpdated.listen((e) => {
      if (e.payload.key.startsWith("library:playlists")) void this.refresh();
    });
  }

  async refresh(): Promise<void> {
    // Several parts request it at once on startup: a single request for all of them.
    const r = await shared("library:playlists", () => commands.ytLibraryPlaylists());
    if (r.status === "ok") {
      this.playlists = r.data;
      this.loaded = true;
    }
  }

  /// Own playlists that songs can be added to (not "Liked" nor automatic ones).
  get editable(): PlaylistSummary[] {
    return this.playlists.filter((p) => p.id !== "VLLM" && p.id !== "VLSE" && !p.id.startsWith("VLRD"));
  }
}

export const library = new LibraryStore();
