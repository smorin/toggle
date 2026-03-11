<context>
# toggle
A Rust-based CLI and library for toggling, adding, removing, and updating code/comments across multiple languages, featuring flexible extension mappings and full configurability via file or command-line.


Below is a structured technical requirements document for a command-line tool called **`toggle`**. This tool enables toggling comment blocks in one or more files. The focus is on Rust implementation details, supported comment styles, and multi-file toggling. Each section is designed to capture your requirements in a clear, concise format.

All of it should be Thoroughly tested with integration and unit tests. There should both be completely in-memory tests, test file format parsing, and toggling. But there should also be some tests that are on disk for full integration. 

---

## 1. Overview

**Goal**  
Create a Rust-based CLI tool, **`toggle`**, that can:
- Comment or uncomment designated lines or blocks of text in code files.
- Detect and apply correct single-line or multi-line comment styles by file extension.
- Work off a configuration file (`.toggleConfig`) or command-line arguments.
- Identify labeled "sections" to toggle on or off across multiple files.
- Provide granular control (line-based, section-based, file-based, multi-file).

**Core Objectives**  
1. **Line-based toggling**: Support start/end line numbers, or a start line with a fixed number of lines, or a start line to the end of the file.  
2. **Section-based toggling**: Recognize in-file sections tagged with an ID (e.g., `SECTION_ID=foo`) and toggle all occurrences (on/off) across a codebase.  
3. **Configurable comment styles**: Auto-detect comment style by file extension or override with custom settings.  
4. **Extendable**: Allow a `.toggleConfig` file to hold global or per-language comment preferences.

</context>
<PRD>
# Phase 0 – Detailed CLI Specification (Python‑only, **revised 17 Apr 2025**)

## 0.0 Overview
Phase 0 delivers a proof‑of‑concept CLI that toggles single‑line comments (`#`) in a **single** Python source file. It is intentionally minimal yet production‑grade in safety and determinism so we can port the algorithm to Rust in Phase 1.

---

## 0.1 Prerequisites & Platforms
| Requirement                | Minimum / Supported |
|----------------------------|---------------------|
| **Python runtime**         | CPython ≥ 3.9 (3.9 / 3.10 / 3.11 tested) |
| **Operating systems**      | Linux (glibc ≥ 2.31), macOS 11+, Windows 10 (21H2)+ |
| **Filesystem semantics**   | POSIX `rename()` and Windows `ReplaceFileW` available |
| **Reference hardware for perf guard‑rail** | Apple M1 Pro (2021) / 16 GB RAM |

> The PoC relies only on the Python standard library; no third‑party wheels are required.

---

## 0.2 Exact Toggle Algorithm
1. **Range evaluation** – Merge all `--line` ranges (see § 0.5) into a sorted list of non‑overlapping intervals.
2. **Read** – Decode the file using the resolved encoding (see § 0.6).
3. **Skip protected pragmas** – The first *non‑blank* line that matches either pattern is **never toggled**:
   * `^#!` → shebang
   * `^#.*coding[:=]` → PEP 263 encoding pragma
4. **Per‑line toggle** – For every selected line *not* protected above:
   * Locate the first non‑whitespace character index `i`.
   * **Comment → uncomment** if `line[i:].startswith('#')` → delete the `#` and, if present, a single following space.
   * **Uncomment → comment** otherwise → insert `# ` at index `i` (space after `#` mirrors `black` style).
5. **EOL normalisation** – If `--eol` ≠ `preserve`, translate line endings *after* toggling.
6. **Write atomically** – See § 0.7.

> **No parsing** of Python grammar occurs; `#` inside string literals is ignored—it is treated as data, not syntax.

---

## 0.3 JSON Output Schema (`--json`)
When `--json` is supplied *exclusive* of `--verbose`, the CLI writes a single‑line JSON object to **stdout** and nothing to **stderr**.

```jsonc
{
  "file": "samples/hello.py",            // Path supplied by the user
  "ranges": [[3,5]],                     // Final merged ranges (inclusive, 1‑based)
  "action": "invert",                   // "on", "off", or "invert"
  "lines_commented": 2,                  // Count of lines that became comments
  "lines_uncommented": 1,                // Count of lines that became code
  "duration_ms": 1.7,                    // Wall‑clock time, float
  "exit_code": 0                         // Numeric code actually returned
}
```
The schema is stable for Phase 0; new keys will be added only with a minor version bump.

