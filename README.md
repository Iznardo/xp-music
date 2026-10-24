# XP Music

Reproductor de escritorio para **YouTube Music** con el aspecto de Windows Media Player 9 en Windows XP. Para Windows 10/11 (x64 y ARM64), con soporte experimental para Linux (x64 y ARM64) y macOS (Intel y Apple Silicon).

![Biblioteca de XP Music](docs/biblioteca.jpg)

![Reproduciendo con visualización](docs/reproduciendo.jpg)

- Inicio, búsqueda, biblioteca, «Me gusta», historial y radio de tu cuenta de YouTube Music.
- Listas de reproducción: crear, añadir canciones y reproducirlas.
- Visualizaciones que siguen el ritmo de la música.
- Modo compacto y **máscaras de Windows Media Player** (`.wmz`).
- Teclas multimedia, icono en la bandeja, controles en la miniatura de la barra de tareas y actualizaciones automáticas.

> XP Music no está afiliado, patrocinado ni respaldado por Google, YouTube ni Microsoft. «YouTube Music», «Windows Media Player» y «Windows XP» son marcas de sus respectivos dueños. La app usa tu propia cuenta para lo que tú harías desde el navegador.

## Instalar

Descarga el archivo de tu sistema desde [Releases](https://github.com/Iznardo/xp-music/releases).
Arriba de las notas de cada release hay una tabla con todos los archivos, su
SHA-256 y el enlace directo de descarga (`sha256sum archivo` para comprobarlo).

| Sistema | Arquitectura | Archivo (`x.y.z` = versión) | Estado |
| --- | --- | --- | --- |
| Windows 10/11 | x64 | `XP Music_x.y.z_x64-setup.exe` | Soportado; firmado con el certificado propio del autor |
| Windows 10/11 | ARM64 | `XP Music_x.y.z_arm64-setup.exe` | Experimental; firmado con el certificado propio del autor |
| Linux | x64 | `…_amd64.deb`, `…_amd64.AppImage`, `…-1.x86_64.rpm` | Experimental |
| Linux | ARM64 | `…_arm64.deb`, `…_aarch64.AppImage`, `…-1.aarch64.rpm` | Experimental |
| macOS | Intel (x64) | `…_x64.dmg` | Experimental; sin actualizaciones automáticas |
| macOS | Apple Silicon (ARM64) | `…_aarch64.dmg` | Experimental; sin actualizaciones automáticas |
| Arch Linux | x86_64, aarch64 | `packaging/arch/PKGBUILD` (se compila desde el código) | Experimental |
| NixOS / Nix | x86_64, aarch64 | flake del repositorio | Experimental |

«Experimental» significa que el CI compila y prueba el arranque, pero no hay
pruebas manuales en cada sistema.

### Windows

1. Descarga el instalador (`XP Music_x.y.z_x64-setup.exe`, o `_arm64-setup.exe` en equipos ARM).
2. Ábrelo. Como no lleva una firma comercial, Windows SmartScreen puede avisar la primera vez: «Más información» → «Ejecutar de todas formas».
3. Inicia sesión con tu cuenta de Google desde la propia app.

Los dos instaladores se firman con un certificado propio del autor (autofirmado: SmartScreen
no lo reconoce) y llevan la firma del updater, que es lo que comprueba la app antes de
actualizarse.
En Windows la app se actualiza sola: cuando hay una versión nueva, te lo ofrece al arrancar.

### Linux (experimental)

Descarga el paquete de tu arquitectura (x64 o ARM64) desde [Releases](https://github.com/Iznardo/xp-music/releases).
Se compilan en Ubuntu 24.04 (glibc 2.39, WebKitGTK 4.1), y el CI los prueba en Ubuntu
y, el `.rpm`, en Fedora; no son binarios universales para cualquier distribución.

```bash
sudo apt install ./XP*.deb          # Debian, Ubuntu y derivadas
sudo dnf install ./XP*.rpm          # Fedora (openSUSE: sudo zypper install ./XP*.rpm)
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
En gestores de ventanas con teselado (Hyprland, sway, i3…) conviene que XP Music flote,
sobre todo para el modo compacto: una ventana teselada ocupa todo su hueco y el reproductor
compacto queda en una esquina. Una aplicación no puede pedir flotar en Wayland, así que
hay que indicarlo en el gestor (clase/`app_id`: `xp-music`):

```ini
# Hyprland ≥ 0.53
windowrule = float on, match:class ^xp-music$
# Hyprland anterior
windowrulev2 = float, class:^(xp-music)$
# sway
for_window [app_id="xp-music"] floating enable
```

Linux usa MPRIS para los controles multimedia; no incluye los botones de miniatura,
la pausa al bloquear la sesión ni el clic a través de las zonas transparentes de las máscaras.

### Arch Linux

`packaging/arch/PKGBUILD` compila la versión publicada desde el código fuente
(x86_64 y aarch64). Usa el `yt-dlp` y el `deno` de los repositorios oficiales, y deja las
actualizaciones a pacman (la app no se autoactualiza). No está publicado en el AUR.

```bash
git clone https://github.com/Iznardo/xp-music
cd xp-music/packaging/arch
makepkg -si
```

### macOS (experimental)

Requiere macOS 11 o posterior.

1. Descarga el `.dmg` de tu Mac (`_x64.dmg` para Intel, `_aarch64.dmg` para Apple Silicon), ábrelo y arrastra **XP Music** a *Aplicaciones*.
2. La primera vez, macOS lo bloqueará: la app lleva una firma *ad hoc*, sin certificado de desarrollador de Apple ni notarización (salvo que el mantenedor configure la firma de Apple en el CI, ver «Publicar»). Para abrirla:
   - macOS 15 (Sequoia) y posteriores: intenta abrirla, y luego **Ajustes del Sistema → Privacidad y seguridad → «Abrir igualmente»**.
   - macOS 14 y anteriores: clic derecho sobre la app → **Abrir** → **Abrir**.
   - O desde la terminal: `xattr -dr com.apple.quarantine "/Applications/XP Music.app"`.
3. Inicia sesión con tu cuenta de Google desde la propia app.

El CI abre el `.dmg`, comprueba la firma del bundle y arranca la app en macOS 15
(Intel y Apple Silicon). Faltan las funciones propias de Windows (botones de miniatura,
pausa al bloquear la sesión, clic a través de las máscaras).

**No se actualiza sola:** para cambiar de versión, descarga el `.dmg` nuevo y sustituye la
app en *Aplicaciones*. Con firma *ad hoc*, cada versión tiene una firma distinta y macOS
pedirá una vez permiso para que la app lea su sesión del Llavero («Permitir siempre»).
Las actualizaciones automáticas se activarán cuando la app se firme con Developer ID.

### NixOS / Nix

El repositorio incluye un flake (`x86_64-linux` y `aarch64-linux`; este último solo se
comprueba en el CI, sin pruebas en hardware):

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

### Actualizaciones automáticas

Las releases firmadas ofrecen actualizaciones desde la propia app:
- **AppImage:** reemplaza el archivo original; debe estar en una carpeta donde puedas escribir.
- **`.deb` y `.rpm`:** instalan la nueva versión solicitando autorización de administrador.

No se activa en macOS (experimental, ver arriba), binarios de desarrollo, Nix (`/nix/store`), Flatpak ni Snap.
Para paquetes mantenidos por un repositorio (como el de Arch), el empaquetador puede
desactivarlo con `XP_MUSIC_DISABLE_UPDATER=1` y dejar las actualizaciones al gestor de paquetes.
Las compilaciones locales sin clave pública tampoco se autoactualizan.

## Máscaras de Windows Media Player

XP Music lee las máscaras originales de WMP (`.wmz`). No incluye ninguna: copia las que tengas en la carpeta de máscaras (menú **Ver → Selector de máscaras… → Abrir carpeta**, que es `%APPDATA%\xp-music\skins` en Windows y `~/.config/xp-music/skins` en Linux (o `$XDG_CONFIG_HOME/xp-music/skins`) y `~/Library/Application Support/xp-music/skins` en macOS) y pulsa **Actualizar**.

Para volver al aspecto normal: **Ctrl+1**, clic derecho sobre la máscara → «Volver al modo completo», o desde el icono de la bandeja.

Los scripts de cada máscara se ejecutan aislados, sin acceso a tu cuenta ni a tus archivos. Las ventanas secundarias (ecualizador, lista aparte) todavía no se abren.

## Compilar

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
- `npm run check`: tests de los scripts de release, TypeScript, formato, Clippy y tests de Rust.
- `npm run release`: prepara los instaladores Windows x64 y ARM64 firmados y lanza la publicación multiplataforma (ver más abajo).
- `npm run tauri build`: genera los paquetes de tu sistema en `src-tauri/target/release/bundle/`
  (Windows: NSIS; Linux: `.deb`, AppImage y `.rpm`; macOS: `.app` y `.dmg`).

El workflow único `.github/workflows/release.yml` compila, comprueba y empaqueta
**los seis targets** (Windows x64/ARM64, Linux x64/ARM64, macOS Intel/Apple Silicon) en
cada PR y en cada push a `main`, y comprueba el flake de Nix en x86_64 y aarch64. Cada
target ejecuta la acción local `.github/actions/build-target` (dependencias, `npm run check`,
empaquetado, smoke test y subida de artefactos). Los smoke tests arrancan el paquete real:

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

## Publicar una versión (mantenedor)

### Quién compila qué

| Target | Quién lo compila | Firmas |
| --- | --- | --- |
| Windows x64 y ARM64 | **El equipo Windows del mantenedor** (`npm run release`; ARM64 con compilación cruzada) | Authenticode con el certificado local + updater |
| Linux x64 / ARM64 | CI (`ubuntu-24.04`, `ubuntu-24.04-arm`): `.deb`, AppImage, `.rpm` | Updater |
| macOS Intel / Apple Silicon | CI (`macos-15-intel`, `macos-15`): `.dmg` | Sin updater; *ad hoc* o Developer ID + notarización si hay secretos `APPLE_*` |

Se conserva el certificado Authenticode de Windows: `setup-signing.ps1` lo crea
**no exportable**, así que los instaladores de Windows (x64 y ARM64) se firman en el equipo
Windows del mantenedor y el CI los reutiliza byte a byte desde el draft (en el runner
`windows-11-arm` solo se instala y se prueba el de ARM64). Así la clave del updater tampoco
sale de ese equipo para Windows.

Para compilar ARM64 en un equipo x64 hacen falta, una vez, `rustup target add
aarch64-pc-windows-msvc` y el componente «MSVC ARM64 build tools» de Visual Studio Build
Tools. `npm run release` descarga el yt-dlp y el deno de ARM64 (`tools.lock.json`) y los
pone en `tools/` solo mientras compila ese instalador.

Configura una vez en **Iznardo/xp-music → Settings → Secrets and variables → Actions**:

| Tipo | Nombre | Valor |
| --- | --- | --- |
| Variable | `XP_MUSIC_UPDATER_PUBKEY` | Contenido de la clave pública existente `~/.xp-music/updater.key.pub` |
| Variable | `WINDOWS_SIGNING_THUMBPRINT` | Huella de `src-tauri/signing.local.json` |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | Contenido de la clave privada **existente** `~/.xp-music/updater.key` (no una ruta) |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Su contraseña; vacío si no tiene |
| Secret (opcional) | `APPLE_CERTIFICATE` | Certificado «Developer ID Application» exportado como `.p12`, en base64 |
| Secret (opcional) | `APPLE_CERTIFICATE_PASSWORD` | Contraseña del `.p12` |
| Secret (opcional) | `APPLE_SIGNING_IDENTITY` | Nombre de la identidad, p. ej. `Developer ID Application: Nombre (TEAMID)` |
| Secret (opcional) | `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Para notarizar: Apple ID, contraseña específica de app y Team ID |

No generes otra clave del updater: las versiones ya instaladas confían en la actual.
Los secretos solo se pasan al compilar releases, nunca en los checks de PR, ni en push
ni en el dry run. Sin `APPLE_CERTIFICATE` y `APPLE_SIGNING_IDENTITY`, macOS se firma
*ad hoc* (los usuarios deben saltarse Gatekeeper, ver arriba); con ellos se firma con
Developer ID, y se notariza si además están `APPLE_ID`, `APPLE_PASSWORD` y `APPLE_TEAM_ID`.

### Lanzar la release

Desde `main` en el equipo Windows del mantenedor, con `origin` apuntando al upstream:

```powershell
npm run release                     # incrementa patch
npm run release -- -Bump minor
npm run release -- -Bump none -NoPublish  # prueba local, no publica ni lanza CI
npm run release -- -Notes "Qué cambia en esta versión"
```

El script compila y verifica los instaladores Windows x64 y ARM64, sube el tag y prepara una
**release draft** con los EXE, sus firmas del updater y las notas (`-Notes`). Después lanza
el workflow único:

1. Valida el tag, la versión, que sea un draft y que el commit esté en `main`, antes de
   usar ninguna clave. Solo se publica desde `Iznardo/xp-music`.
2. Compila, comprueba y prueba el arranque de los seis targets. En Windows no recompila:
   verifica la firma Authenticode de cada EXE del draft contra la huella configurada y
   lo prueba tal cual. Linux se compila con la misma clave pública y genera sus firmas
   del updater; macOS se compila sin updater.
3. Verifica criptográficamente con `minisign` **todas** las firmas del updater, sube los
   archivos al draft (salvo los EXE de Windows, que ya estaban) y calcula el SHA-256 de cada uno.
4. Escribe en las notas de la release, **encima** de las notas del mantenedor, una tabla
   *Sistema (SO y arquitectura) · SHA-256 · Descarga* con los instaladores; el texto del
   enlace es el nombre del archivo, que ya indica el formato. Los archivos del actualizador
   (`.sig`, `latest.json`) siguen en la release pero no se listan. La tabla va
   entre marcadores HTML, así que reintentar el workflow la sustituye en vez de duplicarla.
5. Crea un único `latest.json` (con solo las notas del mantenedor) con una entrada por
   target y formato: `windows-x86_64`, `windows-aarch64`, `linux-{x86_64,aarch64}-{deb,appimage,rpm}`
   (macOS no tiene entrada: es experimental y no se autoactualiza); las de Windows conservan
   también la clave sin formato (`windows-x86_64`…) que usan las instalaciones existentes.
6. Publica el draft solo cuando todo ha pasado.

Si falla, el draft no se publica. Corrige el problema y reintenta **el mismo tag**
desde Actions → Desktop → Run workflow, o:

```bash
gh workflow run release.yml --repo Iznardo/xp-music --ref main -f tag=v1.0.3
```

**Dry run:** lanzar el workflow *sin* tag (Actions → Run workflow con el campo vacío, o
`gh workflow run release.yml --ref mi-rama`) compila y prueba los seis targets sin
claves ni publicación; sirve para probar cambios de CI en cualquier rama.

No publiques el draft manualmente ni ejecutes otro proceso que escriba
`latest.json`: el workflow es el único publicador. Rechaza tags ajenos a `main`,
versiones inconsistentes, releases ya publicadas y la publicación desde forks.
Para activar el updater en una compilación Linux anterior sin clave pública,
instala primero uno de los nuevos paquetes firmados.

### Arch Linux

El `PKGBUILD` de `packaging/arch/` no forma parte de la publicación. Tras cada release,
sube `pkgver` (con `pkgrel=1`), ejecuta `updpkgsums` en esa carpeta y confirma el cambio.

## Terceros

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) y [Deno](https://deno.com) (MIT) van incluidos en los instaladores y paquetes de las releases para obtener el audio.
- [ytmapi-rs](https://github.com/nick42d/youtui) (MIT), con un parche, en `vendor/` (ver `vendor/README.md`).
- [Tauri](https://tauri.app), [Svelte](https://svelte.dev) y el resto de dependencias de `Cargo.toml` y `package.json`, cada una con su licencia.

## Licencia

[MIT](LICENSE) © 2026 iznardo.
