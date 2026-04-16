# P02: `--pair` Validation Flag — Implementation Plan

> Use `superpowers:executing-plans`. Steps use `- [ ]` checkboxes.

**Goal:** Implement the `--pair` flag from PRD §0.13.4 — a pre-execution guard that errors if any targeted group does not contain exactly 2 variants. No file modifications occur on failure.

**Architecture:** Pure CLI flag. Validation runs before the per-file processing loop in `src/main.rs::main` (or wherever the toggle pipeline starts). For each `-S group` argument, scan the union of all input files; if `discover_variants(content, group).len() != 2` for any file, error and exit with `EC01`.

**Depends on:** P01 (uses `core::discover_variants` and `core::parse_id_parts`).

---

## Task 1: Add `--pair` to the CLI

- [ ] **Step 1: Add the flag**

In `src/cli.rs`, append after the `--scan` flag (around line 91):

```rust
    /// Enforce exactly 2 variants in the targeted group; error otherwise.
    /// Pre-execution check — no file modifications occur on failure.
    #[arg(long = "pair")]
    pub pair: bool,
```

- [ ] **Step 2: Verify it parses**

Run: `cargo run -- --help 2>&1 | grep -i pair`
Expected: the flag appears in help.

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add --pair flag stub"
```

---

## Task 2: Add validation logic

- [ ] **Step 1: Write failing integration tests**

Append to `tests/integration.rs`:

```rust
#[test]
fn pair_succeeds_on_two_variant_group() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "db", "--pair", dst.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn pair_errors_on_three_variant_group() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();
    let before = std::fs::read_to_string(&dst).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "cache", "--pair", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--pair"), "stderr: {stderr}");
    assert!(stderr.contains("3"), "stderr: {stderr}");

    // File must be untouched
    let after = std::fs::read_to_string(&dst).unwrap();
    assert_eq!(before, after, "file modified despite --pair failure");
}

#[test]
fn pair_errors_on_one_variant_group() {
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("variants.py");
    std::fs::copy("tests/fixtures/variants.py", &dst).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["-S", "debug", "--pair", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--pair"));
}
```

- [ ] **Step 2: Verify they fail**

Run: `cargo test --test integration pair_`
Expected: 3 failing tests (flag is parsed but no validation yet, so they all run as if --pair were absent).

- [ ] **Step 3: Implement the validation**

In `src/main.rs`, add a helper near the other validation helpers:

```rust
/// Validate that every -S group targets a group with exactly 2 variants in every input file.
/// Called before any file mutation when --pair is set.
fn validate_pair_groups(cli: &Cli) -> Result<()> {
    let walk_opts = walk::WalkOptions {
        verbose: cli.verbose,
        ..walk::WalkOptions::default()
    };
    let files = walk::collect_files(&cli.paths, cli.recursive, &walk_opts)?;

    for section in &cli.sections {
        let (group, _variant) = core::parse_id_parts(section);
        for file in &files {
            let content = match io::read_file_encoded(file, &cli.encoding) {
                Ok(c) => c,
                Err(_) => continue, // unreadable files reported elsewhere
            };
            let count = core::discover_variants(&content, &group).len();
            if count != 2 {
                return Err(UsageError(format!(
                    "--pair: group '{group}' has {count} variants in {}, expected exactly 2",
                    file.display()
                ))
                .into());
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Wire the call**

In `src/main.rs::main`, after CLI parse and config resolution but **before** the file-processing loop, add:

```rust
    if cli.pair {
        if cli.sections.is_empty() {
            return Err(UsageError("--pair requires at least one -S <group>".into()).into());
        }
        validate_pair_groups(&cli)?;
    }
```

(Insert after the existing argument-conflict checks; if `main` returns `Result<()>` via a wrapper, place inside that wrapper. Search for `cli.list_sections` to find the right region.)

- [ ] **Step 5: Run tests**

Run: `cargo test --test integration pair_ && cargo test`
Expected: P02 integration tests pass; no regressions.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs tests/integration.rs
git commit -m "feat(cli): --pair pre-execution variant-count guard"
```

---

## Task 3: Help text + dev cycle

- [ ] **Step 1: Add unit test ensuring `--pair` rejected without `-S`**

Append to `tests/integration.rs`:

```rust
#[test]
fn pair_without_section_errors() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--pair", "tests/fixtures/variants.py"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
```

- [ ] **Step 2: Run tests + lint**

Run: `cargo test && just lint`
Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: --pair without -S exits non-zero"
```
