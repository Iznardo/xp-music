{
  mkShell,
  lib,
  cargo,
  rustc,
  clippy,
  rustfmt,
  rust-analyzer,
  cargo-tauri,
  nodejs,
  glib-networking,
  gtk3,
  gsettings-desktop-schemas,
  libayatana-appindicator,
  webkitgtk_4_1,
  xdotool,
  xp-music,
}:

mkShell {
  inputsFrom = [ xp-music ];

  packages = [
    cargo
    rustc
    clippy
    rustfmt
    rust-analyzer
    cargo-tauri
    nodejs
    xdotool
  ];

  # yt-dlp y deno de nixpkgs, sin necesidad de `npm run fetch-tools`.
  XP_MUSIC_TOOLS = "${xp-music.passthru.tools}";
  RUST_SRC_PATH = "${rustc.unwrapped}/lib/rustlib/src/rust/library";

  shellHook = ''
    export LD_LIBRARY_PATH=${
      lib.makeLibraryPath [
        libayatana-appindicator
        webkitgtk_4_1
        gtk3
      ]
    }:$LD_LIBRARY_PATH
    export GIO_EXTRA_MODULES=${glib-networking}/lib/gio/modules
    export XDG_DATA_DIRS=${gtk3}/share/gsettings-schemas/${gtk3.name}:${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:$XDG_DATA_DIRS
  '';
}
