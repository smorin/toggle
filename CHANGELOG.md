# Changelog

All notable changes are listed here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/).

## [0.4.0](https://github.com/smorin/toggle/compare/v0.3.0...v0.4.0) (2026-05-30)


### Features

* **nix:** add flake exposing togl CLI + libtogl C library ([#29](https://github.com/smorin/toggle/issues/29)) ([f9724f5](https://github.com/smorin/toggle/commit/f9724f5034bd75ad6b338d84f770ae73692b8e2e))

## [0.3.0](https://github.com/smorin/toggle/compare/v0.2.3...v0.3.0) (2026-05-30)


### Features

* **cli:** add --insert marker-insertion mode (P05, v0.3.0) ([#23](https://github.com/smorin/toggle/issues/23)) ([0756d68](https://github.com/smorin/toggle/commit/0756d683ff2f3ab28a8720e3cead2013f209248c))
* **cli:** wire --insert mode and --desc flag (P05) ([de6d767](https://github.com/smorin/toggle/commit/de6d7674a3ef07f58133df7c7cf0e3456d2f34a0))
* **core:** add insert_section marker insertion (P05) ([b23a82e](https://github.com/smorin/toggle/commit/b23a82e8d9e94f146a58aa7db6f67161cf00d3b9))
* **ffi:** add togl-ffi C library (libtogl) + togl-* crate rename ([#24](https://github.com/smorin/toggle/issues/24)) ([c9ba98d](https://github.com/smorin/toggle/commit/c9ba98dcf7a46ca3f98ae8db5dfe000628bcbeb0))
* P05 --insert + togl-ffi C library (libtogl) + togl-* rename ([1124c61](https://github.com/smorin/toggle/commit/1124c61615fe31fbeb7a8737000dc95845706047))


### Bug Fixes

* **ci:** self-build libtogl staticlib in C smoke test; allow MPL-2.0 (cbindgen) in deny ([b5b17fc](https://github.com/smorin/toggle/commit/b5b17fcf9b59ffcd491f1ac378c7663eab6da2f1))
* **cli:** honor --eol and reject --pair in --insert mode (P05) ([5e290e5](https://github.com/smorin/toggle/commit/5e290e5452192b27de3a8378436740c6fe2d0469))
* **cli:** reject --insert with --scan/--atomic; strengthen dup-id test (P05) ([9949b5f](https://github.com/smorin/toggle/commit/9949b5ffc39bd6df3587a324e9d08bf5fcbf4c64))
* silence cargo shared-target warning, correct README install line ([cf2da8c](https://github.com/smorin/toggle/commit/cf2da8cb4d9180dd98e5a744c44742461c0bc8a5))


### Refactor

* split into Cargo workspace with toggle-lib and toggle-cli crates ([#22](https://github.com/smorin/toggle/issues/22)) ([1dcce1a](https://github.com/smorin/toggle/commit/1dcce1ac7bc27c8b5a31e9b7443fcc8a5e0097cc))

## [0.2.3] — 2026-04-16

### Added

- **`togl` binary alias** — `cargo install togl` now installs both `toggle` and `togl`. Same behavior under either name; `--help`, `--version`, completions, and the man page each self-identify by the invoked name.

### Fixed

- `--help` and `--version` now exit with code `0` instead of `1`. Pre-existing bug where clap's display-only errors were classified as Usage failures.

## [0.2.2] — 2026-04-16

### Changed

- **Crate renamed `toggle` → `togl`** for crates.io (`toggle` was taken). The installed binary is still `toggle`. Install with `cargo install togl`.

## [0.2.1] — 2026-04-16

### Added

- **`--completions <SHELL>`** — emits a shell completion script to stdout for `bash`, `zsh`, `fish`, `powershell`, or `elvish`. Example: `toggle --completions bash > /etc/bash_completion.d/toggle`.
- **`--man`** — emits a roff-formatted man page to stdout. Example: `toggle --man > toggle.1 && man ./toggle.1`.

### Fixed

- `repository` field in `Cargo.toml` now points at the real GitHub URL (`smorin/toggle`).

### Changed

- Path argument is now optional so `--completions` and `--man` can run without targets.

## [0.2.0] — 2026-04-16

### Added

- **Section variants (`group:variant`)** — IDs with a `:` are now recognized as variants of a shared group. `-S group` flips a 2-variant pair, `-S group:variant` activates one and comments siblings, and `-S group --force on|off` applies the same state to every variant. Errors when an unqualified `-S group` targets a 3+ variant group. (PRD §0.13)
- **`--pair`** — pre-execution guard that errors when the targeted group does not contain exactly 2 variants. No file mutations occur on failure. (PRD §0.13.4)
- **Variant-aware `--scan`** — per-file table now includes a `TYPE` column (`solo` / `pair` / `group`). `--scan -R` aggregates per group across files; `--scan -S <group>` shows the detailed variant-by-variant view with file refs. (PRD §0.14.1–§0.14.2)
- **`--scan --json`** — emits a nested `{ sections: [...] }` tree with variants grouped under their parent. (PRD §0.14.4)
- **`--scan --check`** — read-only validation: reports unclosed `toggle:start` markers, duplicate IDs in a single file, cross-file variant gaps, and (with `--pair`) pair-count mismatches. Exits non-zero on any error finding; warnings do not fail the run. (PRD §0.14.3)

### Changed

- `--scan -S <id>` is no longer an error — it is now the detailed view mode.
- `--scan --json` output shape changed from a flat `[...]` array to a nested `{ sections: [...] }` tree. Tooling that parsed the old shape needs to read `.sections[]`.

### Removed

- The unrunnable task-master npm scaffolding (`package.json`, `scripts/dev.js`, etc.) which closed 17 Dependabot alerts in unused npm transitives.

## [0.1.0]

Initial release: line-range and section-based toggling with auto-detected
comment styles, `--scan` discovery mode, atomic single-file writes, and
multi-file atomic mode with write-ahead journal recovery.

[0.2.3]: https://github.com/smorin/toggle/releases/tag/v0.2.3
[0.2.2]: https://github.com/smorin/toggle/releases/tag/v0.2.2
[0.2.1]: https://github.com/smorin/toggle/releases/tag/v0.2.1
[0.2.0]: https://github.com/smorin/toggle/releases/tag/v0.2.0
