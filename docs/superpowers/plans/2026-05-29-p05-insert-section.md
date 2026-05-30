# P05 — `--insert` (Single-File Marker Insertion) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `--insert` mode that wraps a single file's `-l start:end` block in a `toggle:start`/`toggle:end` marker pair, leaving the body uncommented.

**Architecture:** A pure string transform `core::insert_section` does the work; `main.rs::run_insert` resolves the comment prefix, parses the range (reusing `parse_line_range`), and reuses the existing `apply_changes` path for dry-run/backup/write. CLI exposes `--insert` and `--desc` as flags, consistent with existing mode flags (`--scan`, `--list-sections`).

**Tech Stack:** Rust, clap (CLI), anyhow (errors), assert_cmd + predicates + tempfile (integration tests).

**Reference:** Design spec at `docs/superpowers/specs/2026-05-29-marker-insert-strip-list-filters-design.md` (P05 section).

---

## File Structure

- `src/core.rs` — add `insert_section()` (pure transform; lives beside `discover_sections` so it can reuse the private `parse_section_id` helper).
- `src/cli.rs` — add `--insert` (bool) and `--desc` (Option<String>) fields.
- `src/main.rs` — add `--insert` validation in `run()` and a `run_insert()` dispatch function.
- `tests/unit/core_tests.rs` — unit tests for `insert_section`.
- `tests/integration.rs` — end-to-end CLI tests for `--insert`.
- `README.md`, `PROJECTS.md` — docs + project tracking.

### Key facts about the existing code (verified)

- `core::parse_line_range(spec) -> Result<(usize, usize)>` returns a 1-based inclusive `(start, end)` and already supports `start:end` and `start:+count` (`src/core.rs:259`).
- Content is reconstructed with: `let mut result = lines.join("\n"); if content.ends_with('\n') { result.push('\n'); }` (`src/core.rs:432-437`). `insert_section` MUST mirror this for EOL parity.
- `parse_section_id(line) -> Option<String>` is a private fn in `core.rs:56`; `insert_section` is in the same module so it can call it for the duplicate-ID guard.
- `main.rs::apply_changes(path, original, modified, opts) -> Result<usize>` handles dry-run (prints diff), interactive prompt, backup, and the encoded write (`src/main.rs:833`).
- `main.rs::resolve_comment_style(path, opts) -> Result<core::CommentStyle>` applies `--comment-style` override then falls back to `get_comment_style` (`src/main.rs:887`). `CommentStyle.single_line: String` is the prefix to use.
- Integration test helpers `setup_temp_file(content, filename)` and `cmd()` already exist (`tests/integration.rs:11`).

### EOL handling

`insert_section` is a pure transform that builds output with LF (`lines.join("\n")` + preserved trailing newline). To stay consistent with the normal toggle paths, `run_insert` applies `io::normalize_eol(&modified, opts.eol)` after `insert_section` and before `apply_changes`, so `--insert --eol crlf` emits CRLF just like a normal toggle. (Resolved during final review; covered by `test_insert_respects_eol_crlf`.)

---

## Task 1: `core::insert_section` — happy path

**Files:**
- Modify: `src/core.rs` (add `insert_section` after `discover_variants`, ~line 143)
- Test: `tests/unit/core_tests.rs`

- [ ] **Step 1: Write the failing test**

In `tests/unit/core_tests.rs`, add `insert_section` to the existing `use toggle::core::{...}` import block, then add:

```rust
// ── insert_section ──

#[test]
fn test_insert_section_basic() {
    let content = "a\nb\nc\nd\n";
    // Wrap lines 2..3 (1-based inclusive) with ID=feat
    let result = insert_section(content, "feat", None, 2, 3, "#").unwrap();
    assert_eq!(
        result,
        "a\n# toggle:start ID=feat\nb\nc\n# toggle:end ID=feat\nd\n"
    );
}

#[test]
fn test_insert_section_with_desc() {
    let content = "a\nb\n";
    let result = insert_section(content, "feat", Some("hello world"), 1, 2, "//").unwrap();
    assert_eq!(
        result,
        "// toggle:start ID=feat desc=\"hello world\"\na\nb\n// toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_section_matches_indentation() {
    let content = "def f():\n    x = 1\n    y = 2\n";
    // Wrap the two indented lines (2..3)
    let result = insert_section(content, "feat", None, 2, 3, "#").unwrap();
    assert_eq!(
        result,
        "def f():\n    # toggle:start ID=feat\n    x = 1\n    y = 2\n    # toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_section_no_trailing_newline() {
    let content = "a\nb"; // no trailing newline
    let result = insert_section(content, "feat", None, 1, 2, "#").unwrap();
    assert_eq!(result, "# toggle:start ID=feat\na\nb\n# toggle:end ID=feat");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test unit insert_section`
Expected: FAIL — `cannot find function insert_section in module toggle::core` (compile error).

- [ ] **Step 3: Write minimal implementation**

In `src/core.rs`, after the `discover_variants` function (~line 143), add:

```rust
/// Insert a `toggle:start`/`toggle:end` marker pair around the 1-based inclusive
/// line range `[start, end]`. Markers inherit the leading whitespace of the
/// `start` line and use `comment_prefix` (e.g. `"#"`, `"//"`). The body is left
/// unchanged (uncommented). Returns the new file content.
///
/// Errors if the range is invalid, out of bounds, the ID/desc is malformed, or
/// a section with `id` already exists in the file.
pub fn insert_section(
    content: &str,
    id: &str,
    desc: Option<&str>,
    start: usize,
    end: usize,
    comment_prefix: &str,
) -> Result<String> {
    if id.is_empty() || id.contains(char::is_whitespace) || id.contains('"') {
        return Err(UsageError(format!("Invalid section ID: '{}'", id)).into());
    }
    if let Some(d) = desc {
        if d.contains('"') {
            return Err(UsageError("Section description must not contain '\"'".into()).into());
        }
    }
    if start == 0 || end < start {
        return Err(UsageError(format!("Invalid line range: {}:{}", start, end)).into());
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    if end > lines.len() {
        return Err(UsageError(format!(
            "Line range {}:{} exceeds file length ({} lines)",
            start,
            end,
            lines.len()
        ))
        .into());
    }

    // Duplicate-ID guard: refuse if any start marker already uses this ID.
    for line in &lines {
        if line.contains("toggle:start") && parse_section_id(line).as_deref() == Some(id) {
            return Err(UsageError(format!("Section ID '{}' already exists in file", id)).into());
        }
    }

    let indent: String = lines[start - 1]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    let start_marker = match desc {
        Some(d) => format!("{}{} toggle:start ID={} desc=\"{}\"", indent, comment_prefix, id, d),
        None => format!("{}{} toggle:start ID={}", indent, comment_prefix, id),
    };
    let end_marker = format!("{}{} toggle:end ID={}", indent, comment_prefix, id);

    // Insert bottom-up so the start index stays valid: end marker goes after
    // line `end` (0-based index `end`), start marker before line `start`.
    lines.insert(end, end_marker);
    lines.insert(start - 1, start_marker);

    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test unit insert_section`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): add insert_section marker insertion (P05)"
```

---

## Task 2: `insert_section` guards

**Files:**
- Modify: `src/core.rs` (no new code — guards already written in Task 1)
- Test: `tests/unit/core_tests.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/unit/core_tests.rs`:

```rust
#[test]
fn test_insert_section_rejects_duplicate_id() {
    let content = "# toggle:start ID=feat\nx\n# toggle:end ID=feat\ny\n";
    let err = insert_section(content, "feat", None, 4, 4, "#");
    assert!(err.is_err());
}