---

## 0.4 Command‑Line Flag Matrix (Phase 0 Purity)
| Flag / Option | Active in Phase 0? | Description (Phase 0 semantics) | First Fully‑Active Phase |
|---------------|--------------------|---------------------------------|--------------------------|
| `-l, --line <range>` | **Yes** | *Required.* Specify line ranges to toggle. | P0 |
| `-f, --force {on\|off}` | **Yes** | Force comment (`on`) or uncomment (`off`) instead of invert. | P0 |
| `-F, --force on\|off\|invert` | No (alias inactive) | Shorthand alias exposing `invert`; behaves like `-f` when enabled. | P1 |
| `-t, --temp-suffix <ext>` | **Yes** | Extension for atomic temp file (`file.py.<ext>`). | P0 |
| `-e, --encoding <name>` | **Yes** | Override file codec (**UTF‑8 only** accepted in P0). | P0 |
| `-v, --verbose` | **Yes** | Human‑readable log lines to `stderr`. | P0 |
| `--json` | **Yes** | Machine‑readable single‑line JSON to `stdout`. | P0 |
| `--strict-ext` | **Yes** | Error (EC01) if target is *not* `.py`. | P0 |
| `-N, --no-dereference` | **Yes** | Operate on the symlink itself instead of target. | P0 |
| `--eol {preserve\|lf\|crlf}` | **Yes** | Preserve or normalise line endings. | P0 |
| `-x, --posix-exit` | **Yes** | Map exit codes to `sysexits.h` values. | P0 |
| `--help`, `--version` | **Yes** | Auto‑generated by Clap; print usage/version. | P0 |
| `--section, -S` | **Reserved** (ignored) | Future section‑marker operations. | P3 |
| `--config <path>` | **Reserved** (ignored) | Specify custom config file path. | P1 |
| `--dry-run` | No | Print diff but make no changes. | P1 |
| `--backup <ext>` | No | Write a backup copy before modifying. | P1 |
| `-R, --recursive` | No | Recurse into sub‑directories. | P5 |
| `--interactive` | No | Prompt before overwriting files. | P2 |

> Any flag marked **No** or **Reserved** will trigger **EC01 / EX_USAGE** if supplied in Phase 0, ensuring determinism for scripts.

---

## 0.5 Line‑Range Grammar & Merge Rules
*Grammar* (unchanged):
```
<range> ::= <N> ':' <M> | <N> ':+"<K>'
```
*Merge algorithm*:
1. Parse all ranges into `[start, end]` pairs (inclusive).
2. Sort ascending by `start` then `end`.
3. Coalesce any interval whose `start` ≤ `prev_end + 1`.
4. The result is a minimal list of non‑overlapping, ascending intervals passed to the toggle engine.

Example: `-l 3:5 -l 4:+4 -l 12:12` → `[[3,8], [12,12]]`.

---

## 0.6 Encoding & Shebang Handling
| Step | Rule |
|------|------|
| 1    | If `--encoding` given, trust it; otherwise check for UTF‑8 BOM; else assume UTF‑8. |
| 2    | If the file cannot be decoded with the chosen codec, exit **EC30 / EX_IOERR**. |
| 3    | Lines protected from toggling: <br>• First shebang line (`^#!`).<br>• First encoding pragma (`^#.*coding[:=]`). |

Phase 0 supports UTF‑8 only; the detection logic is future‑proofed for Phase 3 encodings.

---

## 0.7 Atomic I/O & Symlink Semantics
1. **Dereference default** – Operate on the symlink *target*.
2. **`--no‑dereference`** – Modify the link itself; Phase 0 errors with **EC02** if atomic replace cannot be performed (e.g., hardlink to another device).
3. **Cross‑device fallback** – If `os.rename` fails with `EXDEV`, fall back to `shutil.copy2` + `fsync` + `os.replace`; this is still atomic on the destination device.
4. **Symlink cycles** – Not detected; user responsibility. Documented as non‑goal in Phase 0.

---

