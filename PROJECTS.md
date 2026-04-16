# PROJECTS

Tracks implementation work for `toggle`. Each project corresponds to one PR.
The PRD source of truth lives in `prds/phase-0-prd.md` (notably §0.13 and §0.14).

**Status Legend:**
- `[x]` Completed
- `[-]` In Progress
- `[ ]` Not Started
- `[~]` Won't fix / Invalid / False positive

---

> **Release plan:** P01–P04 land together as a single `v0.2.0` release on `main`.
> Intermediate version bumps in this file are notional; only the final commit
> after P04 bumps `Cargo.toml`.

## [ ] Project P01: Section Variants Core (v0.2.0)
**Goal**: Implement `group:variant` section markers and the four CLI variant
behaviors from PRD §0.13.3 — solo, pair-flip, activate, force-all — with
per-file scope. Errors when a group has 3+ variants and `-S` lacks a
`:variant` qualifier.

**Out of Scope**
- The `--pair` validation flag (P02)
- Cross-file consistency checks (P04)
- Scan output changes (P03)

### Tests & Tasks
- [ ] [P01-T01] Add `parse_id_parts(id) -> (group, Option<variant>)` helper to `src/core.rs`
- [ ] [P01-TS01] Unit tests in `tests/unit/core_tests.rs` covering solo, pair, edge cases
- [ ] [P01-T02] Add `discover_variants(content, group) -> Vec<SectionInfo>` helper to `src/core.rs`
- [ ] [P01-TS02] Unit tests for `discover_variants` (no matches, 1, 2, 3 variants, mixed solo+variant)
- [ ] [P01-T03] Add `toggle_variant_group(...)` and `activate_variant(...)` functions to `src/core.rs`
- [ ] [P01-TS03] Unit tests covering pair-flip, activate, force-on-all, force-off-all, error on 3+
- [ ] [P01-T04] Route variant CLI arguments in `src/main.rs::toggle_section` to the new core functions
- [ ] [P01-TS04] Integration tests in `tests/integration.rs` exercising each CLI form against a fixture file
- [ ] [P01-T05] Add `tests/fixtures/variants.py` with a pair group and a 3-variant group
- [ ] [P01-T06] Update CLI help text on `-S` to mention variant syntax
- [ ] [P01-T07] Run `just dev` (format, lint, test, build) and ensure clean
- [ ] [P01-T08] Run `just dev`, commit (no version bump yet)

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

## [ ] Project P02: `--pair` Validation Flag (v0.2.1)
**Goal**: Add the `--pair` pre-execution guard from PRD §0.13.4 that errors
when the targeted group does not contain exactly 2 variants. No file
modifications occur on failure.

### Tests & Tasks
- [ ] [P02-T01] Add `--pair` flag to `src/cli.rs`
- [ ] [P02-TS01] Unit test that `--pair` requires `-S <group>` (no line ranges)
- [ ] [P02-T02] Insert pre-execution validation in `src/main.rs` before file mutations
- [ ] [P02-TS02] Integration tests: success on 2-variant group; error on 1, 3, 4
- [ ] [P02-T03] Update README / help text
- [ ] [P02-T04] Run `just dev`
- [ ] [P02-T05] Bump `Cargo.toml` to `0.2.1`, commit, tag `v0.2.1`

---

## [ ] Project P03: Variant-Aware Scan Output (v0.3.0)
**Goal**: Extend `--scan` per PRD §0.14.1–§0.14.2 and §0.14.4: group variants
under their parent group, infer the `TYPE` column (solo/pair/group), add
recursive summary mode, the `--scan -S <id>` detailed view, and JSON output
that nests variants under groups.

### Tests & Tasks
- [ ] [P03-T01] Extend `ScanSectionInfo` (or add `ScanGroupInfo`) in `src/core.rs` with group/type/variant fields
- [ ] [P03-TS01] Unit tests for type inference (solo / pair / group / mixed)
- [ ] [P03-T02] Update `print_scan_results` and `run_scan` in `src/main.rs`
- [ ] [P03-TS02] Integration: per-file table, recursive summary, `--scan -S db`
- [ ] [P03-T03] Update JSON schema per §0.14.4
- [ ] [P03-TS03] Snapshot test for JSON output
- [ ] [P03-T04] Run `just dev`
- [ ] [P03-T05] Bump version to `0.3.0`, commit, tag `v0.3.0`

---

## [ ] Project P04: `--check` Validation Mode (v0.3.1)
**Goal**: Add `--scan --check` per PRD §0.14.3 — read-only validation that
reports unclosed markers, pair groups with ≠2 variants, cross-file variant
mismatches, and duplicate IDs within a single file.

### Tests & Tasks
- [ ] [P04-T01] Add `--check` flag in `src/cli.rs` (only valid with `--scan`)
- [ ] [P04-T02] Add validation logic in `src/core.rs` returning a list of `CheckIssue { level, group, message }`
- [ ] [P04-TS01] Unit tests for each check (unclosed, pair-mismatch, file-mismatch, duplicate ID)
- [ ] [P04-T03] Wire `--check` output (table + JSON) into `src/main.rs`
- [ ] [P04-TS02] Integration tests with multi-file fixture trees
- [ ] [P04-T04] Behavior with `--check --pair` (only checks pair-inferred groups)
- [ ] [P04-T05] Run `just dev`
- [ ] [P04-T06] Bump version to `0.3.1`, commit, tag `v0.3.1`
