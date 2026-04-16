# P01: Section Variants Core — Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `group:variant` section IDs per PRD §0.13: parse them, discover them by group, and add the four CLI behaviors (solo / pair-flip / activate / force-all). Per-file scope only — cross-file consistency lives in P04.

**Architecture:** Extend `src/core.rs` with three pure helpers (`parse_id_parts`, `discover_variants`, `toggle_variant_group`/`activate_variant`) — no I/O — and route to them from `src/main.rs::toggle_section` based on whether the `-S` argument contains a colon and how many variants exist.

**Tech Stack:** Rust 2021, clap 4.4, anyhow, serde, existing `core::find_and_toggle_section` for the per-block toggle primitive.

---

## File Structure

| File | Responsibility | Change Type |
|---|---|---|
| `src/core.rs` | New variant helpers + new `VariantError` mapping | Modify |
| `src/main.rs` | Route `-S` arguments through variant logic in `toggle_section` (line 694) and `compute_section_changes` (line 519) | Modify |
| `src/cli.rs` | Update `-S` doc comment to mention `group:variant` syntax | Modify |
| `tests/unit/core_tests.rs` | Unit tests for the three new helpers | Modify |
| `tests/integration.rs` | End-to-end CLI tests against the new fixture | Modify |
| `tests/fixtures/variants.py` | New fixture with `db:sqlite` + `db:postgres` (pair) and `cache:redis` + `cache:memcached` + `cache:inmemory` (3-variant) | Create |
| `Cargo.toml` | Bump version `0.1.0` → `0.2.0` | Modify |

**Design notes:**
- `parse_id_parts("db:postgres") -> ("db", Some("postgres"))`; `parse_id_parts("debug") -> ("debug", None)`.
- A *group* is the prefix before `:`. A solo section has the same group as its id and no variant.
- `discover_variants(content, "db")` returns every `SectionInfo` whose `parse_id_parts(.id).0 == "db"`. So `-S db` matching a literal section id `db` AND `db:sqlite` AND `db:postgres` produces 3 items — error per PRD.
- `find_and_toggle_section` is reused per-variant; we never re-implement the line-by-line toggle.

---

## Task 1: Add `parse_id_parts` to `core.rs`

**Files:**
- Modify: `src/core.rs` (add helper near `parse_section_id` at lines 53-64)
- Test: `tests/unit/core_tests.rs`

- [ ] **Step 1: Write failing test**

Append to `tests/unit/core_tests.rs`:

```rust
#[test]
fn parse_id_parts_solo() {
    assert_eq!(toggle::core::parse_id_parts("debug"), ("debug".to_string(), None));
}

#[test]
fn parse_id_parts_variant() {
    assert_eq!(
        toggle::core::parse_id_parts("db:postgres"),
        ("db".to_string(), Some("postgres".to_string()))
    );
}

#[test]
fn parse_id_parts_empty_variant_treated_as_solo() {
    // "db:" — colon with empty variant. Treat as solo group "db:" rather than crash.
    let (g, v) = toggle::core::parse_id_parts("db:");
    assert_eq!(g, "db");
    assert_eq!(v, Some("".to_string()));
}

#[test]
fn parse_id_parts_multiple_colons_uses_first() {
    // "a:b:c" — group "a", variant "b:c". First colon wins.
    assert_eq!(
        toggle::core::parse_id_parts("a:b:c"),
        ("a".to_string(), Some("b:c".to_string()))
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test unit parse_id_parts -- --nocapture`
Expected: FAIL — `parse_id_parts` not found in `toggle::core`.

- [ ] **Step 3: Add the function**

Add to `src/core.rs` immediately after the existing `parse_section_id` function (line 64):

```rust
/// Split a section ID into `(group, variant)` parts using the first `:` as separator.
/// Solo IDs (no colon) return `(id, None)`; variant IDs return `(group, Some(variant))`.
pub fn parse_id_parts(id: &str) -> (String, Option<String>) {
    match id.split_once(':') {
        Some((g, v)) => (g.to_string(), Some(v.to_string())),
        None => (id.to_string(), None),
    }
}
```

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test unit parse_id_parts`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): add parse_id_parts helper for group:variant IDs"
```

---

## Task 2: Add `discover_variants` to `core.rs`

