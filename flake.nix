{
  description = "XP Music: reproductor de YouTube Music con aspecto de Windows Media Player";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      # aarch64-linux: evaluado, pero sin probar en hardware (solo lo compila el CI).
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        xp-music = pkgs.callPackage ./nix/package.nix { };
        default = xp-music;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.callPackage ./nix/shell.nix {
          xp-music = self.packages.${pkgs.stdenv.hostPlatform.system}.xp-music;
        };
      });

      overlays.default = final: _prev: { xp-music = final.callPackage ./nix/package.nix { }; };

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