## 0.8 Exit Codes (Phase 0 mapping)
| Condition | Tag | Default | `--posix-exit` |
|-----------|-----|---------|----------------|
| Success | EC00 | **0** | **0 (EX_OK)** |
| Bad CLI / range | EC01 | **1** | **64 (EX_USAGE)** |
| File R/W error | EC02 | **2** | **74 (EX_IOERR)** |
| Toggle logic issue | EC03 | **3** | **70 (EX_SOFTWARE)** |
| Internal panic | EC04 | **4** | **71 (EX_OSERR)** |

The string tags (ECxx) appear only in human‑readable messages.

---

## 0.9 Benchmark Harness
* Fixture: `bench/fixture_1000.py` (1 000 lines, 60 % commented, 40 % code).
* Command: `python -m pytest -k bench` (uses `pytest‑benchmarks`).
* CI Budget: mean ≤ 10 ms, stdev < 1 ms on reference hardware.
* Local quick check: `hyperfine --warmup 3 "toggle -l 1:+1000 bench/fixture_1000.py"`.

---

## 0.10 Logging
| Level | Env Var | Stream | Notes |
|-------|---------|--------|-------|
| ERROR | `TOGGLE_LOG=error` | `stderr` | default |
| DEBUG | `TOGGLE_LOG=debug` | `stderr` | prints parsed ranges, timing |

`--verbose` implies `TOGGLE_LOG=debug` when the env var is unset.

---

## 0.11 Contribution Quick Start
```bash
python -m venv venv && source venv/bin/activate
pip install -r requirements-dev.txt
make test      # runs unit + golden tests
make bench     # runs performance guard‑rail
```
See `CONTRIBUTING.md` for commit message convention and pre‑push hooks.

---

## 0.12 Non‑Goals Explicitly Out‑of‑Scope for Phase 0
* Recursive directory traversal
* Multi‑file atomic groups
* Section markers
* Encodings other than UTF‑8
* IDE or editor integration hooks

These will be addressed in later phases as detailed in the main PRD roadmap.

---

## 0.13 Section Variants (Paired & Grouped Sections)

### 0.13.1 Motivation

Solo sections (`ID=foo`) toggle a single block on or off. **Variants** extend this by linking multiple sections under a shared group — toggling one variant on simultaneously toggles its siblings off. The primary use case is swapping between two alternative implementations (e.g., dev vs prod config, SQLite vs Postgres backend).

### 0.13.2 Marker Syntax

Variants use a **colon separator** in the section ID: `group:variant`.

```python
# toggle:start ID=db:sqlite desc="SQLite backend"
import sqlite3
conn = sqlite3.connect("app.db")
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres desc="Postgres backend"
# import psycopg2
# conn = psycopg2.connect("host=localhost")
# toggle:end ID=db:postgres
```

- The **group** is the portion before the colon (`db`).
- The **variant** is the portion after the colon (`sqlite`, `postgres`).
- IDs without a colon remain **solo sections** with unchanged behavior.
- The `desc="..."` attribute remains optional and applies per-variant.
- Variant markers follow the same `toggle:start` / `toggle:end` convention — no new keywords are added to file markers.

### 0.13.3 CLI Behavior

| CLI Argument | Variants Found | Behavior |
|---|---|---|
| `-S debug` (no colon, solo) | 1 | **Solo** — invert/force as existing behavior |
| `-S db` (group, no variant) | 2 | **Pair flip** — swap active/inactive |
| `-S db` (group, no variant) | 3+ | **Error** — `"group 'db' has N variants; specify one with -S db:<name>"` |
| `-S db:postgres` | any | **Activate** — uncomment `db:postgres`, comment all other `db:*` variants |
| `-S db --force on` | any | **Force all** — comment every `db:*` variant |
| `-S db --force off` | any | **Force all** — uncomment every `db:*` variant |

### 0.13.4 `--pair` Enforcement Flag

The `--pair` flag is a **pre-execution validation guard**. When supplied, the tool scans for all variants in the targeted group and errors if the count is not exactly 2. No file modifications occur on failure.

```bash
# Succeeds — db has exactly 2 variants
toggle -S db --pair myfile.py

# Errors before any changes
# Error: --pair: group 'db' has 3 variants, expected exactly 2
toggle -S db --pair myfile.py
```

