# Compilar

Requisitos: [Rust](https://rustup.rs) estable y Node.js 24. En Windows hace falta
WebView2 (viene con Windows 11). En Ubuntu 24.04 instala además:

```bash
sudo apt install build-essential curl pkg-config unzip libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev \
  libdbus-1-dev libssl-dev
```

```bash
npm install
npm run fetch-tools    # descarga yt-dlp y deno en tools/ (versiones fijadas en tools.lock.json)
npm run tauri dev      # la app en modo desarrollo
```

Otras órdenes útiles:

- `npm run dev:mock`: la interfaz en el navegador con datos simulados, sin backend.
- `npm run check`: comprobaciones de TypeScript, formato, Clippy y tests de Rust.
- `npm run release`: prepara el instalador Windows firmado y lanza la publicación (ver [Publicar una versión](publicar.md)).
- `npm run tauri build`: en Linux genera `.deb` y AppImage en `src-tauri/target/release/bundle/`.
- `npm run site:dev` y `npm run site:build`: la web del proyecto (`site/`), que también muestra esta documentación.

El workflow `.github/workflows/release.yml` comprueba y empaqueta **Windows y Linux** en cada PR.
En Linux prueba además el arranque, el modo compacto y el cierre de ambos paquetes.
Para repetir el smoke test localmente (requiere Xvfb, D-Bus, Openbox, xdotool, GNOME Keyring y PulseAudio):

```bash
xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh /usr/bin/xp-music
```

## La web

La web vive en `site/` y se publica en GitHub Pages con `.github/workflows/pages.yml` en cada cambio de `main` que toque `site/`, `docs/` o los estilos y visualizaciones que reutiliza de `src/`.
Las páginas de documentación salen de los `.md` de `docs/`: para añadir una, crea el archivo (empieza por un título `# …`) y enlázalo desde [el índice](README.md).

La música de la portada está en `site/public/music/` y su lista (título, autor, duración y origen) en `site/src/content.ts`.
Solo puede ser música **CC0** (dominio público): la web la publica para cualquiera.
Para añadir una pista, recodifícala con el volumen igualado:

```bash
ffmpeg -i original.wav -map_metadata -1 -vn -af loudnorm=I=-16:TP=-1.5:LRA=11 \
  -ar 44100 -ac 2 -c:a libmp3lame -b:a 80k site/public/music/nombre.mp3
```
