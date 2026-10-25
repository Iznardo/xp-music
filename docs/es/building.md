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
- `npm run release`: prepara los instaladores Windows x64 y ARM64 firmados y lanza la publicación multiplataforma (ver [Publicar una versión](releasing.md)).
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
  `.dmg` (macOS) y espera hasta 60 s a las líneas «UI mounted» y «tools found»
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

## Traducciones

El inglés es el idioma base y el español es una traducción. Si cambias un texto visible, actualiza los dos:

- **Interfaz de la app:** `src/lib/locales/en.ts` es la fuente de verdad (define la forma) y `es.ts` debe coincidir. El idioma se elige en Opciones o sigue el del sistema.
- **Backend** (menú de la bandeja, botones de la barra de tareas, mensajes de error): cada mensaje es un par inglés/español en línea con `tr!("English", "Español")` (`src-tauri/src/i18n.rs`).
- **Web:** `site/src/locales/en.ts` y `es.ts`.
- **Documentación:** `docs/*.md` en inglés y `docs/es/*.md` en español, con los mismos nombres de archivo. `README.md` y `README.es.md` son espejo uno del otro.

## La web

La web vive en `site/` y se publica en GitHub Pages con `.github/workflows/pages.yml` en cada cambio de `main` que toque `site/`, `docs/` o los estilos y visualizaciones que reutiliza de `src/`. En los PR el mismo workflow solo la compila, sin desplegar.
Las páginas de documentación salen de los `.md` de `docs/` (inglés) y `docs/es/` (español): para añadir una, crea el archivo en las dos carpetas (empieza por un título `# …`) y enlázalo desde el índice ([inglés](../README.md), [español](README.md)).
Las páginas solo para el mantenedor (`releasing.md`) no salen en la web: añádelas a `MAINTAINER_ONLY` en `site/vite.config.ts` y exclúyelas en el glob de `site/src/docs.ts`.

La música de la portada está en `site/public/music/` y su lista (título, autor, duración y origen) en `site/src/content.ts`.
Solo puede ser música **CC0** (dominio público): la web la publica para cualquiera.
Para añadir una pista, recodifícala con el volumen igualado:

```bash
ffmpeg -i original.wav -map_metadata -1 -vn -af loudnorm=I=-16:TP=-1.5:LRA=11 \
  -ar 44100 -ac 2 -c:a libmp3lame -b:a 80k site/public/music/name.mp3
```