#[test]
fn test_insert_section_rejects_out_of_bounds() {
    let content = "a\nb\n";
    assert!(insert_section(content, "feat", None, 1, 5, "#").is_err());
}

#[test]
fn test_insert_section_rejects_bad_id() {
    let content = "a\nb\n";
    assert!(insert_section(content, "a b", None, 1, 2, "#").is_err());
    assert!(insert_section(content, "", None, 1, 2, "#").is_err());
}

#[test]
fn test_insert_section_rejects_quote_in_desc() {
    let content = "a\nb\n";
    assert!(insert_section(content, "feat", Some("has \" quote"), 1, 2, "#").is_err());
}

#[test]
fn test_insert_section_rejects_inverted_range() {
    let content = "a\nb\nc\n";
    assert!(insert_section(content, "feat", None, 3, 1, "#").is_err());
}
```

- [ ] **Step 2: Run test to verify it passes (guards already implemented)**

Run: `cargo test --test unit insert_section`
Expected: PASS (9 tests total). If any guard test fails, fix the corresponding check in `insert_section`.

- [ ] **Step 3: Commit**

```bash
git add tests/unit/core_tests.rs
git commit -m "test(core): cover insert_section guards (P05)"
```

---

## Task 3: CLI flags + validation + `run_insert`

**Files:**
- Modify: `src/cli.rs` (add `--insert`, `--desc`)
- Modify: `src/main.rs` (validation in `run()`, new `run_insert()`, dispatch)
- Test: `tests/integration.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/integration.rs`:

```rust
// ── --insert (P05) ──

#[test]
fn test_insert_basic() {
    let (_dir, path) = setup_temp_file("a\nb\nc\nd\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "2:3"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(
        result,
        "a\n# toggle:start ID=feat\nb\nc\n# toggle:end ID=feat\nd\n"
    );
}

