# P03: Variant-Aware Scan Output — Implementation Plan

> Use `superpowers:executing-plans`. Steps use `- [ ]` checkboxes.

**Goal:** Per PRD §0.14, extend `--scan` so that:
- Per-file table output groups variants under their group (`TYPE` column = `solo` / `pair` / `group`).
- A recursive summary mode (`--scan -R`) aggregates files-per-group and variant counts.
- `--scan -S <id>` produces a detailed view of one section/group.
- `--json` emits the nested schema in §0.14.4.

**Architecture:** Extend `core::ScanSectionInfo` with `group: String` and `variant: Option<String>` (parsed once at scan time). Add a new `core::SectionType` enum and a `core::summarize_scan(sections: &[ScanSectionInfo]) -> Vec<GroupSummary>` post-processor. Replace `print_scan_results` in `src/main.rs` with three branches (default per-file, recursive summary when `-R`, detailed when `-S`).

**Depends on:** P01 (uses `core::parse_id_parts`).

---

## Task 1: Extend `ScanSectionInfo` with parsed group/variant

- [ ] **Step 1: Failing unit tests**

Append to `tests/unit/core_tests.rs`:

```rust
#[test]
fn scan_sections_populates_group_and_variant() {
    use std::path::Path;
    let content = r#"
# toggle:start ID=db:sqlite
import sqlite3
# toggle:end ID=db:sqlite

# toggle:start ID=debug
print("x")
# toggle:end ID=debug
"#;
    let sections = toggle::core::scan_sections(Path::new("test.py"), content);
    let sqlite = sections.iter().find(|s| s.id == "db:sqlite").unwrap();
    assert_eq!(sqlite.group, "db");
    assert_eq!(sqlite.variant.as_deref(), Some("sqlite"));

    let debug = sections.iter().find(|s| s.id == "debug").unwrap();
    assert_eq!(debug.group, "debug");
    assert_eq!(debug.variant, None);
}
```

- [ ] **Step 2: Verify fail**

Run: `cargo test --test unit scan_sections_populates_group_and_variant`
Expected: compile error — `group` / `variant` fields missing.

- [ ] **Step 3: Add fields**

In `src/core.rs`, modify the `ScanSectionInfo` struct (around line 33):

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanSectionInfo {
    pub id: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub file: String,
    pub start_line: usize,
    pub end_line: Option<usize>,
    pub description: Option<String>,
    pub state: String,
}
```

In `scan_sections` (around line 168), after computing `id`, derive both fields:

```rust
            let (group, variant) = parse_id_parts(&id);

            sections.push(ScanSectionInfo {
                id,
                group,
                variant,
                file: file_str.clone(),
                start_line,
                end_line,
                description,
                state,
            });
```

- [ ] **Step 4: Verify pass**

Run: `cargo test --test unit scan_sections_populates_group_and_variant`
Expected: pass. Other code that constructs `ScanSectionInfo` directly (search: `grep -rn "ScanSectionInfo {" src/ tests/`) needs the new fields filled in.

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): expose group/variant on ScanSectionInfo"
```

---

## Task 2: Add `SectionType` and `summarize_scan` post-processor

- [ ] **Step 1: Failing unit test**

Append to `tests/unit/core_tests.rs`:

```rust
#[test]
fn summarize_scan_infers_types() {
    use toggle::core::SectionType;
    use std::path::Path;
    let content = r#"
# toggle:start ID=db:sqlite
x = 1
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# y = 2
# toggle:end ID=db:postgres

# toggle:start ID=cache:redis
z = 3
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# a = 4
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# b = 5
# toggle:end ID=cache:inmemory

# toggle:start ID=debug
c = 6
# toggle:end ID=debug
"#;
    let sections = toggle::core::scan_sections(Path::new("t.py"), content);
    let summary = toggle::core::summarize_scan(&sections);

    let by_group = |g: &str| summary.iter().find(|s| s.group == g).unwrap().clone();
    assert_eq!(by_group("db").section_type, SectionType::Pair);
    assert_eq!(by_group("db").variant_count, 2);
    assert_eq!(by_group("cache").section_type, SectionType::Group);
    assert_eq!(by_group("cache").variant_count, 3);
    assert_eq!(by_group("debug").section_type, SectionType::Solo);
}
```

- [ ] **Step 2: Verify fail**

Run: `cargo test --test unit summarize_scan_infers_types`
Expected: compile error.

- [ ] **Step 3: Implement**

Append to `src/core.rs`:

