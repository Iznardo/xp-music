# Instalar

XP Music funciona en **Windows 10/11** (x64 y ARM64) y, de forma experimental, en **Linux** (x64 y ARM64) y **macOS** (Intel y Apple Silicon).
Todas las descargas están en la página de [Releases](https://github.com/Iznardo/xp-music/releases/latest).
Arriba de las notas de cada release hay una tabla con todos los archivos, su SHA-256 y el enlace directo de descarga (`sha256sum archivo` para comprobarlo).

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

«Experimental» significa que el CI compila y prueba el arranque, pero no hay pruebas manuales en cada sistema.

## Windows

1. Descarga el instalador (`XP Music_x.y.z_x64-setup.exe`, o `_arm64-setup.exe` en equipos ARM).
2. Ábrelo. Como no lleva una firma comercial, Windows SmartScreen puede avisar la primera vez: «Más información» → «Ejecutar de todas formas».
3. Inicia sesión con tu cuenta de Google desde la propia app.

Los dos instaladores se firman con un certificado propio del autor (autofirmado: SmartScreen no lo reconoce) y llevan la firma del updater, que es lo que comprueba la app antes de actualizarse.
La app se actualiza sola: cuando hay una versión nueva, te lo ofrece al arrancar.

## Linux (experimental)

Descarga el paquete de tu arquitectura (x64 o ARM64). Se compilan en Ubuntu 24.04 (glibc 2.39, WebKitGTK 4.1), y el CI los prueba en Ubuntu y, el `.rpm`, en Fedora; no son binarios universales para cualquier distribución.

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

- En GNOME, el icono de bandeja necesita una extensión AppIndicator. Por eso, en Linux cerrar la ventana sale de la app por defecto; activa «Cerrar a la bandeja» solo si tu escritorio muestra ese icono.
- Linux usa MPRIS para los controles multimedia; no incluye los botones de miniatura, la pausa al bloquear la sesión ni el clic a través de las zonas transparentes de las máscaras.
- En gestores de ventanas con teselado (Hyprland, sway, i3…) conviene que XP Music flote, sobre todo para el modo compacto: una ventana teselada ocupa todo su hueco y el reproductor compacto queda en una esquina. Una aplicación no puede pedir flotar en Wayland, así que hay que indicarlo en el gestor (clase/`app_id`: `xp-music`):

```ini
# Hyprland ≥ 0.53
windowrule = float on, match:class ^xp-music$
# Hyprland anterior
windowrulev2 = float, class:^(xp-music)$
# sway
for_window [app_id="xp-music"] floating enable
```

### Arch Linux

`packaging/arch/PKGBUILD` compila la versión publicada desde el código fuente (x86_64 y aarch64). Usa el `yt-dlp` y el `deno` de los repositorios oficiales, y deja las actualizaciones a pacman (la app no se autoactualiza). No está publicado en el AUR.

```bash
git clone https://github.com/Iznardo/xp-music
cd xp-music/packaging/arch
makepkg -si
```

### NixOS / Nix

El repositorio incluye un flake (`x86_64-linux` y `aarch64-linux`; este último solo se comprueba en el CI, sin pruebas en hardware):

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

## macOS (experimental)

Requiere macOS 11 o posterior.

1. Descarga el `.dmg` de tu Mac (`_x64.dmg` para Intel, `_aarch64.dmg` para Apple Silicon), ábrelo y arrastra **XP Music** a *Aplicaciones*.
2. La primera vez, macOS lo bloqueará: la app lleva una firma *ad hoc*, sin certificado de desarrollador de Apple ni notarización. Para abrirla:
   - macOS 15 (Sequoia) y posteriores: intenta abrirla, y luego **Ajustes del Sistema → Privacidad y seguridad → «Abrir igualmente»**.
   - macOS 14 y anteriores: clic derecho sobre la app → **Abrir** → **Abrir**.
   - O desde la terminal: `xattr -dr com.apple.quarantine "/Applications/XP Music.app"`.
3. Inicia sesión con tu cuenta de Google desde la propia app.

El CI abre el `.dmg`, comprueba la firma del bundle y arranca la app en macOS 15 (Intel y Apple Silicon). Faltan las funciones propias de Windows (botones de miniatura, pausa al bloquear la sesión, clic a través de las máscaras).

**No se actualiza sola:** para cambiar de versión, descarga el `.dmg` nuevo y sustituye la app en *Aplicaciones*. Con firma *ad hoc*, cada versión tiene una firma distinta y macOS pedirá una vez permiso para que la app lea su sesión del Llavero («Permitir siempre»). Las actualizaciones automáticas se activarán cuando la app se firme con Developer ID.

## Actualizaciones automáticas

Las releases firmadas ofrecen actualizaciones desde la propia app:

- **Windows:** descarga e instala el nuevo instalador.
- **AppImage:** reemplaza el archivo original; debe estar en una carpeta donde puedas escribir.
- **`.deb` y `.rpm`:** instalan la nueva versión solicitando autorización de administrador.

No se activa en macOS (experimental, ver arriba), binarios de desarrollo, Nix (`/nix/store`), Flatpak ni Snap.
Para paquetes mantenidos por un repositorio (como el de Arch), el empaquetador puede
desactivarlo con `XP_MUSIC_DISABLE_UPDATER=1` y dejar las actualizaciones al gestor de paquetes.
Las compilaciones locales sin clave pública tampoco se autoactualizan.

## Conectar tu cuenta

Al abrir la app por primera vez aparece «Conectar con YouTube Music». Puedes:

- **Iniciar sesión con Google** en la ventana que abre la app.
- **Importar la sesión de Firefox** (o Zen, LibreWolf, Floorp, Waterfox) si ya la tienes abierta allí.
- **Pegar la cookie** de YouTube Music a mano.

La sesión se guarda cifrada (DPAPI en Windows, Secret Service en Linux, el Llavero en macOS). Puedes cerrarla en **Opciones → Cuenta** o en **Archivo → Cerrar sesión de YouTube Music**.
