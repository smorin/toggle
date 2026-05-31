# PROJECTS

Tracks implementation work for `toggle`. Each project corresponds to one PR.
The PRD source of truth lives in `prds/phase-0-prd.md` (notably §0.13 and §0.14).

**Status Legend:**
- `[x]` Completed
- `[-]` In Progress
- `[ ]` Not Started
- `[~]` Won't fix / Invalid / False positive

---

> **Release plan:** P01–P04 landed together as a single `v0.2.0` release on `main`.

## [x] Project P01: Section Variants Core (v0.2.0)
**Goal**: Implement `group:variant` section markers and the four CLI variant
behaviors from PRD §0.13.3 — solo, pair-flip, activate, force-all — with
per-file scope. Errors when a group has 3+ variants and `-S` lacks a
`:variant` qualifier.

**Out of Scope**
- The `--pair` validation flag (P02)
- Cross-file consistency checks (P04)
- Scan output changes (P03)

### Tests & Tasks
- [x] [P01-T01] Add `parse_id_parts(id) -> (group, Option<variant>)` helper to `src/core.rs`
- [x] [P01-TS01] Unit tests in `tests/unit/core_tests.rs` covering solo, pair, edge cases
- [x] [P01-T02] Add `discover_variants(content, group) -> Vec<SectionInfo>` helper to `src/core.rs`
- [x] [P01-TS02] Unit tests for `discover_variants` (no matches, 1, 2, 3 variants, mixed solo+variant)
- [x] [P01-T03] Add `toggle_variant_group(...)` and `activate_variant(...)` functions to `src/core.rs`
- [x] [P01-TS03] Unit tests covering pair-flip, activate, force-on-all, force-off-all, error on 3+
- [x] [P01-T04] Route variant CLI arguments in `src/main.rs::toggle_section` to the new core functions
- [x] [P01-TS04] Integration tests in `tests/integration.rs` exercising each CLI form against a fixture file
- [x] [P01-T05] Add `tests/fixtures/variants.py` with a pair group and a 3-variant group
- [x] [P01-T06] Update CLI help text on `-S` to mention variant syntax
- [x] [P01-T07] Run `just dev` (format, lint, test, build) and ensure clean
- [x] [P01-T08] Run `just dev`, commit (no version bump yet)

### Deliverable
```bash
# Pair flip — db has exactly 2 variants
$ toggle -S db tests/fixtures/variants.py
# Activate one variant
$ toggle -S db:postgres tests/fixtures/variants.py
# Force all variants of a group commented
$ toggle -S db --force on tests/fixtures/variants.py
# Error when group has 3+ variants and no qualifier
$ toggle -S cache tests/fixtures/variants.py
Error: group 'cache' has 3 variants; specify one with -S cache:<name>
```

### Automated Verification
- `cargo test` passes including new unit + integration tests
- `just lint` clean (clippy with `-D warnings`)
- `just build` succeeds

### Manual Verification
- `toggle -S db variants.py` swaps active/commented blocks
- `toggle -S db:postgres variants.py` activates postgres, comments others

### Detailed Implementation Plan
See `docs/superpowers/plans/2026-04-16-p01-section-variants-core.md`

---

## [x] Project P02: `--pair` Validation Flag (v0.2.0)
**Goal**: Add the `--pair` pre-execution guard from PRD §0.13.4 that errors
when the targeted group does not contain exactly 2 variants. No file
modifications occur on failure.

### Tests & Tasks
- [x] [P02-T01] Add `--pair` flag to `src/cli.rs`
- [x] [P02-TS01] Unit test that `--pair` requires `-S <group>` (no line ranges)
- [x] [P02-T02] Insert pre-execution validation in `src/main.rs` before file mutations
- [x] [P02-TS02] Integration tests: success on 2-variant group; error on 1, 3, 4
- [x] [P02-T03] Update README / help text
- [x] [P02-T04] Run `just dev`

---

## [x] Project P03: Variant-Aware Scan Output (v0.2.0)
**Goal**: Extend `--scan` per PRD §0.14.1–§0.14.2 and §0.14.4: group variants
under their parent group, infer the `TYPE` column (solo/pair/group), add
recursive summary mode, the `--scan -S <id>` detailed view, and JSON output
that nests variants under groups.