```rust
/// Inferred type of a section group (PRD §0.14.1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionType {
    Solo,
    Pair,
    Group,
}

/// Per-group summary across one or more files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupSummary {
    pub group: String,
    pub section_type: SectionType,
    pub variant_count: usize,
    pub file_count: usize,
    pub state: String,
    pub variants: Vec<String>,
}

/// Group flat scan results into per-group summaries with inferred type.
pub fn summarize_scan(sections: &[ScanSectionInfo]) -> Vec<GroupSummary> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&ScanSectionInfo>> = BTreeMap::new();
    for s in sections {
        groups.entry(s.group.clone()).or_default().push(s);
    }

    groups
        .into_iter()
        .map(|(group, items)| {
            let mut variants: Vec<String> = items
                .iter()
                .filter_map(|s| s.variant.clone())
                .collect();
            variants.sort();
            variants.dedup();

            let section_type = if variants.is_empty() {
                SectionType::Solo
            } else if variants.len() == 2 {
                SectionType::Pair
            } else if variants.len() == 1 {
                SectionType::Solo
            } else {
                SectionType::Group
            };

            let files: std::collections::BTreeSet<&String> =
                items.iter().map(|s| &s.file).collect();

            let states: std::collections::BTreeSet<&String> =
                items.iter().map(|s| &s.state).collect();
            let state = if states.len() == 1 {
                states.into_iter().next().unwrap().clone()
            } else {
                "mixed".to_string()
            };

            GroupSummary {
                group,
                section_type,
                variant_count: variants.len(),
                file_count: files.len(),
                state,
                variants,
            }
        })
        .collect()
}
```

- [ ] **Step 4: Verify pass**

Run: `cargo test --test unit summarize_scan_infers_types`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src/core.rs tests/unit/core_tests.rs
git commit -m "feat(core): add SectionType + summarize_scan grouping"
```

---

## Task 3: Per-file table — group variants under their parent

- [ ] **Step 1: Update `print_scan_results` in `src/main.rs`**

Replace the function body (around lines 956-978) with one that:
- Sorts entries by group, then puts solo entries directly and variant entries indented under a group header line.
- Adds the `TYPE` column.

```rust
fn print_scan_results(sections: &[core::ScanSectionInfo]) {
    if sections.is_empty() {
        println!("No toggle sections found.");
        return;
    }

    println!(
        "{:<18} {:<7} {:<11} {:<10} {}",
        "SECTION", "TYPE", "STATE", "LINES", "DESCRIPTION"
    );
    println!("{}", "\u{2500}".repeat(80));

    let summaries = core::summarize_scan(sections);
    for summary in &summaries {
        // Pull this group's sections in start_line order.
        let mut items: Vec<&core::ScanSectionInfo> =
            sections.iter().filter(|s| s.group == summary.group).collect();
        items.sort_by_key(|s| s.start_line);

        for s in items {
            let lines = match s.end_line {
                Some(e) => format!("{}-{}", s.start_line, e),
                None => format!("{}-?", s.start_line),
            };
            let type_label = match summary.section_type {
                core::SectionType::Solo => "solo",
                core::SectionType::Pair => "pair",
                core::SectionType::Group => "group",
            };
            let desc = s.description.as_deref().unwrap_or("");
            println!(
                "{:<18} {:<7} {:<11} {:<10} {}",
                s.id, type_label, s.state, lines, desc
            );
        }
    }
}
```

- [ ] **Step 2: Add a snapshot-style integration test**

Append to `tests/integration.rs`:

```rust
#[test]
fn scan_table_shows_type_column() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--scan", "tests/fixtures/variants.py"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SECTION"));
    assert!(stdout.contains("TYPE"));
    assert!(stdout.contains("db:sqlite"));
    assert!(stdout.contains("pair"));
    assert!(stdout.contains("group"));
    assert!(stdout.contains("solo"));
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test --test integration scan_table_shows_type_column
git add src/main.rs tests/integration.rs
git commit -m "feat(scan): per-file table with TYPE column"
```

---

## Task 4: Recursive summary mode

When `--scan` runs with `-R`, switch the table to the §0.14.1 summary form (one row per group, with FILES + VARIANTS columns).

- [ ] **Step 1: Failing test**

Append to `tests/integration.rs`:

```rust
#[test]
fn scan_recursive_emits_summary() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::copy(
        "tests/fixtures/variants.py",
        tmp.path().join("variants.py"),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--scan", "-R", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FILES"));
    assert!(stdout.contains("VARIANTS"));
    // Per-group rows, NOT per-file rows
    assert!(stdout.contains("db"));
    assert!(stdout.contains("cache"));
    // No file paths
    assert!(!stdout.contains("variants.py"));
}
```

- [ ] **Step 2: Add summary printer**

In `src/main.rs`, add:

```rust
fn print_scan_summary(sections: &[core::ScanSectionInfo]) {
    if sections.is_empty() {
        println!("No toggle sections found.");
        return;
    }
    println!(
        "{:<18} {:<7} {:<7} {:<9} {}",
        "SECTION", "TYPE", "FILES", "VARIANTS", "STATE"
    );
    println!("{}", "\u{2500}".repeat(60));

    for s in core::summarize_scan(sections) {
        let type_label = match s.section_type {
            core::SectionType::Solo => "solo",
            core::SectionType::Pair => "pair",
            core::SectionType::Group => "group",
        };
        let variants = if matches!(s.section_type, core::SectionType::Solo) {
            "—".to_string()
        } else {
            s.variant_count.to_string()
        };
        println!(
            "{:<18} {:<7} {:<7} {:<9} {}",
            s.group, type_label, s.file_count, variants, s.state
        );
    }
}
```

- [ ] **Step 3: Route in `run_scan`**

In `src/main.rs::run_scan` (around line 945), replace the print branch:

```rust
    if cli.json {
        // JSON nesting handled in Task 6
        println!(
            "{}",
            serde_json::to_string_pretty(&all_sections).expect("Failed to serialize JSON")
        );
    } else if !cli.sections.is_empty() {
        // Detailed view handled in Task 5
        print_scan_detailed(&all_sections, &cli.sections);
    } else if cli.recursive {
        print_scan_summary(&all_sections);
    } else {
        print_scan_results(&all_sections);
    }
