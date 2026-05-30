{
  description = "togl — toggle code comments across languages; CLI + ABI-stable C library (libtogl)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs: rec {
          togl = pkgs.callPackage ./nix/togl.nix { src = self; };
          libtogl = pkgs.callPackage ./nix/libtogl.nix { src = self; };
          default = togl;
        }
      );
    };
}