### Tests & Tasks
- [x] [P03-T01] Extend `ScanSectionInfo` (or add `ScanGroupInfo`) in `src/core.rs` with group/type/variant fields
- [x] [P03-TS01] Unit tests for type inference (solo / pair / group / mixed)
- [x] [P03-T02] Update `print_scan_results` and `run_scan` in `src/main.rs`
- [x] [P03-TS02] Integration: per-file table, recursive summary, `--scan -S db`
- [x] [P03-T03] Update JSON schema per §0.14.4
- [x] [P03-TS03] Test for JSON output
- [x] [P03-T04] Run `just dev`

---

## [x] Project P04: `--check` Validation Mode (v0.2.0)
**Goal**: Add `--scan --check` per PRD §0.14.3 — read-only validation that
reports unclosed markers, pair groups with ≠2 variants, cross-file variant
mismatches, and duplicate IDs within a single file.

### Tests & Tasks
- [x] [P04-T01] Add `--check` flag in `src/cli.rs` (only valid with `--scan`)
- [x] [P04-T02] Add validation logic in `src/core.rs` returning a list of `CheckIssue { level, group, message }`
- [x] [P04-TS01] Unit tests for each check (unclosed, pair-mismatch, file-mismatch, duplicate ID)
- [x] [P04-T03] Wire `--check` output (table + JSON) into `src/main.rs`
- [x] [P04-TS02] Integration tests
- [x] [P04-T04] Behavior with `--check --pair` (only checks pair-inferred groups)
- [x] [P04-T05] Run `just dev`

---

## [x] Project P05: `--insert` Marker Insertion (v0.3.0)
**Goal**: Add a `--insert` mode that wraps a single file's `-l start:end` block
in a `toggle:start`/`toggle:end` marker pair, leaving the body uncommented.
See `docs/superpowers/specs/2026-05-29-marker-insert-strip-list-filters-design.md`.

### Tests & Tasks
- [x] [P05-T01] `core::insert_section` happy path + unit tests
- [x] [P05-T02] `insert_section` guards (dup ID, bounds, bad id/desc) + unit tests
- [x] [P05-T03] `--insert` / `--desc` CLI flags, validation, `run_insert` + integration tests
- [x] [P05-T04] `--to-end`, `--comment-style`, `--dry-run` integration tests
- [x] [P05-T05] README + PROJECTS.md + `just dev`

---

> **Note:** The marker roadmap (P05–P07) is complete. The C ABI library is
> numbered P08; release automation P09.

## [x] Project P06: `--remove` (recursive strip, three modes) (v0.5.0)
**Goal**: Add `--remove -S <ID>` to strip a named section, recursively with `-R`.
`--remove-mode markers|commented|all` (default `commented`): `markers` deletes
only the marker lines; `commented` also deletes fully-commented body lines but
keeps live code; `all` deletes the whole span. Refuses an ambiguous bare variant
group; `--require-match` exits non-zero on no match. Reuses `--dry-run`/`--backup`.
See `docs/superpowers/specs/2026-05-29-marker-insert-strip-list-filters-design.md` §P06.

**Out of Scope**
- `--remove --atomic` (guarded with an error for now; reuse the atomic batch later)
- Multi-line-delimited (`/* … */`) "fully-wrapped body" detection (single-line prefix handled)

### Tests & Tasks
- [x] [P06-T01] `core::remove_section(content, id, mode, comment_style) -> (String, usize)` (3 modes, dup-IDs)
- [x] [P06-TS01] Unit tests: markers / commented / all / duplicate-IDs / not-found
- [x] [P06-T02] `RemoveMode` enum + `--remove` / `--remove-mode` / `--require-match` in `cli.rs`
- [x] [P06-T03] `run_remove` in `main.rs` (walk, variant-group refusal, write via `apply_changes`, match counting)
- [x] [P06-T04] Validation + dispatch; `--remove` conflicts + `--atomic` guard
- [x] [P06-TS02] Integration tests: each mode, recursive, require-match, not-found, variant refusal, dry-run