```

(`print_scan_detailed` is added in Task 5; if compiling now fails, stub it: `fn print_scan_detailed(_s: &[core::ScanSectionInfo], _ids: &[String]) {}` and remove the stub once Task 5 lands.)

- [ ] **Step 4: Run + commit**

```bash
cargo test --test integration scan_recursive_emits_summary
git add src/main.rs tests/integration.rs
git commit -m "feat(scan): recursive summary mode"
```

---

## Task 5: `--scan -S <id>` detailed view

- [ ] **Step 1: Failing test**

Append to `tests/integration.rs`:

```rust
#[test]
fn scan_detailed_view_for_group() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--scan", "-S", "db", "tests/fixtures/variants.py"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GROUP: db"));
    assert!(stdout.contains("db:sqlite"));
    assert!(stdout.contains("db:postgres"));
}
```

- [ ] **Step 2: Implement `print_scan_detailed`**

In `src/main.rs`:

```rust
fn print_scan_detailed(sections: &[core::ScanSectionInfo], ids: &[String]) {
    for id in ids {
        let (group, variant) = core::parse_id_parts(id);
        let in_scope: Vec<&core::ScanSectionInfo> = sections
            .iter()
            .filter(|s| {
                s.group == group && variant.as_deref().is_none_or(|v| s.variant.as_deref() == Some(v))
            })
            .collect();

        if in_scope.is_empty() {
            println!("No sections found for '{id}'.");
            continue;
        }

        let summary = core::summarize_scan(sections)
            .into_iter()
            .find(|s| s.group == group);
        if let Some(s) = summary {
            println!(
                "GROUP: {} ({}, {} variants)\n",
                s.group,
                match s.section_type {
                    core::SectionType::Solo => "solo",
                    core::SectionType::Pair => "pair",
                    core::SectionType::Group => "group",
                },
                s.variant_count.max(1)
            );
        }

        // Group items by full id
        let mut by_id: std::collections::BTreeMap<&String, Vec<&core::ScanSectionInfo>> =
            std::collections::BTreeMap::new();
        for s in in_scope {
            by_id.entry(&s.id).or_default().push(s);
        }
        for (vid, items) in by_id {
            let state = items[0].state.clone();
            println!("  {vid} [{state}]");
            for it in items {
                let end = it.end_line.map_or("?".to_string(), |e| e.to_string());
                println!("    {:<40} lines {}-{}", it.file, it.start_line, end);
            }
            println!();
        }
    }
}
```

> If clippy complains about `is_none_or` (newer clippy lint), substitute `match variant { None => true, Some(ref v) => s.variant.as_deref() == Some(v.as_str()) }`.

- [ ] **Step 3: Run + commit**

```bash
cargo test --test integration scan_detailed_view_for_group
git add src/main.rs tests/integration.rs
git commit -m "feat(scan): --scan -S detailed group view"
```

---

## Task 6: Nested JSON output

PRD §0.14.4 wants variants nested under groups. Build a custom serializable view rather than dumping the flat `Vec<ScanSectionInfo>`.

- [ ] **Step 1: Failing test**

Append to `tests/integration.rs`:

```rust
#[test]
fn scan_json_nests_variants_under_groups() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_toggle"))
        .args(["--scan", "--json", "tests/fixtures/variants.py"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid JSON");
    let sections = v.get("sections").expect("missing sections");
    let arr = sections.as_array().expect("sections not array");

    // Find db group
    let db = arr
        .iter()
        .find(|e| e.get("group").and_then(|g| g.as_str()) == Some("db"))
        .expect("db group missing");
    assert_eq!(db.get("type").and_then(|t| t.as_str()), Some("pair"));
    let variants = db.get("variants").and_then(|v| v.as_array()).unwrap();
    assert_eq!(variants.len(), 2);
}
```

- [ ] **Step 2: Add a serializable view + emit it**

In `src/main.rs`, add:

```rust
#[derive(serde::Serialize)]
struct ScanJsonRoot {
    sections: Vec<ScanJsonEntry>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum ScanJsonEntry {
    Solo {
        id: String,
        #[serde(rename = "type")]
        section_type: core::SectionType,
        files: Vec<ScanJsonFile>,
    },
    Group {
        group: String,
        #[serde(rename = "type")]
        section_type: core::SectionType,
        variants: Vec<ScanJsonVariant>,
    },
}

#[derive(serde::Serialize)]
struct ScanJsonVariant {
    id: String,
    state: String,
    files: Vec<ScanJsonFile>,
}

#[derive(serde::Serialize)]
struct ScanJsonFile {
    path: String,
    start: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<usize>,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<String>,
}

fn build_scan_json(sections: &[core::ScanSectionInfo]) -> ScanJsonRoot {
    let summaries = core::summarize_scan(sections);
    let mut entries = Vec::new();
    for s in summaries {
        let group_items: Vec<&core::ScanSectionInfo> =
            sections.iter().filter(|x| x.group == s.group).collect();

        if matches!(s.section_type, core::SectionType::Solo) {
            let files = group_items
                .iter()
                .map(|x| ScanJsonFile {
                    path: x.file.clone(),
                    start: x.start_line,
                    end: x.end_line,
                    state: x.state.clone(),
                    desc: x.description.clone(),
                })
                .collect();
            entries.push(ScanJsonEntry::Solo {
                id: s.group.clone(),
                section_type: s.section_type,
                files,
            });
        } else {
            let mut variants = Vec::new();
            for v in &s.variants {
                let id = format!("{}:{}", s.group, v);
                let files: Vec<&&core::ScanSectionInfo> =
                    group_items.iter().filter(|x| x.id == id).collect();
                let state = files
                    .first()
                    .map(|x| x.state.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                variants.push(ScanJsonVariant {
                    id,
                    state,
                    files: files
                        .into_iter()
                        .map(|x| ScanJsonFile {
                            path: x.file.clone(),
                            start: x.start_line,
                            end: x.end_line,
                            state: x.state.clone(),
                            desc: x.description.clone(),
                        })
                        .collect(),
                });
            }
            entries.push(ScanJsonEntry::Group {
                group: s.group,
                section_type: s.section_type,
                variants,
            });
        }
    }
    ScanJsonRoot { sections: entries }
}
```

In `run_scan`, replace the JSON branch:

```rust
    if cli.json {
        let root = build_scan_json(&all_sections);
        println!("{}", serde_json::to_string_pretty(&root).expect("JSON"));
    } else if ...
```

- [ ] **Step 3: Run + commit**

```bash
cargo test --test integration scan_json_nests_variants_under_groups
git add src/main.rs tests/integration.rs
git commit -m "feat(scan): nested JSON output per PRD §0.14.4"
```

---

## Task 7: Final dev cycle

- [ ] **Step 1: Run full cycle**

Run: `just dev`
Expected: pass clean. Fix any unused-import or dead-code warnings introduced by removed flat-JSON paths.

- [ ] **Step 2: Commit any cleanup**

```bash
git status
# if changes:
git add -p
git commit -m "chore: cleanup after scan rewrite"
```
