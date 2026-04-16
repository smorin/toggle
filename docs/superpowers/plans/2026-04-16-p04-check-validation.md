# P04: `--check` Validation Mode — Implementation Plan

> Use `superpowers:executing-plans`. Steps use `- [ ]` checkboxes.

**Goal:** Per PRD §0.14.3, add `--scan --check` — read-only validation that reports:
- Unclosed `toggle:start` markers
- `pair`-inferred groups with ≠2 variants (warning)
- Variant sets inconsistent across files referencing the same group (warning)
- Duplicate section IDs within a single file (error)

When combined with `--pair`, only check groups that should be pairs (i.e. exclude solos and 3+ groups from the pair-mismatch warning).

**Architecture:** New `core::CheckIssue { level, group, file, message }` and `core::validate_sections(per_file: &[(PathBuf, Vec<ScanSectionInfo>)], pair_only: bool) -> Vec<CheckIssue>`. Emitted from a new `print_check_results` (or `build_check_json`) in `src/main.rs`. Exit code: `0` if no errors (warnings allowed), `EC03` if any error.

**Depends on:** P03 (uses `ScanSectionInfo.group`/`variant`, and `summarize_scan` for type inference).

---

## Task 1: Add `CheckIssue` + `validate_sections`

- [ ] **Step 1: Failing unit tests**

Append to `tests/unit/core_tests.rs`:

```rust
use std::path::PathBuf;

fn scan_one(path: &str, content: &str) -> (PathBuf, Vec<toggle::core::ScanSectionInfo>) {
    let p = PathBuf::from(path);
    let v = toggle::core::scan_sections(&p, content);
    (p, v)
}

#[test]
fn validate_flags_unclosed_marker() {
    let (p, mut v) = scan_one("a.py", "# toggle:start ID=foo\nx = 1\n");
    // scan_sections currently records unclosed sections with end_line = None
    let _ = v.first().unwrap(); // ensure populated
    let issues = toggle::core::validate_sections(&[(p, v)], false);
    assert!(issues.iter().any(|i| i.message.contains("unclosed")));
}

#[test]
fn validate_flags_pair_mismatch_when_pair_inferred() {
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
    let (p, v) = scan_one("a.py", three);
    let issues = toggle::core::validate_sections(&[(p, v)], true);
    // pair_only=true → 3-variant group fails the pair check.
    assert!(issues.iter().any(|i| i.group == "cache" && i.message.contains("expected 2")));
}

#[test]
fn validate_flags_duplicate_id_in_file() {
    let dup = r#"
# toggle:start ID=foo
x = 1
# toggle:end ID=foo

# toggle:start ID=foo
y = 2
# toggle:end ID=foo
"#;
    let (p, v) = scan_one("a.py", dup);
    let issues = toggle::core::validate_sections(&[(p, v)], false);
    assert!(issues.iter().any(|i| i.message.contains("duplicate")));
}

#[test]
fn validate_flags_cross_file_variant_mismatch() {
    let a = r#"
# toggle:start ID=db:sqlite
x = 1
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# y = 2
# toggle:end ID=db:postgres
"#;
    let b = r#"
# toggle:start ID=db:sqlite
z = 3
# toggle:end ID=db:sqlite
"#;
    let issues = toggle::core::validate_sections(
        &[scan_one("a.py", a), scan_one("b.py", b)],
        false,
    );
    assert!(
        issues.iter().any(|i| i.group == "db" && i.message.contains("missing")),
        "issues: {issues:?}"
    );
}
```

- [ ] **Step 2: Verify fail**

Run: `cargo test --test unit validate_`
Expected: compile error — types/functions missing.

- [ ] **Step 3: Implement**

Append to `src/core.rs`:

```rust
/// Severity for a `--check` finding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    Ok,
    Warn,
    Err,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckIssue {
    pub level: CheckLevel,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub message: String,
}

/// Run validation on already-scanned sections grouped by file.
/// `pair_only = true` skips the pair-mismatch check on solos and 3+ groups
/// (i.e. when invoked with `--check --pair`).
pub fn validate_sections(
    per_file: &[(std::path::PathBuf, Vec<ScanSectionInfo>)],
    pair_only: bool,
) -> Vec<CheckIssue> {
    let mut issues = Vec::new();

    // 1. Unclosed markers + duplicate IDs (per-file)
    for (path, sections) in per_file {
        for s in sections {
            if s.end_line.is_none() {
                issues.push(CheckIssue {
                    level: CheckLevel::Err,
                    group: s.group.clone(),
                    file: Some(path.display().to_string()),
                    message: format!("unclosed marker for ID={}", s.id),
                });
            }
        }
        let mut counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for s in sections {
            *counts.entry(s.id.as_str()).or_insert(0) += 1;
        }
        for (id, n) in counts {
            if n > 1 {
                issues.push(CheckIssue {
                    level: CheckLevel::Err,
                    group: parse_id_parts(id).0,
                    file: Some(path.display().to_string()),
                    message: format!("duplicate section ID '{id}' ({n} occurrences)"),
                });
            }
        }
    }

    // 2. Group-level checks across files
    let flat: Vec<ScanSectionInfo> =
        per_file.iter().flat_map(|(_, v)| v.clone()).collect();
    let summaries = summarize_scan(&flat);

    for sum in &summaries {
        // Pair-mismatch
        let group_is_pair_like = !matches!(sum.section_type, SectionType::Solo);
        if pair_only && group_is_pair_like && sum.variant_count != 2 {
            issues.push(CheckIssue {
                level: CheckLevel::Warn,
                group: sum.group.clone(),
                file: None,
                message: format!(
                    "{} variants, expected 2 (pair check)",
                    sum.variant_count
                ),
            });
        }

        // Cross-file variant mismatch
        if matches!(sum.section_type, SectionType::Pair | SectionType::Group) {
            for (path, sections) in per_file {
                let present: std::collections::BTreeSet<String> = sections
                    .iter()
                    .filter(|s| s.group == sum.group)
                    .filter_map(|s| s.variant.clone())
                    .collect();
                if present.is_empty() {
                    continue; // file does not reference this group at all
                }
                let expected: std::collections::BTreeSet<String> =
                    sum.variants.iter().cloned().collect();
                let missing: Vec<&String> = expected.difference(&present).collect();
                if !missing.is_empty() {
                    issues.push(CheckIssue {
                        level: CheckLevel::Warn,
                        group: sum.group.clone(),
                        file: Some(path.display().to_string()),
                        message: format!(
                            "missing variant(s): {}",
                            missing
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }
        }
    }

    issues
}
```

