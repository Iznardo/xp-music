// Reproductor de la portada: un <audio> con las pistas CC0 de TRACKS, pasado por un
// AnalyserNode para que las visualizaciones sigan la música de verdad.

import { TRACKS } from "./content";
import { Analyzer } from "./analyser";
import type { VizData } from "../../src/lib/viz/types";

export const player = $state({
  track: 0,
  pos: 0,
  playing: false,
  /** Ya se ha pulsado ▶ alguna vez (antes, la visualización va sola). */
  started: false,
  volume: 80,
  muted: false,
  shuffle: false,
});

export const current = () => TRACKS[player.track]!;

const audio = new Audio();
audio.preload = "none";
let graph: { ctx: AudioContext; gain: GainNode; node: AnalyserNode; an: Analyzer; freq: Float32Array<ArrayBuffer>; time: Float32Array<ArrayBuffer> } | null = null;

audio.addEventListener("play", () => (player.playing = true));
audio.addEventListener("pause", () => (player.playing = false));
audio.addEventListener("timeupdate", () => (player.pos = audio.currentTime));
audio.addEventListener("ended", () => next(1));

function load(i: number) {
  player.track = i;
  player.pos = 0;
  // Relativa: la web vive en /xp-music/ en GitHub Pages.
  audio.src = `music/${TRACKS[i]!.file}.mp3`;
  const t = TRACKS[i]!;
  if ("mediaSession" in navigator) navigator.mediaSession.metadata = new MediaMetadata({ title: t.title, artist: t.artist, album: "XP Music" });
}
load(0);

function gain() {
  if (graph) graph.gain.gain.value = player.muted ? 0 : (player.volume / 100) ** 2;
}

/** Llamar desde un clic: el navegador solo deja sonar audio tras un gesto del usuario. */
export function play(i = player.track) {
  if (i !== player.track) load(i);
  if (!graph) {
    const ctx = new AudioContext();
    const node = ctx.createAnalyser();
    node.fftSize = 2048;
    node.smoothingTimeConstant = 0; // El suavizado lo hace Analyzer, como en la app.
    const g = ctx.createGain();
    // El análisis va antes del volumen: la visualización no depende de él.
    ctx.createMediaElementSource(audio).connect(node);
    node.connect(g).connect(ctx.destination);
    graph = { ctx, gain: g, node, an: new Analyzer(ctx.sampleRate, node.fftSize), freq: new Float32Array(node.frequencyBinCount), time: new Float32Array(node.fftSize) };
    gain();
  }
  void graph.ctx.resume();
  player.started = true;
  void audio.play().catch(() => {});
}

export const pause = () => audio.pause();
export const toggle = () => (player.playing ? pause() : play());

export function stop() {
  audio.pause();
  seek(0);
}

export function seek(s: number) {
  audio.currentTime = s;
  player.pos = s;
}

export function next(d: 1 | -1) {
  // Como en WMP: «Anterior» con la canción ya empezada vuelve a su principio.
  if (d === -1 && player.pos > 3) return seek(0);
  const n = TRACKS.length;
  const i = player.shuffle ? (player.track + 1 + Math.floor(Math.random() * (n - 1))) % n : (player.track + d + n) % n;
  if (player.playing || audio.ended) play(i);
  else load(i);
}

export function setVolume(v: number) {
  player.volume = v;
  gain();
}

export function toggleMute() {
  player.muted = !player.muted;
  gain();
}

/** Fotograma de la música que suena, o null si aún no ha sonado nada. */
export function vizFrame(): VizData | null {
  if (!graph) return null;
  graph.node.getFloatFrequencyData(graph.freq);
  graph.node.getFloatTimeDomainData(graph.time);
  return graph.an.analyze(graph.freq, graph.time);
}

if ("mediaSession" in navigator) {
  navigator.mediaSession.setActionHandler("play", () => play());
  navigator.mediaSession.setActionHandler("pause", pause);
  navigator.mediaSession.setActionHandler("nexttrack", () => next(1));
  navigator.mediaSession.setActionHandler("previoustrack", () => next(-1));
}
