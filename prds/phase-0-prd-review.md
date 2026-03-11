# Phase 0 PRD Review — Bugs, Inconsistencies & Unclear Items

**Reviewed:** 2026-03-11
**PRD:** `prds/phase-0-prd.md` (revised 17 Apr 2025)
**Codebase:** Rust implementation in `src/`

---

## Category 1: BUGS (PRD states something incorrect)

### BUG-1: Line-Range Grammar has a typo (§0.5)
**Severity:** Medium

The grammar specification contains a stray double-quote:
```
<range> ::= <N> ':' <M> | <N> ':+"<K>'
```
Should be:
```
<range> ::= <N> ':' <M> | <N> ':+' <K>
```
The `"` before `<K>` is erroneous and the surrounding quotes are inconsistent.

### BUG-2: Grammar is incomplete — missing single-line form (§0.5)
**Severity:** Medium

The grammar only defines two forms (`N:M` and `N:+K`), but the implementation (`core.rs`) also supports a bare single-line form (just `N`), which resolves to `(N, N)`. The grammar should include:
```
<range> ::= <N> | <N> ':' <M> | <N> ':+' <K>
```

### BUG-3: `--no-dereference` description is confusing/inverted (§0.7)
**Severity:** Medium

§0.7 step 2 says "`--no-dereference` – Modify the link itself." This is confusing because you cannot meaningfully "modify" a symlink as a text file. The standard POSIX meaning (and the implementation's behavior) is: do not resolve symlinks when determining the target path for the atomic replace step. The description should be clarified to match standard POSIX semantics.

---

## Category 2: INCONSISTENCIES (PRD contradicts itself or the implementation)

### INC-1: JSON schema doesn't match implementation (§0.3)
**Severity:** High

The PRD specifies these JSON output fields that do **not** exist in the implementation:
- `"ranges"` — not emitted
- `"action": "on" | "off" | "invert"` — implementation emits `"toggle_line_range"` or `"toggle_section"`
- `"lines_commented"` / `"lines_uncommented"` — implementation uses a single `"lines_changed"` counter
- `"duration_ms"` — not emitted
- `"exit_code"` — not emitted

The implementation emits fields **not** in the PRD:
- `"success"` (bool)
- `"error"` (optional string)
- `"dry_run"` (bool)
- `"section_id"` (optional string)
- `"desc"` (optional string)

### INC-2: `-f` vs `-F` flag semantics contradict (§0.4)
**Severity:** Medium

The PRD defines:
- `-f, --force {on|off}` — Active in P0, two values only
- `-F, --force on|off|invert` — **Inactive** alias until P1

The implementation has a single `-f/--force` flag with `-F` as a visible short alias, already accepting `on`, `off`, **and** `invert` in Phase 0.

### INC-3: "Reserved" / "No" flags are fully implemented (§0.4)
**Severity:** High

The PRD states: *"Any flag marked No or Reserved will trigger EC01 / EX_USAGE if supplied in Phase 0."* But these flags are all fully implemented:

| Flag | PRD Status | Actual Status |
|------|-----------|---------------|
| `--section, -S` | Reserved (P3) | Fully implemented |
| `--config <path>` | Reserved (P1) | Fully implemented |
| `--dry-run` | No (P1) | Fully implemented |
| `--backup <ext>` | No (P1) | Fully implemented |
| `-R, --recursive` | No (P5) | Fully implemented |
| `--interactive` | No (P2) | Fully implemented |

### INC-4: PRD says "Python-only" but implementation is multi-language
**Severity:** High

The PRD title is "Python-only" and `--strict-ext` enforces `.py`. The implementation supports 24+ languages with native comment styles (JS, Rust, Go, Java, C/C++, etc.).

### INC-5: PRD says "single file" but implementation supports multiple files/directories
**Severity:** High

§0.0 says Phase 0 works on "a single Python source file." The implementation accepts multiple paths (`Vec<PathBuf>`), supports directory recursion, and features multi-file atomic batch operations with a journal-based two-phase commit protocol.

### INC-6: PRD says "standard library only" but implementation uses third-party crates
**Severity:** Low

§0.1 says "The PoC relies only on the Python standard library; no third-party wheels are required." The implementation is in Rust and uses clap, serde, serde_json, anyhow, encoding_rs, criterion, fd-lock, tempfile, etc. This prerequisite is stale from the Python PoC concept.

### INC-7: `--json` exclusivity with `--verbose` not enforced (§0.3)
**Severity:** Medium

The PRD says `--json` is "supplied exclusive of --verbose" and outputs "nothing to stderr." The implementation does not enforce mutual exclusivity — both flags can be active simultaneously.

### INC-8: Benchmark harness doesn't match implementation (§0.9)
**Severity:** Low

PRD specifies `python -m pytest -k bench` using `pytest-benchmarks`. Implementation uses Criterion for Rust (`cargo bench`). The described fixture `bench/fixture_1000.py` doesn't exist as a static file — benchmarks generate content programmatically.

---

## Category 3: UNCLEAR / AMBIGUOUS

### UNC-1: `invert` mode toggle heuristic is underspecified (§0.2)
**Severity:** High

Step 4 describes pure per-line inversion: if line starts with `#`, uncomment; otherwise, comment. The implementation uses a **majority vote** heuristic: if the majority of non-empty, non-protected lines in the range are commented, ALL are uncommented (and vice versa). This significant behavioral difference is not documented in the PRD.

### UNC-2: "# inside string literals" note is misleading (§0.2)
**Severity:** Medium

The note says "`#` inside string literals is ignored—it is treated as data, not syntax." This reads like a feature, but it's actually a **known limitation**: since no parsing occurs, `#` inside strings **will** be incorrectly toggled. The wording should clarify this is a caveat, not a capability.

### UNC-3: Cross-device fallback atomicity claim is imprecise (§0.7)
**Severity:** Low

Step 3 says the fallback "is still atomic on the destination device." But `os.replace` also fails with `EXDEV` if source and destination are on different devices. The description needs to clarify that the temp file must be created on the same device as the target.

### UNC-4: `--eol` scope is ambiguous (§0.2)
**Severity:** Medium

Step 5 says "translate line endings after toggling." It's unclear whether this applies to only the toggled lines or the entire file. The implementation normalizes the **entire file's** line endings.

### UNC-5: `--verbose` vs `TOGGLE_LOG` env var precedence unclear (§0.10)
**Severity:** Low

The PRD says "`--verbose` implies `TOGGLE_LOG=debug` when the env var is unset." It doesn't specify what happens when both `--verbose` and `TOGGLE_LOG=error` are set simultaneously. Which takes precedence?

### UNC-6: Hardlink vs symlink confusion in `--no-dereference` error case (§0.7)
**Severity:** Low

Step 2 says Phase 0 errors with EC02 "if atomic replace cannot be performed (e.g., hardlink to another device)." Hardlinks and symlinks are different concepts. The example scenario is unclear — is it about symlinks pointing across devices, or actual hardlinks?

### UNC-7: Protected line behavior when explicitly in toggle range (§0.2)
**Severity:** Medium

Step 3 says shebangs and encoding pragmas are "never toggled." It doesn't specify what happens when the user explicitly includes line 1 (a shebang) in a `--line 1:5` range. Does the tool silently skip the protected line? Emit a warning? This should be documented.

### UNC-8: `--to-end` flag missing from PRD (§0.4)
**Severity:** Medium

The implementation has a `--to-end` flag that extends the last `--line` range to the end of the file. This flag is not mentioned anywhere in the PRD's flag matrix or any other section.

---

## Summary

| Category | Count | Severity Breakdown |
|----------|-------|-------------------|
| Bugs | 3 | 3 Medium |
| Inconsistencies | 8 | 4 High, 2 Medium, 2 Low |
| Unclear / Ambiguous | 8 | 1 High, 4 Medium, 3 Low |
| **Total** | **19** | |

### Recommended Priority

1. **INC-1** (JSON schema mismatch) — breaks any consumer relying on the documented schema
2. **INC-3** (reserved flags implemented) — PRD is misleading about Phase 0 scope
3. **INC-4 + INC-5** (Python-only / single-file claims) — PRD header is fundamentally outdated
4. **UNC-1** (majority vote heuristic) — undocumented core algorithm behavior
5. **BUG-1 + BUG-2** (grammar typo & missing form) — spec is unparseable / incomplete