**Files:**
- Modify: `src/core.rs` (add helper after `discover_sections` ending around line 121)
- Test: `tests/unit/core_tests.rs`

- [ ] **Step 1: Write failing test**

Append to `tests/unit/core_tests.rs`:

```rust
const VARIANTS_FIXTURE: &str = r#"
# toggle:start ID=db:sqlite desc="SQLite backend"
import sqlite3
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres desc="Postgres backend"
# import psycopg2
# toggle:end ID=db:postgres

# toggle:start ID=debug
print("debug")
# toggle:end ID=debug
"#;

#[test]
fn discover_variants_returns_pair() {
    let v = toggle::core::discover_variants(VARIANTS_FIXTURE, "db");
    assert_eq!(v.len(), 2);
    let ids: Vec<&str> = v.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"db:sqlite"));
    assert!(ids.contains(&"db:postgres"));
}

#[test]
fn discover_variants_solo_only() {
    let v = toggle::core::discover_variants(VARIANTS_FIXTURE, "debug");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].id, "debug");
}

#[test]
fn discover_variants_no_match() {
    let v = toggle::core::discover_variants(VARIANTS_FIXTURE, "missing");
    assert!(v.is_empty());
}

#[test]
fn discover_variants_distinguishes_groups() {
    // "db" must NOT match "debug" (prefix collision guard).
    let v = toggle::core::discover_variants(VARIANTS_FIXTURE, "db");
    for s in &v {
        let (g, _) = toggle::core::parse_id_parts(&s.id);
        assert_eq!(g, "db");
    }
}
```

- [ ] **Step 2: Verify fail**

Run: `cargo test --test unit discover_variants`
Expected: FAIL — `discover_variants` not defined.

- [ ] **Step 3: Implement**

Add to `src/core.rs` after `discover_sections` (after line 121):

```rust
/// Return all `SectionInfo` whose ID parses into the given group.
/// `discover_variants(content, "db")` matches both `db` (solo) and `db:postgres` (variant).
pub fn discover_variants(content: &str, group: &str) -> Vec<SectionInfo> {
    discover_sections(content)
        .into_iter()
        .filter(|s| parse_id_parts(&s.id).0 == group)
        .collect()
}
```

- [ ] **Step 4: Verify pass**

Run: `cargo test --test unit discover_variants`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): add discover_variants for group-prefix matching"
```

---

## Task 3: Add the variant toggle engine

**Files:**
- Modify: `src/core.rs` (add `toggle_variant_group` + `activate_variant`)
- Test: `tests/unit/core_tests.rs`

The PRD §0.13.3 behaviors handled here:
- pair-flip (group, no variant qualifier, exactly 2 found) — for each variant, invert its current state
- activate (`-S group:variant`) — uncomment the named variant, comment all other variants of the group
- force-all (`-S group --force on/off`) — apply the force to every variant of the group
- error (group, no variant qualifier, 3+ found)

- [ ] **Step 1: Write failing tests**

Append to `tests/unit/core_tests.rs`:

```rust
fn comment_style_py() -> toggle::core::CommentStyle {
    toggle::core::CommentStyle {
        single_line: "#".to_string(),
        multi_line_start: None,
        multi_line_end: None,
    }
}

#[test]
fn toggle_variant_group_pair_flip_swaps_states() {
    // Initial: db:sqlite uncommented, db:postgres commented (per fixture).
    let result = toggle::core::toggle_variant_group(
        VARIANTS_FIXTURE,
        "db",
        &None,
        &comment_style_py(),
    )
    .unwrap();
    // After flip: db:sqlite commented, db:postgres uncommented.
    assert!(result.contains("# import sqlite3") || result.contains("#import sqlite3"));
    assert!(result.contains("\nimport psycopg2"));
}

#[test]
fn toggle_variant_group_force_on_comments_all() {
    let result = toggle::core::toggle_variant_group(
        VARIANTS_FIXTURE,
        "db",
        &Some("on".to_string()),
        &comment_style_py(),
    )
    .unwrap();
    assert!(result.contains("# import sqlite3") || result.contains("#import sqlite3"));
    assert!(result.contains("# import psycopg2") || result.contains("#import psycopg2"));
}

