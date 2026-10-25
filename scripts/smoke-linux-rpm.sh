#!/usr/bin/env bash
# Installs an .rpm in a Fedora container and runs smoke-linux.sh on it (needs docker).
#   bash scripts/smoke-linux-rpm.sh PATH/TO/package.rpm
set -euo pipefail
rpm=$(realpath "$1")
scripts=$(realpath "$(dirname "$0")")
# IPC_LOCK: the Fedora gnome-keyring-daemon aborts without that capability inside docker.
docker run --rm --cap-add IPC_LOCK \
  -v "$rpm:/pkg/xp-music.rpm:ro" -v "$scripts:/scripts:ro" fedora:latest bash -euo pipefail -c '
    # Desktop packages the smoke test needs (Xvfb, WM, virtual keyboard, Secret Service, audio).
    dnf install -y -q --setopt=install_weak_deps=False \
      xorg-x11-server-Xvfb xorg-x11-xauth openbox xdotool gnome-keyring dbus-x11 dbus-daemon \
      pulseaudio alsa-plugins-pulseaudio procps-ng findutils
    # dnf resolves the rpm dependencies; webkit2gtk and appindicator (dlopen for the tray)
    # are requested here in case the package does not declare them.
    dnf install -y -q --setopt=install_weak_deps=False /pkg/xp-music.rpm \
      webkit2gtk4.1 libayatana-appindicator-gtk3
    test -x /usr/bin/xp-music
    # The Fedora Xvfb defaults to 640x480x8 and the window (960x640) would not fit.
    xvfb-run -a -s "-screen 0 1280x1024x24" dbus-run-session -- bash /scripts/smoke-linux.sh /usr/bin/xp-music
  '
