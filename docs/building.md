# Building

Requirements: stable [Rust](https://rustup.rs) and Node.js 24. On Windows you need
WebView2 (included with Windows 11). On Ubuntu 24.04, also install:

```bash
sudo apt install build-essential curl pkg-config unzip libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev \
  libdbus-1-dev libssl-dev
```

```bash
npm install
npm run fetch-tools    # downloads yt-dlp and deno into tools/ (versions pinned in tools.lock.json)
npm run tauri dev      # the app in development mode
```

Other useful commands:

- `npm run dev:mock`: the interface in the browser with simulated data, without the backend.
- `npm run check`: TypeScript checks, formatting, Clippy and Rust tests.
- `npm run release`: prepares the signed Windows installer and starts the release (see [Releasing](releasing.md)).
- `npm run tauri build`: on Linux, generates `.deb` and AppImage in `src-tauri/target/release/bundle/`.
- `npm run site:dev` and `npm run site:build`: the project website (`site/`), which also shows this documentation.

The `.github/workflows/release.yml` workflow checks and packages **Windows and Linux** on every PR.
On Linux it also tests startup, compact mode and shutdown of both packages.
To repeat the smoke test locally (requires Xvfb, D-Bus, Openbox, xdotool, GNOME Keyring and PulseAudio):

```bash
xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh /usr/bin/xp-music
```

## Translations

English is the base language; Spanish is a translation. Keep both in step when you change user-facing text:

- **App interface:** `src/lib/locales/en.ts` is the source of truth (it defines the shape); `es.ts` must match it. The language is chosen in Options, or follows the system language.
- **Backend** (tray menu, taskbar buttons, error messages): each message is an inline English/Spanish pair with `tr!("English", "Español")` (`src-tauri/src/i18n.rs`).
- **Website:** `site/src/locales/en.ts` and `es.ts`.
- **Documentation:** `docs/*.md` in English and `docs/es/*.md` in Spanish, with the same file names. `README.md` and `README.es.md` mirror each other.

## The website

The website lives in `site/` and is published to GitHub Pages with `.github/workflows/pages.yml` on every change to `main` that touches `site/`, `docs/` or the styles and visualizations it reuses from `src/`.
The documentation pages come from the `.md` files in `docs/` (English) and `docs/es/` (Spanish): to add one, create the file in both folders (start with a `# …` title) and link it from the index ([English](README.md), [Spanish](es/README.md)).
Pages meant only for the maintainer (`releasing.md`) are not shown on the website: list them in `MAINTAINER_ONLY` in `site/vite.config.ts` and exclude them in the glob of `site/src/docs.ts`.

The music on the home page is in `site/public/music/` and its list (title, author, duration and source) in `site/src/content.ts`.
It can only be **CC0** (public domain) music: the website publishes it for everyone.
To add a track, re-encode it with the volume evened out:

```bash
ffmpeg -i original.wav -map_metadata -1 -vn -af loudnorm=I=-16:TP=-1.5:LRA=11 \
  -ar 44100 -ac 2 -c:a libmp3lame -b:a 80k site/public/music/name.mp3
```
