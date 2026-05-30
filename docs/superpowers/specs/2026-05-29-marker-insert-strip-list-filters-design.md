# Design: Marker Insert, Strip, and List Filters

**Date:** 2026-05-29
**Status:** Approved (brainstorming complete, pending implementation plans)
**Projects:** P05 (`--insert`), P06 (`--remove`), P07 (list filters)

## Context

`toggle` wraps blocks of code in paired marker comments
(`# toggle:start ID=foo desc="..."` / `# toggle:end ID=foo`) and
comments/uncomments the wrapped body. Today the tool can **toggle the comment
state** of existing markers and **discover/scan** them, but it cannot **write
markers** (insert) or **delete markers** (strip). This design adds those two
operations plus a filtering refinement to the existing list output.

### What already exists (verified, not re-built)

- `--list-sections` (`src/main.rs:706`) walks a path, aggregates sections by
  ID, and prints each ID, its `desc`, and every `file:start-end` location, with
  a `--json` variant.
- `--scan` / `--scan -R` / `--scan -S <id>` / `--scan --check` provide per-file
  tables, recursive group summaries, per-group detail, and validation.
- `discover_sections` / `parse_section_desc` (`src/core.rs`) parse marker lines
  including the `desc="..."` value.
- `--dry-run`, `--backup`, `--atomic`, `--recover` provide write safety.
- Mode flags as a precedent: `--scan`, `--check`, `--list-sections`,
  `--recover`, `--completions`, `--man` are all non-toggle modes expressed as
  plain flags. The new operations follow this house style rather than
  introducing subcommands.

### Scope decision

The "list" capability the user described is largely already present. The only
new list work is **output filtering** (P07). The substantive new work is
**insert** (P05) and **strip** (P06). All three ship as separate flag-based
mode flags, one PR each.

## Cross-cutting rules

- **Mutually exclusive mode flags.** `--insert`, `--remove`, `--scan`,
  `--check`, `--list-sections` cannot be combined with each other or with
  ordinary line/section toggling. Violations error before any file write.
- **Reuse existing infrastructure**: `-l` range parsing, comment-style
  auto-detection, `walk`, `io`, `discover_sections`, and the
  `--dry-run`/`--backup`/`--atomic` write paths.

---

## P05 — `--insert` (single-file marker insertion)

### CLI

```bash
toggle --insert -S <ID> -l <start>:<end> [--desc "text"] \
       [--dry-run] [--backup .bak] <file>
```

### Semantics

- `--insert` is a mode flag (see cross-cutting rules).
- Requires **exactly one** `-S <ID>` and **exactly one** `-l` range. Multiple
  `-l` ranges are rejected — they would create duplicate IDs in one file, which
  `--check` (P04) reports as an error.
- `-l` reuses the existing range parser: `start:end`, `start:+count`, and
  `--to-end` all work.
- **Single file only.** A directory, multiple paths, or `-R` is a usage error;
  insert requires concrete line numbers.

### Placement & formatting

- Line numbers are interpreted against the **original** file, **inclusive**.
- `toggle:start` is inserted as a new line immediately **above** `start`;
  `toggle:end` as a new line immediately **below** `end`.
- Insertion is **bottom-up** (end marker first) so the start offset stays valid.
- Comment prefix is auto-detected from the file extension, or overridden with
  `--comment-style`.
- Markers inherit the **leading whitespace of the `start` line**:

  ```python
      # toggle:start ID=featureX desc="text"
      <your block, untouched>
      # toggle:end ID=featureX
  ```

- The body is left **uncommented**. The user runs `toggle -S featureX file.py`
  afterward to flip state. Insert and toggle stay composable and decoupled.

### Guards

- Refuse if `<ID>` already exists anywhere in the file (prevents the
  duplicate-ID state).
- `start > end`, out-of-bounds line numbers, or a malformed ID/desc → clear
  usage error, no write.

### New code

- `core::insert_section(content, id, desc, start, end, comment_style) -> Result<String>`
- `run_insert()` in `src/main.rs`, reusing `io` read/write + `--dry-run` /
  `--backup`.

### Verification

- Insert → `--scan` round-trips: the section appears with the correct `desc`
  and an uncommented ("off") state. This catches a subtly-wrong marker format.