#[test]
fn test_insert_with_desc() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([
            path.to_str().unwrap(),
            "--insert",
            "-S",
            "feat",
            "-l",
            "1:2",
            "--desc",
            "my note",
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(
        result,
        "# toggle:start ID=feat desc=\"my note\"\na\nb\n# toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_round_trips_through_scan() {
    let (_dir, path) = setup_temp_file("a\nb\nc\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "1:2"])
        .assert()
        .success();
    cmd()
        .args([path.to_str().unwrap(), "--scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feat"));
}

#[test]
fn test_insert_rejects_duplicate_id() {
    let (_dir, path) =
        setup_temp_file("# toggle:start ID=feat\nx\n# toggle:end ID=feat\ny\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "4:4"])
        .assert()
        .failure();
}

#[test]
fn test_insert_requires_single_section() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([
            path.to_str().unwrap(),
            "--insert",
            "-S",
            "a",
            "-S",
            "b",
            "-l",
            "1:2",
        ])
        .assert()
        .failure();
}

#[test]
fn test_insert_rejects_recursive() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "1:2", "-R"])
        .assert()
        .failure();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration test_insert_`
Expected: FAIL — `--insert` / `--desc` are unknown args (clap error), tests fail.

- [ ] **Step 3: Add the CLI flags**

In `src/cli.rs`, after the `--list-sections` field (~line 29), add:

```rust
    /// Insert a toggle:start/end marker pair around a single -l range (single file).
    /// Requires exactly one -S <ID> and one -l <range>. Leaves the body uncommented.
    #[arg(long = "insert")]
    pub insert: bool,

    /// Description for the inserted section marker (use with --insert).
    #[arg(long = "desc")]
    pub desc: Option<String>,
```

- [ ] **Step 4: Add validation + dispatch in `main.rs`**

In `src/main.rs::run()`, immediately **before** the `let opts = ToggleOptions {` block (~line 305), add:

```rust
    // ── --insert mode validation (P05) ──
    if cli.insert {
        if cli.scan || cli.list_sections {
            return Err(UsageError("--insert cannot be combined with --scan or --list-sections".into()).into());
        }
        if cli.force.is_some() {
            return Err(UsageError("--insert does not take --force (the body is left uncommented)".into()).into());
        }
        if cli.recursive {
            return Err(UsageError("--insert operates on a single file; -R is not allowed".into()).into());
        }
        if cli.paths.len() != 1 {
            return Err(UsageError("--insert requires exactly one file path".into()).into());
        }
        if cli.sections.len() != 1 {
            return Err(UsageError("--insert requires exactly one -S <ID>".into()).into());
        }
        if cli.lines.len() != 1 {
            return Err(UsageError("--insert requires exactly one -l <range>".into()).into());
        }
    } else if cli.desc.is_some() {
        return Err(UsageError("--desc is only valid with --insert".into()).into());
    }
```

Then in the dispatch chain at the end of `run()` (~line 322), change:

```rust
    if cli.list_sections {
        run_list_sections(cli, &opts)
```

to:

```rust
    if cli.insert {
        run_insert(cli, &opts)
    } else if cli.list_sections {
        run_list_sections(cli, &opts)
```

Then add the `run_insert` function near `run_list_sections` (~line 706):

```rust
fn run_insert(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    let path = &cli.paths[0];
    let comment_prefix = resolve_comment_style(path, opts)?.single_line;
    let content = io::read_file_encoded(path, opts.encoding)?;

    let (start, mut end) = core::parse_line_range(&cli.lines[0])?;
    if opts.to_end {
        end = content.lines().count();
    }

    let id = &cli.sections[0];
    let modified = core::insert_section(
        &content,
        id,
        cli.desc.as_deref(),
        start,
        end,
        &comment_prefix,
    )?;

    apply_changes(path, &content, &modified, opts)?;

    if opts.verbose {
        eprintln!(
            "Inserted section '{}' into {} (lines {}-{})",
            id,
            path.display(),
            start,
            end
        );
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test integration test_insert_`
Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs tests/integration.rs
git commit -m "feat(cli): wire --insert mode and --desc flag (P05)"
```

---

## Task 4: `--to-end`, comment-style override, dry-run

**Files:**
- Test: `tests/integration.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_insert_to_end() {
    let (_dir, path) = setup_temp_file("a\nb\nc\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "2", "--to-end"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(
        result,
        "a\n# toggle:start ID=feat\nb\nc\n# toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_comment_style_override() {
    // .txt has no known comment style; --comment-style supplies one.
    let (_dir, path) = setup_temp_file("a\nb\n", "notes.txt");
    cmd()
        .args([
            path.to_str().unwrap(),
            "--insert",
            "-S",
            "feat",
            "-l",
            "1:2",
            "--comment-style",
            ";;",
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(
        result,
        ";; toggle:start ID=feat\na\nb\n;; toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_dry_run_does_not_write() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--insert", "-S", "feat", "-l", "1:2", "--dry-run"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "a\nb\n"); // unchanged
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --test integration test_insert_`
Expected: PASS — `--to-end`, `--comment-style`, and `--dry-run` already flow through `run_insert`/`apply_changes`. If `test_insert_to_end` fails, confirm the `if opts.to_end { end = content.lines().count(); }` line is present in `run_insert`.

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test(cli): cover --insert with --to-end, --comment-style, --dry-run (P05)"
```

---

## Task 5: Docs + PROJECTS.md + full dev cycle

**Files:**
- Modify: `README.md` (add an Insert subsection)
- Modify: `PROJECTS.md` (add P05 entry)

- [ ] **Step 1: Add README section**

In `README.md`, after the "Section markers" section (~line 60), add:

```markdown
## Inserting a section

Wrap an existing line range in marker comments without commenting the body:

```bash
# Wrap lines 10–20 of main.py in an ID=featureX marker pair
toggle --insert -S featureX -l 10:20 main.py

# With a description
toggle --insert -S featureX -l 10:20 --desc "new feature" main.py
```

`--insert` operates on a single file and leaves the body uncommented. Run
`toggle -S featureX main.py` afterward to comment the block.
```

- [ ] **Step 2: Add PROJECTS.md entry (as In Progress)**

Use **this repo's** status legend (top of `PROJECTS.md`): `[x]` Completed, `[-]`
In Progress, `[ ]` Not Started, `[~]` Won't fix. (Note: this is the *inverse* of
the global CLAUDE.md legend — follow the repo file.) Create the entry as In
Progress with unchecked tasks; it is flipped to Completed in Step 4 after
`just dev` passes.

In `PROJECTS.md`, after the P04 block, add:

```markdown
---

## [-] Project P05: `--insert` Marker Insertion (v0.3.0)
**Goal**: Add a `--insert` mode that wraps a single file's `-l start:end` block
in a `toggle:start`/`toggle:end` marker pair, leaving the body uncommented.
See `docs/superpowers/specs/2026-05-29-marker-insert-strip-list-filters-design.md`.

### Tests & Tasks
- [ ] [P05-T01] `core::insert_section` happy path + unit tests
- [ ] [P05-T02] `insert_section` guards (dup ID, bounds, bad id/desc) + unit tests
- [ ] [P05-T03] `--insert` / `--desc` CLI flags, validation, `run_insert` + integration tests
- [ ] [P05-T04] `--to-end`, `--comment-style`, `--dry-run` integration tests
- [ ] [P05-T05] README + PROJECTS.md + `just dev`
```

> P05 ships as its own minor version **v0.3.0**. Version bumping/tagging is
> handled by **release-please** from the Conventional Commit messages in this
> plan (`feat(core):` / `feat(cli):` → minor bump) — there is no manual
> version-bump or `git tag` step in this plan.

- [ ] **Step 3: Run the full dev cycle**

Run: `just dev`
Expected: format clean, `clippy -D warnings` clean, all tests pass, build succeeds.

- [ ] **Step 4: Flip P05 to Completed in PROJECTS.md**

Only after Step 3 passes, change the P05 header `## [-]` → `## [x]` and mark every
`[P05-T0x]` task `[x]`. Do not mark complete before the build is green.

- [ ] **Step 5: Commit**

```bash
git add README.md PROJECTS.md
git commit -m "docs: document --insert and track P05 (P05)"
```

---

## Self-Review

**Spec coverage (P05 section of the design):**
- Mode flag, mutually exclusive with other modes → Task 3 validation. ✓
- Exactly one `-S` + one `-l`, reject multiple ranges → Task 3 validation + `test_insert_requires_single_section`. ✓
- `-l` reuses parser incl. `:+count` and `--to-end` → `run_insert` uses `parse_line_range` + to_end; Task 4 `test_insert_to_end`. ✓
- Single file only; reject `-R`/multiple paths → Task 3 validation + `test_insert_rejects_recursive`. ✓
- Line numbers against original file, inclusive; bottom-up insert → `insert_section` impl + Task 1 tests. ✓
- Comment prefix auto-detected or `--comment-style` → `resolve_comment_style`; Task 4 `test_insert_comment_style_override`. ✓
- Indentation matches start line → Task 1 `test_insert_section_matches_indentation`. ✓
- Body left uncommented → all insert tests assert untouched body. ✓
- Refuse duplicate ID → Task 1/2/3 dup-ID tests. ✓
- Range/ID/desc errors → Task 2 guard tests. ✓
- Round-trips through `--scan` → Task 3 `test_insert_round_trips_through_scan`. ✓
- Reuse dry-run/backup → Task 4 `test_insert_dry_run_does_not_write`. ✓

**Placeholder scan:** none — every step has concrete code/commands.

**Type consistency:** `insert_section(content, id, desc, start, end, comment_prefix)` signature is identical across Tasks 1–4 and the `run_insert` caller. `CommentStyle.single_line` matches the field used in `scan_sections` (`src/core.rs:153`).
