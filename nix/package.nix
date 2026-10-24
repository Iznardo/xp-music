{
  lib,
  rustPlatform,
  cargo-tauri,
  importNpmLock,
  nodejs,
  pkg-config,
  wrapGAppsHook3,
  runCommand,
  cacert,
  openssl,
  webkitgtk_4_1,
  glib-networking,
  alsa-lib,
  dbus,
  libayatana-appindicator,
  xdg-utils,
  yt-dlp,
  deno,
}:

let
  fs = lib.fileset;

  # The app looks for `deno` and `yt-dlp_linux` in XP_MUSIC_TOOLS. On Nix, nixpkgs
  # provides them (same versions as tools.lock.json) instead of downloading them with fetch-tools.
  tools = runCommand "xp-music-tools" { } ''
    mkdir -p $out
    ln -s ${lib.getExe deno} $out/deno
    ln -s ${lib.getExe yt-dlp} $out/yt-dlp_linux
  '';
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "xp-music";
  version = (builtins.fromJSON (builtins.readFile ../package.json)).version;

  src = fs.toSource {
    root = ../.;
    fileset = fs.unions [
      ../src
      # Without target/ when building outside a flake from an already used tree.
      (fs.difference ../src-tauri (fs.maybeMissing ../src-tauri/target))
      ../vendor
      ../index.html
      ../package.json
      ../package-lock.json
      ../svelte.config.js
      ../tsconfig.json
      ../vite.config.ts
      ../LICENSE
      ../THIRD-PARTY-NOTICES.txt
    ];
  };

  cargoLock.lockFile = ../src-tauri/Cargo.lock;
  cargoRoot = "src-tauri";
  buildAndTestSubdir = finalAttrs.cargoRoot;

  # No hash to maintain: resolved from package-lock.json.
  npmDeps = importNpmLock.buildNodeModules {
    npmRoot = finalAttrs.src;
    inherit nodejs;
  };

  nativeBuildInputs = [
    cargo-tauri.hook
    importNpmLock.linkNodeModulesHook
    nodejs
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = [
    openssl
    webkitgtk_4_1
    glib-networking
    alsa-lib
    dbus
    libayatana-appindicator
  ];

  # The tests build reqwest clients, which need system certificates.
  nativeCheckInputs = [ cacert ];
  preCheck = ''
    export SSL_CERT_FILE=${cacert}/etc/ssl/certs/ca-bundle.crt
  '';

  # tauri.linux.conf.json bundles tools/ as a resource. Here nixpkgs provides them, so
  # empty placeholders are used for packaging and removed on install.
  preBuild = ''
    mkdir -p tools/yt-dlp
    touch tools/deno
  '';

  postInstall = ''
    rm -rf "$out"/lib/*/tools
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      --set XP_MUSIC_TOOLS ${tools}
      --prefix PATH : ${lib.makeBinPath [ xdg-utils ]}
      # The tray icon loads libayatana-appindicator with dlopen.
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ libayatana-appindicator ]}
    )
  '';

  passthru = { inherit tools; };

  meta = {
    description = "YouTube Music player with a Windows Media Player / XP look";
    homepage = "https://github.com/Iznardo/xp-music";
    license = lib.licenses.mit;
    mainProgram = "xp-music";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  };
})