### Automated Verification
- `cargo test --workspace` green (13 new tests; lib 99→104, integration 135→143)
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## [x] Project P07: List filters (`--fields`) (v0.5.0)
**Goal**: Add `--fields ids|files|lines` to `--list-sections` to filter its text
output. Default `lines` keeps current behavior (backward compatible); `--json`
is unaffected. See `docs/superpowers/specs/2026-05-29-marker-insert-strip-list-filters-design.md` §P07.

### Tests & Tasks
- [x] [P07-T01] `ListFields` enum (clap `ValueEnum`) + `--fields` arg in `cli.rs`
- [x] [P07-T02] Guard: `--fields` requires `--list-sections`
- [x] [P07-T03] Branch `run_list_sections` text output on the level (ids / files / lines)
- [x] [P07-TS01] Integration tests: each level, `lines` == default, `--json` unchanged, requires-guard

### Automated Verification
- `cargo test --workspace` green (5 new integration tests; integration 130 → 135)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `--fields lines` output byte-identical to prior behavior

## [x] Project P08: ABI-Stable C Library (`libtogl`) (v0.3.0)
**Goal**: Add a `togl-ffi` workspace crate exposing an ABI-stable C library
(`libtogl`, static + shared) over `togl-lib`'s string-core and introspection,
with a cbindgen-generated committed header, a C smoke test, and pkg-config.
Also renames the workspace crates to the `togl-*` convention (`togl-lib`, `togl-cli`).
See `docs/superpowers/specs/2026-05-29-togl-c-abi-library-design.md` and
`docs/superpowers/plans/2026-05-29-togl-ffi-c-library.md`.

**Out of Scope**
- Nix flake output and the nixpkgs `libtogl` package (separate follow-on)
- Python and TypeScript bindings (future projects consuming `libtogl`)

### Tests & Tasks
- [x] [P08-T01] Rename crates → `togl-lib` / `togl-cli` for `togl-*` consistency
- [x] [P08-T02] Scaffold `togl-ffi` crate (`[lib] name="togl"`, static+cdylib) → `libtogl`
- [x] [P08-T03] Error codes, panic guard, string + metadata lifecycle
- [x] [P08-T04] Transform functions (toggle_comments, section toggle, activate_variant)
- [x] [P08-T05] Introspection functions returning JSON (discover, scan, validate)
- [x] [P08-T06] cbindgen build script + committed `include/togl.h`
- [x] [P08-TS01] C smoke test linking and exercising `libtogl`
- [x] [P08-T07] pkg-config template + C API README

### Automated Verification
- `cargo test --workspace --all-features` passes (249 tests, incl. FFI unit + C smoke)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `libtogl.a` / `libtogl.dylib` + `include/togl.h` produced

### Detailed Implementation Plan
See `docs/superpowers/plans/2026-05-29-togl-ffi-c-library.md`

---

## [x] Project P09: Release automation (release-please + commitlint) (v0.3.0)
**Goal**: Add PR-driven, Conventional-Commit-based releases modeled on
py-launch-blueprint, adapted to the Rust workspace: release-please opens a
release PR (bumps `[workspace.package].version` + `CHANGELOG.md`); merging it
tags `v*` which triggers the existing `release.yml` binary build. commitlint
enforces Conventional Commits on PRs.
See `docs/superpowers/specs/2026-05-29-release-please-commitlint-design.md`.

**Out of Scope**
- A crates.io publish workflow (separate follow-on)
- Creating the GitHub App / repo secrets (manual; see RELEASE.md)

### Tests & Tasks
- [x] [P09-T01] `release-please-config.json` (simple + extra-files for Cargo.toml workspace version), `.release-please-manifest.json`
- [x] [P09-T02] `.github/workflows/release-please.yml` (App/PAT auth, sync-cargo-lock job)
- [x] [P09-T03] `commitlint.yml` + `commitlint.config.mjs` + `commitlint.dependabot.config.mjs` + `package.json`
- [x] [P09-T04] `RELEASE.md` (flow, disabling, token setup, first-release cutover)

### Automated Verification
- `release-please-config.json` / `.release-please-manifest.json` are valid JSON
- `commitlint.config.mjs` / `commitlint.dependabot.config.mjs` parse as ES modules
- `actionlint` clean on both new workflows

