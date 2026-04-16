# Changelog

All notable changes are listed here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/).

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

[0.2.0]: https://github.com/smorin/toggle/releases/tag/v0.2.0
