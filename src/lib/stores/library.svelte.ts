// Playlists del usuario (para el árbol y el submenú "Añadir a playlist").

import { commands, events, type PlaylistSummary } from "../bindings";

class LibraryStore {
  playlists = $state<PlaylistSummary[]>([]);
  loaded = $state(false);

  async init(): Promise<void> {
    await events.ytCacheUpdated.listen((e) => {
      if (e.payload.key.startsWith("library:playlists")) void this.refresh();
    });
  }

  async refresh(): Promise<void> {
    const r = await commands.ytLibraryPlaylists();
    if (r.status === "ok") {
      this.playlists = r.data;
      this.loaded = true;
    }
  }

  /// Playlists propias donde se pueden añadir canciones (no "Me gusta" ni automáticas).
  get editable(): PlaylistSummary[] {
    return this.playlists.filter((p) => p.id !== "VLLM" && p.id !== "VLSE" && !p.id.startsWith("VLRD"));
  }
}

export const library = new LibraryStore();