`--pair` is optional. Without it, a group of 2 still flips via `-S <group>`. The flag adds an explicit guardrail for cases where the pairing contract must be enforced.

| Flag | Type | Description |
|---|---|---|
| `--pair` | `bool` | Enforce exactly 2 variants in the targeted group. Error otherwise. |

### 0.13.5 Multi-File Variant Toggling

Variants work across files with `-R` (recursive) or multi-path arguments. The tool collects all occurrences of the group's variants across all targeted files and applies the toggle atomically.

```bash
# Flip db pair across entire src/ tree
toggle -S db --pair -R src/

# Activate postgres everywhere
toggle -S db:postgres -R src/
```

Cross-file consistency: all files should contain the same set of variants for a given group. Mismatches are reported as warnings (or errors with `--check`).

---

## 0.14 Section Scan Enhancements

### 0.14.1 Variant-Aware Scan Output

The existing `--scan` flag is extended to understand variant groups. Output groups variants under their parent group.

**Default table output:**

```
SECTION          TYPE    STATE        LINES     DESCRIPTION
debug            solo    commented    12-18     Debug output
db:sqlite        pair    active       24-27     SQLite backend
db:postgres      pair    commented    29-33     Postgres backend
feature          solo    active       40-55     Experimental feature
```

**Recursive summary (`--scan -R src/`):**

```
SECTION          TYPE    FILES   VARIANTS  STATE
debug            solo    3       —         mixed
db               pair    5       2         ok
cache            group   2       3         ok
feature          solo    1       —         active
```

The `TYPE` column is inferred:
- **solo** — ID has no colon
- **pair** — group has exactly 2 variants
- **group** — group has 3+ variants

### 0.14.2 Detailed Section View (`--scan -S <id>`)

When `--scan` is combined with `-S`, show detailed file references for one section or group.

```bash
toggle --scan -S db -R src/
```

```
GROUP: db (pair, 2 variants)

  db:sqlite [active]
    src/config.py          lines 24-27
    src/models/base.py     lines 8-14
    tests/conftest.py      lines 3-9

  db:postgres [commented]
    src/config.py          lines 29-33
    src/models/base.py     lines 16-22
    tests/conftest.py      lines 11-17
```

### 0.14.3 Validation Mode (`--scan --check`)

The `--check` flag performs read-only validation and reports issues.

```bash
toggle --scan --check -R src/
```

```
OK    debug            solo    3 files
OK    db               pair    5 files, 2 variants
WARN  cache            group   2 files, 3 variants (src/app.py missing cache:redis)
ERR   auth             pair    1 file, 3 variants (--pair would fail: expected 2)
```

Checks performed:
- All start markers have matching end markers
- `pair`-inferred groups have exactly 2 variants (warning if not)
- Variant sets are consistent across files (same variants present in every file that references the group)
- No duplicate section IDs within a single file

When combined with `--pair`, only groups that should be pairs are checked:
```bash
toggle --scan --check --pair -R src/
```

### 0.14.4 JSON Output

All scan modes support `--json` for machine-readable output.

```bash
toggle --scan -R src/ --json
```

```json
{
  "sections": [
    {
      "id": "debug",
      "type": "solo",
      "files": [
        {"path": "src/app.py", "start": 12, "end": 18, "state": "commented", "desc": "Debug output"}
      ]
    },
    {
      "group": "db",
      "type": "pair",
      "variants": [
        {
          "id": "db:sqlite",
          "state": "active",
          "files": [
            {"path": "src/config.py", "start": 24, "end": 27},
            {"path": "src/models/base.py", "start": 8, "end": 14}
          ]
        },
        {
          "id": "db:postgres",
          "state": "commented",
          "files": [
            {"path": "src/config.py", "start": 29, "end": 33},
            {"path": "src/models/base.py", "start": 16, "end": 22}
          ]
        }
      ]
    }
  ]
}
```

### 0.14.5 New CLI Flags Summary

| Flag | Type | Phase | Description |
|---|---|---|---|
| `--pair` | `bool` | P3 | Enforce exactly 2 variants in targeted group |
| `--check` | `bool` | P3 | Validate section integrity without modifying files |
| `-S` with `--scan` | `String` | P3 | Filter scan output to a specific section or group |

</PRD>