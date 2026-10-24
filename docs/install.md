# Install

XP Music runs on **Windows 10/11** and, experimentally, on **Linux x86_64**.
All downloads are on the [Releases](https://github.com/Iznardo/xp-music/releases/latest) page.

## Windows

1. Download the installer (`XP Music_x.y.z_x64-setup.exe`).
2. Open it. It does not carry a commercial signature, so Windows SmartScreen may warn you the first time: "More info" → "Run anyway".
3. Sign in with your Google account from the app itself.

The app updates itself: when a new version is available, it offers it at startup.

## Linux (experimental)

Download the `.deb` or the `.AppImage`. The packages are built on Ubuntu 24.04 for x86_64; they are not universal binaries for every distribution.

```bash
sudo apt install ./XP*.deb
# Or, without installing (requires FUSE 2):
chmod +x ./*.AppImage
./XP*.AppImage
# Without FUSE: ./*.AppImage --appimage-extract-and-run
```

You need a desktop session with audio and an unlocked **Secret Service**
(GNOME Keyring, or KeePassXC with that integration) to store the YouTube session.
Cookies are not stored in plain text if the keyring is not available.

- On GNOME, the tray icon needs an AppIndicator extension. That is why, on Linux, closing the window quits the app by default; turn on "When closing, keep running in the notification area" only if your desktop shows that icon.
- Linux uses MPRIS for media controls; it does not include the taskbar thumbnail buttons, pause on lock, or click-through on the transparent areas of skins.
- On tiling window managers (Hyprland, sway, i3…) XP Music should float, especially for compact mode: a tiled window fills its whole slot and the compact player ends up in a corner. An application cannot ask to float on Wayland, so tell your window manager (class/`app_id`: `xp-music`):

```ini
# Hyprland ≥ 0.53
windowrule = float on, match:class ^xp-music$
# Older Hyprland
windowrulev2 = float, class:^(xp-music)$
# sway
for_window [app_id="xp-music"] floating enable
```

### NixOS / Nix

The repository includes a flake (`x86_64-linux` only):

```bash
nix run github:Iznardo/xp-music          # try it
nix profile install github:Iznardo/xp-music
nix develop                              # development environment (cargo, node, WebKitGTK…)
```

In your system configuration, use the `overlays.default` overlay or
`inputs.xp-music.packages.${system}.default`. It is built from source, so it does
**not** use the `.deb`/AppImage from the releases. yt-dlp and Deno come from nixpkgs
(`XP_MUSIC_TOOLS`), so neither the app nor its automatic updates replace
them: they are updated with your `flake.lock`. Being in `/nix/store`, it does not
update itself either; use `nix flake update`.
If an old yt-dlp fails with YouTube, update `nixpkgs` and rebuild.

### Updates on Linux

Signed releases offer updates from the app itself:

- **AppImage:** replaces the original file; it must be in a folder you can write to.
- **`.deb`:** installs the new version, asking for administrator authorization.

Updates are not enabled in development binaries, Nix (`/nix/store`), Flatpak or Snap.
For packages maintained by a repository, the packager can disable them
with `XP_MUSIC_DISABLE_UPDATER=1` and leave updates to the package manager.
Local builds without a public key do not update themselves either.

## Connect your account

The first time you open the app, "Connect to YouTube Music" appears. You can:

- **Sign in with Google** in the window the app opens.
- **Import the Firefox session** (or Zen, LibreWolf, Floorp, Waterfox) if you are already signed in there.
- **Paste the cookie** from YouTube Music by hand.

The session is stored encrypted (DPAPI on Windows, Secret Service on Linux). You can sign out in **Options → Account** or in **File → Sign out of YouTube Music**.
