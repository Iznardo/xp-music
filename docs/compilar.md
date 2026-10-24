# Compilar

Requisitos: [Rust](https://rustup.rs) estable y Node.js 24. En Windows hace falta
WebView2 (viene con Windows 11); en macOS, las herramientas de línea de comandos de Xcode.
En Ubuntu 24.04 instala además:

```bash
sudo apt install build-essential curl pkg-config unzip libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev \
  libdbus-1-dev libssl-dev
```

```bash
npm install
npm run fetch-tools    # descarga yt-dlp y deno de tu sistema en tools/ (versiones fijadas en tools.lock.json)
npm run tauri dev      # la app en modo desarrollo
```

Otras órdenes útiles:

- `npm run dev:mock`: la interfaz en el navegador con datos simulados, sin backend.
- `npm run check`: tests de los scripts de release y de la web, TypeScript, formato, Clippy y tests de Rust.
- `npm run release`: prepara los instaladores Windows x64 y ARM64 firmados y lanza la publicación multiplataforma (ver [Publicar una versión](publicar.md)).
- `npm run tauri build`: genera los paquetes de tu sistema en `src-tauri/target/release/bundle/`
  (Windows: NSIS; Linux: `.deb`, AppImage y `.rpm`; macOS: `.app` y `.dmg`).
- `npm run site:dev` y `npm run site:build`: la web del proyecto (`site/`), que también muestra esta documentación.

## Integración continua

El workflow único `.github/workflows/release.yml` compila, comprueba y empaqueta
**los seis targets** (Windows x64/ARM64, Linux x64/ARM64, macOS Intel/Apple Silicon) en
cada PR y en cada push a `main`, y comprueba el flake de Nix en x86_64 y aarch64. Cada
target ejecuta la acción local `.github/actions/build-target` (dependencias, empaquetado,
smoke test y subida de artefactos). `npm run check` corre en un target por sistema
(Windows x64, Linux x64 y macOS Apple Silicon), porque el código específico de cada SO
no cambia con la arquitectura. Fuera de las releases se usa una caché de compilación de
Rust (`Swatinem/rust-cache`, la guarda solo `main`); al publicar se compila siempre sin
caché. Los smoke tests arrancan el paquete real:

- Linux (`.deb`, AppImage): arranque, recursos, modo compacto y cierre, con
  `scripts/smoke-linux.sh`. El `.rpm` se instala en un contenedor `fedora:latest`
  (`scripts/smoke-linux-rpm.sh`, necesita Docker).
- Windows y macOS: `scripts/smoke-desktop.mjs` lanza la app instalada (Windows) o la del
  `.dmg` (macOS) y espera hasta 60 s a las líneas «UI montada» y «herramientas encontradas»
  del log.

Para repetir el de Linux localmente (requiere Xvfb, D-Bus, Openbox, xdotool,
GNOME Keyring y PulseAudio):

```bash
xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh /usr/bin/xp-music
bash scripts/smoke-linux-rpm.sh "src-tauri/target/release/bundle/rpm/XP Music-x.y.z-1.x86_64.rpm"
```

Los nombres y formatos de cada archivo de una release se definen una sola vez en
`scripts/release-targets.mjs`; de ahí salen `latest.json`, la tabla de descargas y la
recogida de artefactos del CI. Al añadir un target, actualiza esa tabla y la matriz del workflow.

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