#[test]
fn toggle_variant_group_force_off_uncomments_all() {
    let result = toggle::core::toggle_variant_group(
        VARIANTS_FIXTURE,
        "db",
        &Some("off".to_string()),
        &comment_style_py(),
    )
    .unwrap();
    assert!(result.contains("\nimport sqlite3"));
    assert!(result.contains("\nimport psycopg2"));
}

#[test]
fn toggle_variant_group_errors_on_three_variants() {
    let three = r#"
# toggle:start ID=cache:redis
x = 1
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# y = 2
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# z = 3
# toggle:end ID=cache:inmemory
"#;
    let err = toggle::core::toggle_variant_group(three, "cache", &None, &comment_style_py())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("3 variants"), "got: {msg}");
    assert!(msg.contains("cache"));
}

#[test]
fn activate_variant_uncomments_target_and_comments_others() {
    let result = toggle::core::activate_variant(
        VARIANTS_FIXTURE,
        "db",
        "postgres",
        &comment_style_py(),
    )
    .unwrap();
    assert!(result.contains("\nimport psycopg2"));
    assert!(result.contains("# import sqlite3") || result.contains("#import sqlite3"));
}

#[test]
fn activate_variant_unknown_variant_errors() {
    let err = toggle::core::activate_variant(
        VARIANTS_FIXTURE,
        "db",
        "mysql",
        &comment_style_py(),
    )
    .unwrap_err();
    assert!(format!("{err}").contains("mysql"));
}
```

- [ ] **Step 2: Verify fail**

Run: `cargo test --test unit toggle_variant_group activate_variant`
Expected: 6 failing tests — symbols not defined.

- [ ] **Step 3: Implement**

Add to `src/core.rs` after `find_and_toggle_section` (after line 691):

```rust
/// Toggle every variant of `group` in `content`.
/// - `force = None` and exactly 2 variants → pair-flip (each variant inverted).
/// - `force = None` and 1 variant → solo invert.
/// - `force = None` and 3+ variants → error per PRD §0.13.3.
/// - `force = Some("on" | "off")` → apply force to every variant regardless of count.
pub fn toggle_variant_group(
    content: &str,
    group: &str,
    force: &Option<String>,
    comment_style: &CommentStyle,
) -> Result<String> {
    let variants = discover_variants(content, group);
    if variants.is_empty() {
        return Err(UsageError(format!("no section or group '{group}' found")).into());
    }
    if force.is_none() && variants.len() >= 3 {
        return Err(UsageError(format!(
            "group '{group}' has {} variants; specify one with -S {group}:<name>",
            variants.len()
        ))
        .into());
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    for v in &variants {
        find_and_toggle_section(&mut lines, &v.id, force, comment_style)?;
    }

    let mut joined = lines.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}

/// Activate `group:variant`: uncomment that variant, comment every other variant of the group.
pub fn activate_variant(
    content: &str,
    group: &str,
    variant: &str,
    comment_style: &CommentStyle,
) -> Result<String> {
    let target_id = format!("{group}:{variant}");
    let variants = discover_variants(content, group);
    if !variants.iter().any(|s| s.id == target_id) {
        return Err(UsageError(format!("variant '{target_id}' not found")).into());
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    for v in &variants {
        let force = if v.id == target_id {
            Some("off".to_string())
        } else {
            Some("on".to_string())
        };
        find_and_toggle_section(&mut lines, &v.id, &force, comment_style)?;
    }

    let mut joined = lines.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}
```

- [ ] **Step 4: Verify pass**

Run: `cargo test --test unit toggle_variant_group activate_variant`
Expected: 6 tests pass. If pair-flip fails because the assertions are too strict about exact spacing, relax them only as needed (`# import sqlite3` vs `#import sqlite3`).

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): add variant toggle and activate engines"
```

---

## Task 4: Create the integration fixture

**Files:**
- Create: `tests/fixtures/variants.py`

- [ ] **Step 1: Write the fixture**

Create `tests/fixtures/variants.py` with this exact content:

```python
"""Fixture for variant section tests."""

# toggle:start ID=db:sqlite desc="SQLite backend"
import sqlite3
conn = sqlite3.connect("app.db")
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres desc="Postgres backend"
# import psycopg2
# conn = psycopg2.connect("host=localhost")
# toggle:end ID=db:postgres

# toggle:start ID=debug
print("debug enabled")
# toggle:end ID=debug

# toggle:start ID=cache:redis
import redis
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# import memcache
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# cache = {}
# toggle:end ID=cache:inmemory
```

- [ ] **Step 2: Commit**

```bash
git add tests/fixtures/variants.py
git commit -m "test: add variants.py fixture for integration tests"
```

---

## Task 5: Route variant CLI args in `main.rs`

**Files:**
- Modify: `src/main.rs::compute_section_changes` (lines 519-539) and `src/main.rs::toggle_section` (search for `fn toggle_section`)

The current code blindly calls `core::find_and_toggle_section(lines, section_id, ...)` for every `-S` argument. Replace this so each `-S X` argument is dispatched as:
- contains `:` → `activate_variant`
- otherwise, count variants in the file: 1 → existing solo path; 2 → `toggle_variant_group`; 3+ → `toggle_variant_group` returns the error

- [ ] **Step 1: Find both call sites**

Run: `grep -n "find_and_toggle_section" src/main.rs`
Expected: two hits — one in `compute_section_changes` (~line 528) and one in `toggle_section` (~line 883).

- [ ] **Step 2: Replace `compute_section_changes` body**

Replace the body of `compute_section_changes` in `src/main.rs` (the function spanning roughly lines 519-539):

```rust
fn compute_section_changes(
    path: &Path,
    section_id: &str,
    opts: &ToggleOptions,
    content: &str,
) -> Result<String> {
    let comment_style = resolve_comment_style(path, opts)?;
    let (group, variant) = core::parse_id_parts(section_id);

    let toggled = match variant {
        Some(v) => core::activate_variant(content, &group, &v, &comment_style)?,
        None => {
            let variants = core::discover_variants(content, &group);
            if variants.len() <= 1 && opts.force.is_none() {
                // Solo path: preserve existing per-section behavior + early exit when no marker.
                let mut lines: Vec<String> = content.lines().map(String::from).collect();
                let result = core::find_and_toggle_section(
                    &mut lines, section_id, opts.force, &comment_style,
                )?;
                if !result.modified {
                    return Ok(content.to_string());
                }
                let mut joined = lines.join("\n");
                if content.ends_with('\n') {
                    joined.push('\n');
                }
                joined
            } else {
                core::toggle_variant_group(content, &group, opts.force, &comment_style)?
            }
        }
    };

    Ok(io::normalize_eol(&toggled, opts.eol))
}
```

- [ ] **Step 3: Update `toggle_section` (the writing path)**

Locate the existing `toggle_section` function (around line 850-900). It reads the file, calls `find_and_toggle_section`, then writes via `apply_changes`. Refactor it to call `compute_section_changes` so the routing logic exists in exactly one place:

```rust
fn toggle_section(path: &Path, section_id: &str, opts: &ToggleOptions) -> Result<ProcessResult> {
    let content = io::read_file_encoded(path, opts.encoding)?;
    let modified = compute_section_changes(path, section_id, opts, &content)?;

    let lines_changed = if modified == content {
        0
    } else {
        apply_changes(path, &content, &modified, opts)?
    };

    let action = match opts.force.as_deref() {
        Some("on") => "comment",
        Some("off") => "uncomment",
        _ => "invert",
    }
    .to_string();

    // Preserve the desc lookup that the previous implementation did via SectionToggleResult.
    // For variant operations we surface the first matching variant's desc.
    let desc = core::discover_variants(&content, &core::parse_id_parts(section_id).0)
        .into_iter()
        .find(|s| s.id == section_id || core::parse_id_parts(&s.id).0 == section_id)
        .and_then(|s| s.desc);

    Ok(ProcessResult {
        action,
        lines_changed,
        section_id: Some(section_id.to_string()),
        desc,
    })
}
```

> If `toggle_section` had additional verbose-logging or skip-if-no-section logic, copy the surrounding lines verbatim and only swap the `find_and_toggle_section` block. Do not delete unrelated code.

- [ ] **Step 4: Verify build**

Run: `cargo build`
Expected: success. Fix any unused-import warnings the editor flags (`SectionToggleResult` may become unused — drop it from the `use` line if so).

- [ ] **Step 5: Run existing tests**

Run: `cargo test`
Expected: all pre-existing tests still pass. If a solo-section test fails, investigate — the solo path in Step 2 may need to mirror previous semantics more closely.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): route -S arguments through variant logic"
```

---

## Task 6: Integration tests against the fixture

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Add tests**

Append to `tests/integration.rs`. The pattern to copy from existing tests is "create a temp dir, copy the fixture in, run the binary, assert content".

```rust
#[test]
fn variant_pair_flip() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "db", dst.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    let content = std::fs::read_to_string(&dst).unwrap();
    // sqlite block now commented, postgres now uncommented
    assert!(content.contains("# import sqlite3"));
    assert!(content.contains("\nimport psycopg2"));
}

#[test]
fn variant_activate_named() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "db:postgres", dst.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    let content = std::fs::read_to_string(&dst).unwrap();
    assert!(content.contains("\nimport psycopg2"));
    assert!(content.contains("# import sqlite3"));
}

#[test]
fn variant_three_errors_without_qualifier() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "cache", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cache"), "stderr: {stderr}");
    assert!(stderr.contains("3 variants"), "stderr: {stderr}");
}

#[test]
fn variant_force_on_all() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "cache", "--force", "on", dst.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    let content = std::fs::read_to_string(&dst).unwrap();
    // All three cache variants now commented (redis was uncommented before)
    assert!(content.contains("# import redis"));
}
```

- [ ] **Step 2: Run**

Run: `cargo test --test integration variant_`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: integration coverage for variant CLI behaviors"
```

---

## Task 7: Update CLI help text

**Files:**
- Modify: `src/cli.rs:18` (the `-S, --section` doc comment)

- [ ] **Step 1: Update doc comment**

In `src/cli.rs`, replace lines 17-19:

```rust
    /// Section ID to toggle. Use `group:variant` (e.g. `db:postgres`) for
    /// variant operations: `-S group` flips a 2-variant pair, `-S group:variant`
    /// activates one variant and comments siblings.
    #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
    pub sections: Vec<String>,
```

- [ ] **Step 2: Verify help renders**

Run: `cargo run -- --help 2>&1 | grep -A2 "section"`
Expected: the new help text appears.

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "docs(cli): document -S group:variant syntax"
```

---

## Task 8: Final verification + version bump

- [ ] **Step 1: Run full dev cycle**

Run: `just dev`
Expected: format, lint, test, build all pass clean.

If lint flags `dead_code` on `SectionToggleResult` because Task 5 stopped using it from `main.rs`, either re-export it where unit tests need it or delete the struct (verify nothing else imports it first with `grep -rn SectionToggleResult src/ tests/`).

- [ ] **Step 2: Bump version**

Edit `Cargo.toml:3` from `version = "0.1.0"` to `version = "0.2.0"`.

- [ ] **Step 3: Update PROJECTS.md**

Mark all P01 tasks complete: change `[ ]` → `[x]` for P01-T01 through P01-T08, and the project header `[ ] Project P01` → `[x] Project P01`.

- [ ] **Step 4: Commit and tag**

```bash
git add Cargo.toml Cargo.lock PROJECTS.md
git commit -m "release: v0.2.0 — section variants core"
git tag v0.2.0
```

(Push and PR creation are deliberately not in this plan — confirm with the user before pushing.)

---

## Self-Review Checklist

- **Spec coverage (PRD §0.13):** §0.13.2 marker syntax ✓ Task 1. §0.13.3 CLI behavior table — solo ✓ existing path, pair-flip ✓ Task 3+5, activate ✓ Task 3+5, force-all ✓ Task 3+5, error on 3+ ✓ Task 3. §0.13.4 `--pair` — deferred to P02 (out of scope). §0.13.5 multi-file — deferred (per-file scope only); `-R` already handled by `walk::collect_files` invoking `process_file` per path.
- **Placeholder scan:** No "TBD" / "implement later" / "handle edge cases" without concrete tests. Every code step shows the code.
- **Type consistency:** `parse_id_parts` returns `(String, Option<String>)` everywhere it appears. `discover_variants` returns `Vec<SectionInfo>` (existing type, no fields added). `toggle_variant_group` and `activate_variant` both return `Result<String>` and take `&CommentStyle`.
