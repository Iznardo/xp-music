// Reflejo del reproductor del backend (principio II). Se hidrata al montar
// (la WebView puede recrearse) y se mantiene al día con los eventos.

import { commands, events, type AppError, type PlayerState, type QueueSnapshot } from "../bindings";

class PlayerStore {
  state = $state<PlayerState | null>(null);
  queue = $state<QueueSnapshot | null>(null);
  bufferedMs = $state(0);
  lastError = $state<AppError | null>(null);

  async init(): Promise<void> {
    await Promise.all([
      events.playerState.listen((e) => {
        this.state = e.payload;
      }),
      events.playerPosition.listen((e) => {
        if (this.state) this.state.positionMs = e.payload.positionMs;
        this.bufferedMs = e.payload.bufferedMs;
      }),
      events.queueChanged.listen((e) => {
        this.queue = e.payload;
      }),
      events.playerError.listen((e) => {
        this.lastError = e.payload;
      }),
    ]);
    const [state, queue] = await Promise.all([commands.playerGetState(), commands.queueGet()]);
    this.state = state;
    this.queue = queue;
  }
}

export const player = new PlayerStore();
