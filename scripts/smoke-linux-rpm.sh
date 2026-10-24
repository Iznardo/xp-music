#!/usr/bin/env bash
# Instala un .rpm en un contenedor Fedora y le pasa smoke-linux.sh (necesita docker).
#   bash scripts/smoke-linux-rpm.sh RUTA/AL/paquete.rpm
set -euo pipefail
rpm=$(realpath "$1")
scripts=$(realpath "$(dirname "$0")")
# IPC_LOCK: gnome-keyring-daemon de Fedora aborta sin esa capability dentro de docker.
docker run --rm --cap-add IPC_LOCK \
  -v "$rpm:/pkg/xp-music.rpm:ro" -v "$scripts:/scripts:ro" fedora:latest bash -euo pipefail -c '
    # Paquetes de escritorio que el smoke necesita (Xvfb, WM, teclado virtual, Secret Service, audio).
    dnf install -y -q --setopt=install_weak_deps=False \
      xorg-x11-server-Xvfb xorg-x11-xauth openbox xdotool gnome-keyring dbus-x11 dbus-daemon \
      pulseaudio alsa-plugins-pulseaudio procps-ng findutils
    # Las dependencias del rpm las resuelve dnf; webkit2gtk y appindicator (dlopen de la bandeja)
    # se piden aquí por si el paquete no las declara.
    dnf install -y -q --setopt=install_weak_deps=False /pkg/xp-music.rpm \
      webkit2gtk4.1 libayatana-appindicator-gtk3
    test -x /usr/bin/xp-music
    # El Xvfb de Fedora usa 640x480x8 por defecto y la ventana (960x640) no cabría.
    xvfb-run -a -s "-screen 0 1280x1024x24" dbus-run-session -- bash /scripts/smoke-linux.sh /usr/bin/xp-music
  '
