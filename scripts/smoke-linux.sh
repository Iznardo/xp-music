#!/usr/bin/env bash
# Run with xvfb-run -a dbus-run-session -- bash scripts/smoke-linux.sh BINARY
set -euo pipefail
binary=$(realpath "$1")
work=$(mktemp -d)
export HOME="$work/home" XDG_RUNTIME_DIR="$work/runtime"
export XDG_CONFIG_HOME="$HOME/.config" XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
unset XP_MUSIC_TOOLS
app_pid=
cleanup() {
  if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; fi
  cat "$work/app.log" 2>/dev/null || true
  find "$XDG_DATA_HOME" -name '*.log' -exec cat {} \; 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
openbox >"$work/wm.log" 2>&1 &
printf '%s' smoke-test-password | gnome-keyring-daemon --unlock --components=secrets
pulseaudio --start --exit-idle-time=-1
printf 'pcm.!default { type pulse }\nctl.!default { type pulse }\n' >"$HOME/.asoundrc"
# Only for root containers; the app configuration is not modified.
if [[ $(id -u) -eq 0 ]]; then
  export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
fi
export WEBKIT_DISABLE_DMABUF_RENDERER=1
cd "$work"
"$binary" >"$work/app.log" 2>&1 &
app_pid=$!
window=
for _ in {1..60}; do
  kill -0 "$app_pid"
  window=$(xdotool search --onlyvisible --name '^XP Music$' 2>/dev/null | head -1) || true
  if [[ -n "$window" ]] && grep -Rq 'UI mounted' "$XDG_DATA_HOME"; then break; fi
  sleep 1
done
test -n "$window"
grep -Rq 'UI mounted' "$XDG_DATA_HOME"
grep -Rq 'tools found' "$XDG_DATA_HOME"
xdotool windowactivate --sync "$window"
xdotool key --clearmodifiers Escape ctrl+2
wait_size() {
  for _ in {1..50}; do
    if xdotool getwindowgeometry --shell "$window" | grep -q "^WIDTH=$1$" &&
       xdotool getwindowgeometry --shell "$window" | grep -q "^HEIGHT=$2$"; then return; fi
    kill -0 "$app_pid"
    sleep 0.1
  done
  xdotool getwindowgeometry "$window"
  return 1
}
wait_size 460 150
xdotool key --clearmodifiers ctrl+1
wait_size 960 640
# On Linux, closing must quit by default, even without a visible tray icon.
xdotool key --clearmodifiers alt+F4
for _ in {1..50}; do
  if ! kill -0 "$app_pid" 2>/dev/null; then
    wait "$app_pid"
    app_pid=
    echo 'Linux smoke OK: startup, resources, compact mode, restore and close.'
    exit 0
  fi
  sleep 0.1
done
echo 'The application did not exit when the window was closed.' >&2
exit 1