### Manual Steps (maintainer)
- Add `RELEASE_PLEASE_APP_ID` + `RELEASE_PLEASE_PRIVATE_KEY` (GitHub App) or `RELEASE_PLEASE_APP_TOKEN` (PAT) secrets.

---

## [x] Project P10: CLI ergonomics refactor (v0.5.0)
**Goal**: Make the CLI more ergonomic without breaking the existing flat-flag
interface. Three additive layers, shipped in order:
1. **Declarative clap constraints** — replace hand-rolled flag-combination
   checks with clap `group`/`requires`, so impossible combinations fail at parse
   time with consistent messages.
2. **Subcommands** — `toggle`/`scan`/`check`/`list`/`insert`/`remove` expose
   only the flags that apply to each operation. *Additive*: the legacy flat
   flags remain; each subcommand translates to the equivalent legacy argv and is
   re-parsed through the same pipeline, so the two forms cannot drift. The legacy
   form emits a TTY-only deprecation nudge.
3. **stdin/stdout filter** — accept `-`/stdin and write to stdout so `togl`
   composes as a Unix filter.

**Out of Scope**
- Removing the flat-flag interface (deprecate only; removal is a future major).
- A `recover` subcommand (the rare `--recover` path stays flat-flag only).

### Tests & Tasks
- [x] [P10-T01] Option 2: `operation` arg-group + `requires` constraints in `cli.rs`
- [x] [P10-T02] Option 1: `Commands` subcommand enum + `GlobalArgs` flatten struct
- [x] [P10-T03] Option 1: `Commands::to_legacy_argv` translation (clap owns defaults)
- [x] [P10-T04] Option 1: `main()` subcommand bridge + TTY-only deprecation notice
- [x] [P10-TS01] Parity tests: each subcommand ≡ its flat-flag form (incl. a
      defaulted `--remove-mode` case to guard default drift); scoping + help/man render
- [x] [P10-T05] Option 3: stdin/stdout filter mode (`-`/`--stdin`/`--stdout`,
      writer ops only; rejection set for `--json`/`--atomic`/`--backup`/etc.)
- [x] [P10-TS02] Option 3: stdin≡file parity + no-op byte-identity (trailing
      newline both ways) + rejection-set tests

### Automated Verification
- `cargo test` green (all parity + existing suites)
- `cargo clippy --all-targets -- -D warnings` clean
- `togl --help` / `--man` / `--completions` render the subcommands under the aliased bin name

---

## [~] Project P11: crates.io publishing (v0.6.0)
**Goal**: Resume publishing to crates.io (stuck at `togl` 0.2.3 since the
release pipeline only ships GitHub binaries). Reclaim the `togl` crate name for
the CLI and auto-publish the library + CLI on each release tag.

**Decisions**
- Rename the CLI **package** `togl-cli` → `togl` (directory stays `crates/togl-cli`;
  binaries stay `toggle`/`togl`) — continues the existing `togl` 0.2.3 line and
  keeps `cargo install togl` working (already in the README).
- Publish `togl-lib` (library) + `togl` (CLI). `togl-ffi` is `publish = false`
  (it's the C library, not a Rust dependency).
- Inter-crate `togl-lib` dep gets a `version` (required for publishing); the
  release workflow syncs it to the release tag before publishing.

**Out of Scope**
- Publishing `togl-ffi` to crates.io.
- Switching release tooling (release-please/`nix-update` stay as-is).

### Tests & Tasks
- [x] [P11-T01] Rename CLI package → `togl`; add `version` to `togl-lib` dep/dev-dep;
      `togl-ffi` `publish = false`; update `-p togl-cli` refs (flake, test script)
- [x] [P11-T02] `publish-crates` job in `release.yml` (sync dep version → tag,
      publish `togl-lib` then `togl`, `CARGO_REGISTRY_TOKEN`)
- [x] [P11-TS01] `cargo publish --dry-run` (togl-lib full; togl manifest),
      full test suite + `nix build .#togl` green after the rename

### Manual Steps (maintainer)
- Add repo secret `CARGO_REGISTRY_TOKEN` (crates.io API token). Until then the
  publish job fails loudly but does not block the binary build.
- First publish fires on the next release tag (e.g. v0.6.0); crates.io versions
  jump 0.2.3 → that release (allowed — versions only need to increase).
