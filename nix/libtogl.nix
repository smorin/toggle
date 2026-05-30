# ABI-stable C library `libtogl` (static + shared) built from the local
# workspace source. Consumed by the repo flake. Single-output for convenience;
# the nixpkgs derivation uses a proper `dev`/`out` split.
{
  lib,
  stdenv,
  rustPlatform,
  src,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
  # ".so" on Linux, ".dylib" on macOS.
  ext = stdenv.hostPlatform.extensions.sharedLibrary;
in
rustPlatform.buildRustPackage {
  pname = "libtogl";
  version = cargoToml.workspace.package.version;
  inherit src;

  cargoLock.lockFile = src + "/Cargo.lock";

  cargoBuildFlags = [ "-p" "togl-ffi" ];

  # The C smoke test shells out to a C compiler and a nested `cargo build`,
  # which is awkward in the build sandbox; the FFI surface is covered by the
  # crate's Rust unit tests in CI.
  doCheck = false;

  # buildRustPackage installs binaries (this crate has none). Install the C
  # artifacts: shared + static library, the generated header, and pkg-config.
  postInstall = ''
    libdir=$(dirname "$(find target -name 'libtogl${ext}' -print -quit)")
    install -Dm755 "$libdir/libtogl${ext}" -t "$out/lib"
    install -Dm644 "$libdir/libtogl.a"     -t "$out/lib"
    install -Dm644 ${src}/crates/togl-ffi/include/togl.h -t "$out/include"

    mkdir -p "$out/lib/pkgconfig"
    substitute ${src}/crates/togl-ffi/togl.pc.in "$out/lib/pkgconfig/togl.pc" \
      --subst-var-by prefix "$out" \
      --subst-var-by version "${cargoToml.workspace.package.version}"
  '';

  meta = {
    description = "ABI-stable C library (libtogl) for toggling code comments";
    homepage = "https://github.com/smorin/toggle";
    license = lib.licenses.mit;
    pkgConfigModules = [ "togl" ];
  };
}
