# CLI package (binaries `toggle` and `togl`), built from the local workspace
# source. Consumed by the repo flake (`flake.nix`). The nixpkgs derivation —
# which fetches a tagged tarball and uses `cargoHash` — lives separately in the
# nixpkgs tree at `pkgs/by-name/to/togl/package.nix`.
{
  lib,
  rustPlatform,
  src,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
in
rustPlatform.buildRustPackage {
  pname = "togl";
  version = cargoToml.workspace.package.version;
  inherit src;

  # Build from the committed lockfile — no vendored-deps hash to maintain.
  cargoLock.lockFile = src + "/Cargo.lock";

  # Virtual workspace: build and test only the CLI crate.
  cargoBuildFlags = [ "-p" "togl" ];
  cargoTestFlags = [ "-p" "togl" ];

  meta = {
    description = "CLI tool for toggling code comments across multiple languages";
    homepage = "https://github.com/smorin/toggle";
    license = lib.licenses.mit;
    mainProgram = "togl";
  };
}
