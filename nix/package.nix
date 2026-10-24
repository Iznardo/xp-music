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

  # La app busca `deno` y `yt-dlp_linux` en XP_MUSIC_TOOLS. En Nix los pone nixpkgs
  # (mismas versiones que tools.lock.json) en lugar de descargarlos con fetch-tools.
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
      # Sin target/ si se compila fuera de un flake desde un árbol ya usado.
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

  # Sin hash que mantener: se resuelve desde package-lock.json.
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

  # Los tests construyen clientes reqwest, que necesitan certificados del sistema.
  nativeCheckInputs = [ cacert ];
  preCheck = ''
    export SSL_CERT_FILE=${cacert}/etc/ssl/certs/ca-bundle.crt
  '';

  # tauri.linux.conf.json empaqueta tools/ como recurso. Aquí lo aportan nixpkgs, así que
  # se usan marcadores vacíos para el empaquetado y se retiran al instalar.
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
      # El icono de bandeja carga libayatana-appindicator con dlopen.
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ libayatana-appindicator ]}
    )
  '';

  passthru = { inherit tools; };

  meta = {
    description = "Reproductor de YouTube Music con estética Windows Media Player / XP";
    homepage = "https://github.com/Iznardo/xp-music";
    license = lib.licenses.mit;
    mainProgram = "xp-music";
    platforms = [ "x86_64-linux" ];
  };
})
