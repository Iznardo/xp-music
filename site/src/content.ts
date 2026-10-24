// Website constants that are not translated text (texts are in locales/, documentation in ../../docs).

export const REPO = "https://github.com/Iznardo/xp-music";
/** Always the latest published release (GitHub redirects to its tag). */
export const RELEASES = `${REPO}/releases/latest`;

/** `id` is the route (`#/<id>`); `key` picks the label in `t.sections`. */
export const SECTIONS = [
  { id: "", key: "home" },
  { id: "download", key: "download" },
  { id: "screenshots", key: "screenshots" },
  { id: "docs", key: "docs" },
] as const;

const OGA = "https://opengameart.org/content/";
const FMA = "https://freemusicarchive.org/music/Loyalty_Freak_Music/ROLLER_DISCO_DANCE_DANCE/";

/**
 * Home page music: every track is CC0 (public domain), checked on its source page
 * (`src`). Files in site/public/music/, re-encoded to 80 kbps MP3 with the volume
 * evened out (ffmpeg loudnorm, -16 LUFS). `dur` is in seconds.
 */
export const TRACKS = [
  { file: "dance-robot-activate", title: "Dance Robot ACTIVATE", artist: "Loyalty Freak Music", dur: 159, src: FMA },
  { file: "espelhar", title: "Espelhar", artist: "Fupi", dur: 186, src: `${OGA}espelhar-loud-melodic-edm` },
  { file: "cyberpunk-moonlight-sonata", title: "Cyberpunk Moonlight Sonata", artist: "Joth", dur: 116, src: `${OGA}cyberpunk-moonlight-sonata` },
  { file: "roller-fever", title: "Roller Fever", artist: "Loyalty Freak Music", dur: 145, src: FMA },
  { file: "raspberry-jam", title: "Raspberry Jam", artist: "congusbongus", dur: 174, src: `${OGA}raspberry-jam` },
  { file: "synth-wave", title: "Cool 80's Synth Wave", artist: "Pro Sensory", dur: 192, src: `${OGA}space-synth-wave` },
  { file: "tech-rave", title: "Tech Rave", artist: "Frenchyboy", dur: 129, src: `${OGA}tech-rave` },
  { file: "liquid-flame", title: "Liquid Flame", artist: "Of Far Different Nature", dur: 220, src: `${OGA}liquid-flame` },
  { file: "im-on-fire", title: "I'M ON FIRE", artist: "Loyalty Freak Music", dur: 184, src: FMA },
  { file: "the-bass-of-the-future", title: "The Bass of the Future", artist: "Chance de la Soul", dur: 137, src: `${OGA}the-bass-of-the-future` },
  { file: "hyper-ultra-racing", title: "Hyper Ultra-Racing", artist: "cynicmusic", dur: 79, src: `${OGA}hyper-ultra-racing` },
  { file: "reggae", title: "Reggae", artist: "Pro Sensory", dur: 238, src: `${OGA}reggae` },
  { file: "bouncy-hamster-dancing", title: "Bouncy Hamster Dancing", artist: "cynicmusic", dur: 64, src: `${OGA}bouncy-hamster-dancing-menu-music` },
] as const;

export const fmt = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
