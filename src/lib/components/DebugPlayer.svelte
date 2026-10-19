<script lang="ts">
  // Panel provisional para probar el motor de audio (T002-15). Lo sustituye la UI de 004.
  import { onMount } from "svelte";
  import { commands, type AudioDevice, type RepeatMode, type Track } from "../bindings";
  import { player, formatTime } from "../stores/player.svelte";

  let ids = $state("yQd4xQBlBa0, IoNOzOaWgTE, qho-ZjkX8S4");
  let devices = $state<AudioDevice[]>([]);
  let seeking = $state<number | null>(null);

  const st = $derived(player.state);
  const repeatModes: RepeatMode[] = ["Off", "All", "One"];

  onMount(async () => {
    devices = await commands.playerListDevices();
  });

  function toTrack(id: string): Track {
    return {
      id,
      title: id,
      artists: [],
      album: null,
      durationMs: null,
      thumbnails: [],
      explicit: false,
      likeStatus: null,
      setVideoId: null,
    };
  }

  async function playIds() {
    const tracks = ids
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter((s) => /^[\w-]{11}$/.test(s))
      .map(toTrack);
    if (tracks.length) await commands.queueSet(tracks, 0, "Manual");
  }

  function cycleRepeat() {
    const i = repeatModes.indexOf(st?.repeat ?? "Off");
    commands.playerSetRepeat(repeatModes[(i + 1) % repeatModes.length] ?? "Off");
  }
</script>

<fieldset class="debug">
  <legend>Motor de audio (depuración)</legend>

  <div class="row">
    <input class="ids" bind:value={ids} placeholder="videoId, videoId…" />
    <button onclick={playIds}>Reproducir</button>
  </div>

  <div class="row">
    <button onclick={() => commands.playerPrevious()}>⏮</button>
    <button onclick={() => commands.playerToggle()}>
      {st?.status === "Playing" || st?.status === "Buffering" || st?.status === "Loading" ? "⏸" : "▶"}
    </button>
    <button onclick={() => commands.playerStop()}>⏹</button>
    <button onclick={() => commands.playerNext()}>⏭</button>
    <button class:on={st?.shuffle} onclick={() => commands.playerSetShuffle(!st?.shuffle)}>Aleatorio</button>
    <button onclick={cycleRepeat}>Repetir: {st?.repeat ?? "Off"}</button>
    <button class:on={st?.muted} onclick={() => commands.playerSetMuted(!st?.muted)}>Silencio</button>
  </div>

  <div class="row">
    <span class="time">{formatTime(seeking ?? st?.positionMs)}</span>
    <input
      class="seek"
      type="range"
      min="0"
      max={st?.durationMs ?? 0}
      value={seeking ?? st?.positionMs ?? 0}
      oninput={(e) => (seeking = Number(e.currentTarget.value))}
      onchange={(e) => {
        commands.playerSeek(Number(e.currentTarget.value));
        seeking = null;
      }}
    />
    <span class="time">{formatTime(st?.durationMs)}</span>
  </div>

  <div class="row">
    <span>Volumen</span>
    <input
      type="range"
      min="0"
      max="100"
      value={Math.round((st?.volume ?? 0.8) * 100)}
      onchange={(e) => commands.playerSetVolume(Number(e.currentTarget.value) / 100)}
    />
    <select onchange={(e) => commands.playerSetDevice(e.currentTarget.value || null)}>
      <option value="">Dispositivo predeterminado</option>
      {#each devices as d}
        <option value={d.id}>{d.name}{d.isDefault ? " (pred.)" : ""}</option>
      {/each}
    </select>
  </div>

  <p class="status">
    Estado: <b>{st?.status ?? "…"}</b> · Pista: {st?.track?.id ?? "—"} · Búfer: {formatTime(player.bufferedMs)}
    {#if st?.error}<span class="err"> · {st.error.message}</span>{/if}
  </p>

  <ol class="queue">
    {#each player.queue?.tracks ?? [] as t, i}
      <li class:current={i === player.queue?.currentIndex}>
        <button class="link" ondblclick={() => commands.queuePlayIndex(i)}>{t.title}</button>
        <button class="x" title="Quitar" onclick={() => commands.queueRemove(i)}>×</button>
      </li>
    {/each}
  </ol>
</fieldset>

<style>
  .debug {
    width: 100%;
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .ids {
    flex: 1;
  }
  .seek {
    flex: 1;
  }
  .time {
    width: 36px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  button.on {
    font-weight: bold;
  }
  .status {
    margin: 0;
  }
  .err {
    color: #b00;
  }
  .queue {
    margin: 0;
    padding-left: 22px;
    max-height: 140px;
    overflow: auto;
    background: #fff;
    border: 1px solid #7f9db9;
  }
  .queue li.current {
    background: var(--selection);
    color: #fff;
  }
  .link {
    border: 0;
    background: none;
    padding: 1px 0;
    color: inherit;
    cursor: default;
  }
  .x {
    border: 0;
    background: none;
    color: inherit;
  }
</style>