- [ ] **Step 4: Verify pass**

Run: `cargo test --test unit validate_`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): validate_sections for --check"
```

---

## Task 2: Add `--check` CLI flag

- [ ] **Step 1: Add to `src/cli.rs`** (after `--scan`):

```rust
    /// Validate section integrity without modifying files. Requires --scan.
    #[arg(long = "check")]
    pub check: bool,
```

- [ ] **Step 2: Validate flag combinations**

In `src/main.rs::main` (or wherever `--scan` validation lives), add:

```rust
    if cli.check && !cli.scan {
        return Err(UsageError("--check requires --scan".into()).into());
    }
```

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat(cli): add --check flag (requires --scan)"
```

---

## Task 3: Wire `--check` output

- [ ] **Step 1: Failing integration tests**

Append to `tests/integration.rs`:

```rust
#[test]
fn check_reports_pair_mismatch_with_pair_flag() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args([
            "--scan",
            "--check",
            "--pair",
            "tests/fixtures/variants.py",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cache"), "stdout: {stdout}");
    assert!(stdout.contains("WARN") || stdout.contains("ERR"));
}

#[test]
fn check_clean_scan_reports_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("clean.py");
    std::fs::write(
        &dst,
        "# toggle:start ID=foo\nx = 1\n# toggle:end ID=foo\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--scan", "--check", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK") || stdout.contains("foo"));
}
```

- [ ] **Step 2: Add print + JSON helpers**

In `src/main.rs`:

```rust
fn print_check_results(issues: &[core::CheckIssue]) {
    if issues.is_empty() {
        println!("OK    no issues found");
        return;
    }
    for i in issues {
        let tag = match i.level {
            core::CheckLevel::Ok => "OK  ",
            core::CheckLevel::Warn => "WARN",
            core::CheckLevel::Err => "ERR ",
        };
        let file_part = i.file.as_deref().map(|f| format!(" ({f})")).unwrap_or_default();
        println!("{tag}  {:<18} {}{file_part}", i.group, i.message);
    }
}

fn check_has_errors(issues: &[core::CheckIssue]) -> bool {
    issues.iter().any(|i| matches!(i.level, core::CheckLevel::Err))
}
```

- [ ] **Step 3: Branch in `run_scan`**

Add a `--check` branch ahead of the other scan output branches in `src/main.rs::run_scan`:

```rust
    if cli.check {
        // Re-bucket the scanned sections by file
        let mut per_file: std::collections::BTreeMap<std::path::PathBuf, Vec<core::ScanSectionInfo>> =
            std::collections::BTreeMap::new();
        for s in &all_sections {
            per_file
                .entry(std::path::PathBuf::from(&s.file))
                .or_default()
                .push(s.clone());
        }
        let per_file_vec: Vec<_> = per_file.into_iter().collect();
        let issues = core::validate_sections(&per_file_vec, cli.pair);

        if cli.json {
            println!("{}", serde_json::to_string_pretty(&issues).expect("JSON"));
        } else {
            print_check_results(&issues);
        }

        if check_has_errors(&issues) {
            std::process::exit(crate::ExitCode::Logic as i32);
        }
        return Ok(());
    }
```

> If `ExitCode::Logic` doesn't exist under that exact name, replace with the existing `EC03` mapping (search `EC03` / `Logic` in `src/exit_codes.rs`).

- [ ] **Step 4: Run tests**

Run: `cargo test --test integration check_`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs tests/integration.rs
git commit -m "feat(scan): wire --check output and exit codes"
```

---

## Task 4: Final dev cycle

- [ ] **Step 1: Run**

Run: `just dev`
Expected: clean.

- [ ] **Step 2: Commit any cleanup**

```bash
git status
# if dirty:
git add -p
git commit -m "chore: cleanup after --check"
```
