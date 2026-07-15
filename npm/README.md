# npm packaging for togl

esbuild-style layout: the unscoped wrapper [`togl-cli`](togl-cli/) exposes the
`togl` and `toggle` commands via Node shims and declares the four platform
packages under [`platform/`](platform/) as `optionalDependencies`
(`@smorinlabs/togl-{linux-x64,darwin-x64,darwin-arm64,win32-x64}`). npm
installs only the package matching the host's `os`/`cpu`; the shim
`require.resolve`s the binary out of it.

Nothing here is published from a dev machine. The `publish-npm` job in
[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs on the
`v*` tag: it downloads the release tarballs, unpacks the binaries into each
platform package's `bin/`, stamps every `version` (and the wrapper's
`optionalDependencies` ranges) to the release version, then publishes the four
platform packages followed by the wrapper — all via npm trusted publishing
(OIDC, no stored token). The committed `0.0.0` versions and empty `bin/`
directories are placeholders by design.

Linux x64 ships the musl (statically linked) build so it works on both glibc
and musl distros.
