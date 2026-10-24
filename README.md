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
### NixOS / Nix

El repositorio incluye un flake (solo `x86_64-linux`):

```bash
nix run github:Iznardo/xp-music          # probar
nix profile install github:Iznardo/xp-music
nix develop                              # entorno de desarrollo (cargo, node, WebKitGTK…)
```

En la configuración del sistema, con el overlay `overlays.default` o
`inputs.xp-music.packages.${system}.default`. Se compila desde el código fuente, así que
**no** usa los `.deb`/AppImage de las releases. yt-dlp y Deno vienen de nixpkgs
(`XP_MUSIC_TOOLS`), por lo que ni la app ni sus actualizaciones automáticas los
sustituyen: se actualizan con tu `flake.lock`. Al estar en `/nix/store`, tampoco se
autoactualiza; usa `nix flake update`.
Si un yt-dlp antiguo falla con YouTube, actualiza `nixpkgs` y reconstruye.

Las releases firmadas ofrecen actualizaciones desde la propia app:
- **AppImage:** reemplaza el archivo original; debe estar en una carpeta donde puedas escribir.
- **`.deb`:** instala la nueva versión solicitando autorización de administrador.

No se activa en binarios de desarrollo, Nix (`/nix/store`), Flatpak ni Snap.
Para paquetes mantenidos por un repositorio, el empaquetador puede desactivarlo
con `XP_MUSIC_DISABLE_UPDATER=1` y dejar las actualizaciones al gestor de paquetes.
Las compilaciones locales sin clave pública tampoco se autoactualizan.

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
- `npm run release`: prepara el instalador Windows firmado y lanza la publicación **Windows/Linux** (ver más abajo).
- `npm run tauri build`: en Linux genera `.deb` y AppImage en `src-tauri/target/release/bundle/`.

El workflow único `.github/workflows/release.yml` comprueba y empaqueta **Windows
y Linux** en cada PR. En Linux prueba además el arranque, modo compacto y cierre
de ambos paquetes. Para repetir el smoke test
localmente (requiere Xvfb, D-Bus, Openbox, xdotool, GNOME Keyring y PulseAudio):

```bash
xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh /usr/bin/xp-music
```

## Publicar una versión (mantenedor)

Se conserva el certificado Authenticode de Windows: `setup-signing.ps1` lo crea
**no exportable**, así que el instalador sigue firmándose en el equipo Windows del
mantenedor. No se sustituye por un EXE sin firmar generado en CI.

Configura una vez en **Iznardo/xp-music → Settings → Secrets and variables → Actions**:

| Tipo | Nombre | Valor |
| --- | --- | --- |
| Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contenido de la clave pública existente `~/.xp-music/updater.key.pub` |
| Variable | `WINDOWS_SIGNING_THUMBPRINT` | Huella de `src-tauri/signing.local.json` |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contenido de la clave privada **existente** `~/.xp-music/updater.key` (no una ruta) |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Su contraseña; vacío si no tiene |

No generes otra clave del updater: las versiones ya instaladas confían en la actual.
Los secretos solo se usan al compilar releases, nunca en los checks de PR.

Desde `main` en el equipo Windows del mantenedor, con `origin` apuntando al upstream:

```powershell
npm run release                     # incrementa patch
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # prueba local, no publica ni lanza CI
```

El script compila y verifica el instalador Windows, sube el tag y prepara una
**release draft** con su EXE y firma del updater. Después lanza el workflow único:

1. Comprueba el código en Windows y Linux. Verifica la firma Authenticode del EXE
   original contra la huella configurada, sin recompilarlo ni cambiarlo.
2. Compila `.deb` y AppImage con la misma clave pública y genera sus firmas.
   Prueba ambos paquetes fuera del repositorio.
3. Verifica criptográficamente las tres firmas del updater, adjunta Linux y crea
   un único `latest.json` con Windows y entradas Linux separadas por formato.
4. Publica el draft solo cuando todo ha pasado. La entrada `windows-x86_64` se
   conserva para los clientes Windows existentes.

Si falla, el draft no se publica. Corrige el problema y reintenta **el mismo tag**
desde Actions → Desktop → Run workflow, o:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

No publiques el draft manualmente ni ejecutes otro proceso que escriba
`latest.json`: el workflow es el único publicador. Rechaza tags ajenos a `main`,
versiones inconsistentes, releases ya publicadas y la publicación desde forks.
Para activar el updater en una compilación Linux anterior sin clave pública,
instala primero uno de los nuevos paquetes firmados.

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en el instalador para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.
