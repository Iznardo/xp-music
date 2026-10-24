# XP Music

Reproductor de escritorio para **YouTube Music** con el aspecto de Windows Media Player 9 en Windows XP. Para Windows 10/11 y Linux x86_64 (soporte experimental).

![Biblioteca de XP Music](docs/biblioteca.jpg)

![Reproduciendo con visualización](docs/reproduciendo.jpg)

- Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta de YouTube Music.
- Listas de reproducción: crear, añadir canciones y reproducirlas.
- Visualizaciones que siguen el ritmo de la música.
- Modo compacto y **máscaras de Windows Media Player** (`.wmz`).
- Teclas multimedia, icono en la bandeja, controles en la miniatura de la barra de tareas y actualizaciones automáticas.

> XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

## Instalar

### Windows

1. Descarga el instalador (`XP Music_x.y.z_x64-setup.exe`) de la página de *releases*.
2. Ábrelo. Como no lleva una firma comercial, Windows SmartScreen puede avisar la primera vez: «Más información» → «Ejecutar de todas formas».
3. Inicia sesión con tu cuenta de Google desde la propia app.

En Windows la app se actualiza sola: cuando hay una versión nueva, te lo ofrece al arrancar.

### Linux (experimental)

Descarga el `.deb` o la `.AppImage` desde [Releases](https://github.com/Iznardo/xp-music/releases).
Los paquetes se compilan en Ubuntu 24.04 para x86_64; no son binarios universales para cualquier distribución.

```bash
sudo apt install ./XP*.deb
# O, sin instalar (requiere FUSE 2):
chmod +x ./*.AppImage
./XP*.AppImage
# Sin FUSE: ./*.AppImage --appimage-extract-and-run
```

Hace falta una sesión de escritorio con audio y un **Secret Service** desbloqueado
(GNOME Keyring o KeePassXC con esa integración) para guardar la sesión de YouTube.
No se guardan las cookies en texto plano si el almacén no está disponible.
En GNOME, el icono de bandeja necesita una extensión AppIndicator.
Por eso, en Linux cerrar la ventana sale de la app por defecto; activa «Cerrar a
la bandeja» solo si tu escritorio muestra ese icono.
Linux usa MPRIS para los controles multimedia; no incluye los botones de miniatura,
la pausa al bloquear la sesión ni el clic a través de las zonas transparentes de las máscaras.
Los paquetes Linux se actualizan descargando la nueva release, sin autoactualizador.

## Máscaras de Windows Media Player

XP Music lee las máscaras originales de WMP (`.wmz`). No incluye ninguna: copia las que tengas en la carpeta de máscaras (menú **Ver → Selector de máscaras… → Abrir carpeta**, que es `%APPDATA%\xp-music\skins` en Windows y `~/.config/xp-music/skins` en Linux (o `$XDG_CONFIG_HOME/xp-music/skins`)) y pulsa **Actualizar**.

Para volver al aspecto normal: **Ctrl+1**, clic derecho sobre la máscara → «Volver al modo completo», o desde el icono de la bandeja.

Los scripts de cada máscara se ejecutan aislados, sin acceso a tu cuenta ni a tus archivos. Las ventanas secundarias (ecualizador, lista aparte) todavía no se abren.

## Compilar

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
- `npm run release`: compila, firma y publica una versión **Windows** (ver `scripts/release.ps1`).
- `npm run tauri build`: en Linux genera `.deb` y AppImage en `src-tauri/target/release/bundle/`.

El workflow `.github/workflows/linux.yml` comprueba, empaqueta y prueba el arranque,
modo compacto y cierre de ambos paquetes en cada PR. Para repetir el smoke test
localmente (requiere Xvfb, D-Bus, Openbox, xdotool, GNOME Keyring y PulseAudio):

```bash
xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh /usr/bin/xp-music
```

Al publicarse una release adjunta ambos paquetes a **esa misma release**, sin
alterar el instalador Windows ni `latest.json`. También se puede ejecutar manualmente
indicando un tag existente que ya incluya el soporte Linux. No necesita secretos
adicionales: solo el `GITHUB_TOKEN` con permiso de escritura en la fase de publicación.

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en el instalador para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.
