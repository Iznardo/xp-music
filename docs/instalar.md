# Instalar

XP Music funciona en **Windows 10/11** y, de forma experimental, en **Linux x86_64**.
Todas las descargas están en la página de [Releases](https://github.com/Iznardo/xp-music/releases).

## Windows

1. Descarga el instalador (`XP Music_x.y.z_x64-setup.exe`).
2. Ábrelo. Como no lleva una firma comercial, Windows SmartScreen puede avisar la primera vez: «Más información» → «Ejecutar de todas formas».
3. Inicia sesión con tu cuenta de Google desde la propia app.

La app se actualiza sola: cuando hay una versión nueva, te lo ofrece al arrancar.

## Linux (experimental)

Descarga el `.deb` o la `.AppImage`. Los paquetes se compilan en Ubuntu 24.04 para x86_64; no son binarios universales para cualquier distribución.

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

### Actualizaciones en Linux

Las releases firmadas ofrecen actualizaciones desde la propia app:

- **AppImage:** reemplaza el archivo original; debe estar en una carpeta donde puedas escribir.
- **`.deb`:** instala la nueva versión solicitando autorización de administrador.

No se activa en binarios de desarrollo, Nix (`/nix/store`), Flatpak ni Snap.
Para paquetes mantenidos por un repositorio, el empaquetador puede desactivarlo
con `XP_MUSIC_DISABLE_UPDATER=1` y dejar las actualizaciones al gestor de paquetes.
Las compilaciones locales sin clave pública tampoco se autoactualizan.

## Conectar tu cuenta

Al abrir la app por primera vez aparece «Conectar con YouTube Music». Puedes:

- **Iniciar sesión con Google** en la ventana que abre la app.
- **Importar la sesión de Firefox** (o Zen, LibreWolf, Floorp, Waterfox) si ya la tienes abierta allí.
- **Pegar la cookie** de YouTube Music a mano.

La sesión se guarda cifrada (DPAPI en Windows, Secret Service en Linux). Puedes cerrarla en **Opciones → Cuenta** o en **Archivo → Cerrar sesión de YouTube Music**.
