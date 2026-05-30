# nixpkgs derivation for togl.
# Destination when contributing upstream: pkgs/by-name/to/togl/package.nix
# Fill the two `lib.fakeHash` values: run `nix build .#togl` twice, pasting the
# real hash reported on each failure (source hash first, then cargoHash).
{
  lib,
  rustPlatform,
  fetchFromGitHub,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "togl";
  version = "0.2.3";

  src = fetchFromGitHub {
    owner = "smorin";
    repo = "toggle";
    tag = "v${finalAttrs.version}";
    hash = lib.fakeHash;
  };

  cargoHash = lib.fakeHash;

  # Workspace root is virtual; build/test only the CLI package.
  cargoBuildFlags = [ "-p" "togl" ];
  cargoTestFlags = [ "-p" "togl" ];

  # Lets the r-ryantm bot auto-open version-bump PRs; also drives `nix-update`.
  passthru.updateScript = nix-update-script { };

  meta = {
    description = "CLI tool for toggling code comments across multiple languages";
    homepage = "https://github.com/smorin/toggle";
    changelog = "https://github.com/smorin/toggle/blob/v${finalAttrs.version}/CHANGELOG.md";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ ]; # add your handle once in maintainer-list.nix
    mainProgram = "togl";
  };
})
