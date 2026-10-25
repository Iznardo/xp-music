# Install

XP Music works on **Windows 10/11** (x64 and ARM64) and, experimentally, on **Linux** (x64 and ARM64) and **macOS** (Intel and Apple Silicon).
All downloads are on the [Releases](https://github.com/Iznardo/xp-music/releases/latest) page.
At the top of each release's notes there is a table with every file, its SHA-256 and the direct download link (`sha256sum file` to check it).

| System | Architecture | File (`x.y.z` = version) | Status |
| --- | --- | --- | --- |
| Windows 10/11 | x64 | `XP Music_x.y.z_x64-setup.exe` | Supported; signed with the author's own certificate |
| Windows 10/11 | ARM64 | `XP Music_x.y.z_arm64-setup.exe` | Experimental; signed with the author's own certificate |
| Linux | x64 | `…_amd64.deb`, `…_amd64.AppImage`, `…-1.x86_64.rpm` | Experimental |
| Linux | ARM64 | `…_arm64.deb`, `…_aarch64.AppImage`, `…-1.aarch64.rpm` | Experimental |
| macOS | Intel (x64) | `…_x64.dmg` | Experimental; no automatic updates |
| macOS | Apple Silicon (ARM64) | `…_aarch64.dmg` | Experimental; no automatic updates |
| Arch Linux | x86_64, aarch64 | `packaging/arch/PKGBUILD` (built from source) | Experimental |
| NixOS / Nix | x86_64, aarch64 | the repository's flake | Experimental |

"Experimental" means CI builds it and tests that it starts, but there is no manual testing on each system.

## Windows

1. Download the installer (`XP Music_x.y.z_x64-setup.exe`, or `_arm64-setup.exe` on ARM PCs).
2. Open it. Since it does not have a commercial signature, Windows SmartScreen may warn you the first time: "More info" → "Run anyway".
3. Sign in with your Google account from the app itself.

Both installers are signed with the author's own certificate (self-signed: SmartScreen does not recognize it) and carry the updater signature, which is what the app checks before updating.
The app updates itself: when there is a new version, it offers it at startup.

## Linux (experimental)

Download the package for your architecture (x64 or ARM64). They are built on Ubuntu 24.04 (glibc 2.39, WebKitGTK 4.1), and CI tests them on Ubuntu and, the `.rpm`, on Fedora; they are not universal binaries for every distribution.

```bash
sudo apt install ./XP*.deb          # Debian, Ubuntu and derivatives
sudo dnf install ./XP*.rpm          # Fedora (openSUSE: sudo zypper install ./XP*.rpm)
# Or, without installing (requires FUSE 2):
chmod +x ./*.AppImage
./XP*.AppImage
# Without FUSE: ./*.AppImage --appimage-extract-and-run
```

You need a desktop session with audio and an unlocked **Secret Service**
(GNOME Keyring, or KeePassXC with that integration) to store the YouTube session.
Cookies are not stored in plain text if the store is not available.

- On GNOME, the tray icon needs an AppIndicator extension. That is why, on Linux, closing the window quits the app by default; turn on "When closing, keep running in the notification area" only if your desktop shows that icon.
- Linux uses MPRIS for the media controls; it does not include the thumbnail buttons, pausing when the session is locked, or clicking through the transparent areas of skins.
- In tiling window managers (Hyprland, sway, i3…) XP Music should float, especially for compact mode: a tiled window fills its whole slot and the compact player ends up in a corner. An application cannot ask to float on Wayland, so it has to be set in the window manager (class/`app_id`: `xp-music`):

```ini
# Hyprland ≥ 0.53
windowrule = float on, match:class ^xp-music$
# Older Hyprland
windowrulev2 = float, class:^(xp-music)$
# sway
for_window [app_id="xp-music"] floating enable
```

### Arch Linux

`packaging/arch/PKGBUILD` builds the released version from source (x86_64 and aarch64). It uses `yt-dlp` and `deno` from the official repositories, and leaves updates to pacman (the app does not update itself). It is not published on the AUR.

```bash
git clone https://github.com/Iznardo/xp-music
cd xp-music/packaging/arch
makepkg -si
```

### NixOS / Nix

The repository includes a flake (`x86_64-linux` and `aarch64-linux`; the latter is only checked in CI, without tests on hardware):

```bash
nix run github:Iznardo/xp-music          # try it
nix profile install github:Iznardo/xp-music
nix develop                              # development environment (cargo, node, WebKitGTK…)
```

In the system configuration, use the `overlays.default` overlay or
`inputs.xp-music.packages.${system}.default`. It is built from source, so it does
**not** use the release `.deb`/AppImage. yt-dlp and Deno come from nixpkgs
(`XP_MUSIC_TOOLS`), so neither the app nor its automatic updates replace them: they are
updated with your `flake.lock`. Since it lives in `/nix/store`, it does not update
itself either; use `nix flake update`.
If an old yt-dlp fails with YouTube, update `nixpkgs` and rebuild.

## macOS (experimental)

Requires macOS 11 or later.

1. Download the `.dmg` for your Mac (`_x64.dmg` for Intel, `_aarch64.dmg` for Apple Silicon), open it and drag **XP Music** to *Applications*.
2. The first time, macOS will block it: the app has an *ad hoc* signature, without an Apple developer certificate or notarization. To open it:
   - macOS 15 (Sequoia) and later: try to open it, then go to **System Settings → Privacy & Security → "Open Anyway"**.
   - macOS 14 and earlier: right-click the app → **Open** → **Open**.
   - Or from the terminal: `xattr -dr com.apple.quarantine "/Applications/XP Music.app"`.
3. Sign in with your Google account from the app itself.

CI opens the `.dmg`, checks the bundle signature and starts the app on macOS 15 (Intel and Apple Silicon). The Windows-specific features are missing (thumbnail buttons, pausing when the session is locked, clicking through skins).

**It does not update itself:** to change versions, download the new `.dmg` and replace the app in *Applications*. With an *ad hoc* signature, each version has a different signature and macOS will ask once for permission for the app to read its session from the Keychain ("Always Allow"). Automatic updates will be enabled once the app is signed with a Developer ID.

## Automatic updates

Signed releases offer updates from the app itself:

- **Windows:** downloads and runs the new installer.
- **AppImage:** replaces the original file; it must be in a folder you can write to.
- **`.deb` and `.rpm`:** install the new version asking for administrator authorization.

It is not enabled on macOS (experimental, see above), development builds, Nix (`/nix/store`), Flatpak or Snap.
For packages maintained by a repository (like Arch's), the packager can
disable it with `XP_MUSIC_DISABLE_UPDATER=1` and leave updates to the package manager.
Local builds without the public key do not update themselves either.

## Connect your account

The first time you open the app, "Connect to YouTube Music" appears. You can:

- **Sign in with Google** in the window the app opens.
- **Import the session from Firefox** (or Zen, LibreWolf, Floorp, Waterfox) if you already have it open there.
- **Paste the YouTube Music cookie** by hand.

The session is stored encrypted (DPAPI on Windows, Secret Service on Linux, the Keychain on macOS). You can sign out in **Options → Account** or in **File → Sign out of YouTube Music**.