- Unit tests: placement at top/middle/end of file, indentation matching,
  `:+count` and `--to-end` forms, duplicate-ID refusal, out-of-bounds error.

---

## P06 — `--remove` (recursive strip, three modes)

### CLI

```bash
toggle --remove -S <ID> [--remove-mode markers|commented|all] \
       [--require-match] [-R] [--dry-run] [--backup .bak] [--atomic] <paths>...
```

### Semantics

- `--remove` is a mode flag (see cross-cutting rules).
- `-S <ID>` is required and matched **exactly**. A bare group with 2+ variants
  (e.g. `-S db` where `db:sqlite` and `db:postgres` exist) is **refused**: the
  command errors and lists the variants, requiring `-S db:postgres`. `-S
  db:postgres` removes that one section. This mirrors the existing
  group-ambiguity behavior in toggling.
- **Recursive** with `-R`: walks the tree and removes the ID from **every**
  file it appears in — the natural inverse of insert's single-file scope, since
  sections legitimately live across files.

### The three modes (`--remove-mode`, default = `commented`)

| Mode                  | Marker lines | Commented body lines | Live (uncommented) body lines |
|-----------------------|--------------|----------------------|-------------------------------|
| `markers`             | deleted      | kept                 | kept                          |
| `commented` *(default)* | deleted    | deleted              | kept                          |
| `all`                 | deleted      | deleted              | deleted                       |

- **`markers`**: unwrap only — delete the two marker lines, leave the body
  exactly as-is. Never loses content.
- **`commented`** *(default)*: delete markers + any fully-commented lines in the
  body, but **keep live code**. A body line is "commented" if, after leading
  whitespace, it starts with the file's single-line comment prefix. For a
  multi-line-delimited (`/* … */`) section, a fully-wrapped body counts as
  commented.
- **`all`**: delete the whole span including live code. Opt-in only — never the
  default, since it can delete working code.

### Safety & behavior

- Reuses `--dry-run` (shows the diff), `--backup`, and `--atomic`. With `-R`,
  `--atomic` gives all-or-nothing across the tree.
- **ID not found anywhere:** warn + exit 0 by default (consistent with toggle's
  tolerant style). `--require-match` makes it exit non-zero when `-S <ID>`
  matched zero sections.
- **Duplicate ID within a single file** (a `--check` error state): remove **all**
  matching start/end pairs in that file rather than erroring, so strip can clean
  up a messy file.

### New code

- `core::remove_section(content, id, mode, comment_style) -> Result<String>`
- `run_remove()` in `src/main.rs`, reusing `walk` + `io` + the atomic/backup
  paths.

### Verification

- `markers` → `--scan` shows the ID gone, body byte-identical minus the 2 marker
  lines.
- `commented` → commented body lines gone, live lines remain.
- `all` → entire span gone.
- `--require-match` exit-code behavior on a missing ID.
- Variant-group refusal; recursive removal across multiple fixture files.

---

## P07 — list filters

### CLI

```bash
toggle --list-sections --fields <ids|files|lines> <paths>...
```

`--fields` filters the existing `--list-sections` output:

| `--fields`        | Output                                            |
|-------------------|---------------------------------------------------|
| `ids`             | `featureX desc="..."` — IDs + descriptions only   |
| `files`           | IDs + each file path, **no** line numbers         |
| `lines` *(default)* | IDs + `file:start-end` — today's behavior        |

### Semantics

- Default is `lines`, so existing text output and `--json` are **unchanged**
  (backward compatible).
- A single field selector expresses all three levels; no separate
  `--no-line-numbers` boolean (avoids two mechanisms for one outcome).
- Scope: applies to `--list-sections`. The `--scan` tables are unchanged; they
  can be extended later if needed.

### New code

- A `--fields` arg in `src/cli.rs` (enum-validated).
- Branching in `run_list_sections()` (`src/main.rs:706`) on the selected level.

### Verification

- Unit/integration tests asserting each `--fields` level's output, and that
  `lines` (default) plus `--json` are byte-identical to current behavior.

---

## Out of scope

- Subcommands (rejected against the flat-flag precedent).
- Commenting the body on insert (insert leaves code uncommented by design).
- `--fields` for `--scan` tables.
- A `--no-line-numbers` boolean (subsumed by `--fields files`).
- Whole-group strip without a `:variant` qualifier (refused).
