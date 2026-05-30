//! Parity tests for the additive subcommand front-end.
//!
//! The subcommand form (`togl scan PATH`) is translated to the legacy flat
//! flags (`togl --scan PATH`) and run through the same pipeline, so the two
//! forms must be behavior-identical. These tests pin that invariant: for
//! read-only ops they compare stdout; for write ops they compare resulting
//! file bytes. At least one case (`remove` without `--remove-mode`) exercises a
//! *defaulted* field, guarding against silent default drift between the two
//! front-ends.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("toggle").unwrap()
}

const SECTION_FILE: &str = "# toggle:start ID=feat\nprint(\"hi\")\n# toggle:end ID=feat\nafter\n";

/// Run `args` and return captured stdout as a String.
fn stdout_of(args: &[&str]) -> String {
    let out = cmd().args(args).assert().get_output().stdout.clone();
    String::from_utf8(out).unwrap()
}

/// Assert that a read-only / dry-run subcommand and its legacy equivalent print
/// identical stdout. Both forms run against the *same* file, so any path in the
/// output (e.g. the `file` field in `--json`) is byte-identical without fragile
/// textual normalization (which broke on Windows, where JSON escapes `\`).
/// Safe because every caller is non-mutating (scan/check/list/`--dry-run`).
fn assert_stdout_parity(content: &str, sub: &[&str], legacy: &[&str]) {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("a.py");
    fs::write(&p, content).unwrap();
    let path = p.to_str().unwrap();

    let mut sub_args: Vec<&str> = sub.to_vec();
    sub_args.push(path);
    let mut legacy_args: Vec<&str> = legacy.to_vec();
    legacy_args.push(path);

    let sub_out = stdout_of(&sub_args);
    let legacy_out = stdout_of(&legacy_args);
    assert_eq!(sub_out, legacy_out, "stdout parity: {sub:?} vs {legacy:?}");
}

/// Assert that a write subcommand and its legacy equivalent produce
/// byte-identical files. `sub`/`legacy` are the args *before* the path; the
/// path is appended (subcommand) / handled per `path_first`.
fn assert_write_parity(content: &str, filename: &str, sub: &[&str], legacy: &[&str]) {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join(format!("a_{filename}"));
    let b = dir.path().join(format!("b_{filename}"));
    fs::write(&a, content).unwrap();
    fs::write(&b, content).unwrap();

    let mut sub_args: Vec<&str> = sub.to_vec();
    sub_args.push(a.to_str().unwrap());
    let mut legacy_args: Vec<&str> = legacy.to_vec();
    legacy_args.push(b.to_str().unwrap());

    cmd().args(&sub_args).assert().success();
    cmd().args(&legacy_args).assert().success();

    let after_a = fs::read(&a).unwrap();
    let after_b = fs::read(&b).unwrap();
    assert_eq!(
        after_a,
        after_b,
        "write parity: {sub:?} vs {legacy:?}\n  sub={:?}\n  leg={:?}",
        String::from_utf8_lossy(&after_a),
        String::from_utf8_lossy(&after_b)
    );
}

// ── Read-only parity ──

#[test]
fn scan_parity() {
    assert_stdout_parity(SECTION_FILE, &["scan"], &["--scan"]);
}

#[test]
fn scan_with_recursive_parity() {
    assert_stdout_parity(SECTION_FILE, &["scan", "-R"], &["--scan", "--recursive"]);
}

#[test]
fn check_parity() {
    assert_stdout_parity(SECTION_FILE, &["check"], &["--scan", "--check"]);
}

#[test]
fn list_default_parity() {
    assert_stdout_parity(SECTION_FILE, &["list"], &["--list-sections"]);
}

#[test]
fn list_fields_ids_parity() {
    assert_stdout_parity(
        SECTION_FILE,
        &["list", "--fields", "ids"],
        &["--list-sections", "--fields", "ids"],
    );
}

#[test]
fn toggle_json_stdout_parity() {
    // --json is a GlobalArgs flag flattened into the subcommand.
    assert_stdout_parity(
        SECTION_FILE,
        &["toggle", "-S", "feat", "--dry-run", "--json"],
        &["-S", "feat", "--dry-run", "--json"],
    );
}

// ── Write parity ──

#[test]
fn toggle_write_parity() {
    assert_write_parity(
        SECTION_FILE,
        "t.py",
        &["toggle", "-S", "feat"],
        &["-S", "feat"],
    );
}

#[test]
fn remove_default_mode_write_parity() {
    // Exercises the DEFAULTED --remove-mode (commented): if the subcommand and
    // legacy front-ends disagreed on the default, these files would diverge.
    assert_write_parity(
        SECTION_FILE,
        "r.py",
        &["remove", "-S", "feat"],
        &["--remove", "-S", "feat"],
    );
}

#[test]
fn remove_explicit_mode_write_parity() {
    assert_write_parity(
        SECTION_FILE,
        "r.py",
        &["remove", "-S", "feat", "--remove-mode", "all"],
        &["--remove", "-S", "feat", "--remove-mode", "all"],
    );
}

#[test]
fn insert_write_parity() {
    // insert takes the path as a leading positional in both forms.
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    let b = dir.path().join("b.py");
    fs::write(&a, "x\ny\nz\n").unwrap();
    fs::write(&b, "x\ny\nz\n").unwrap();

    cmd()
        .args([
            "insert",
            a.to_str().unwrap(),
            "-S",
            "new",
            "-l",
            "2:2",
            "--desc",
            "d",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "--insert",
            b.to_str().unwrap(),
            "-S",
            "new",
            "-l",
            "2:2",
            "--desc",
            "d",
        ])
        .assert()
        .success();

    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
}

// ── Subcommand-specific ergonomics & guards ──

#[test]
fn subcommand_rejects_out_of_scope_flag() {
    // The ergonomic win: each subcommand exposes only its own flags. A flag
    // that belongs to a different operation is rejected at parse time.
    cmd().args(["scan", "--insert"]).assert().failure();
    cmd()
        .args(["list", "--remove-mode", "all"])
        .assert()
        .failure();
}

#[test]
fn insert_requires_section_and_line() {
    // The insert subcommand enforces required -S/-l at parse time.
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    fs::write(&a, "x\n").unwrap();
    cmd()
        .args(["insert", a.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn remove_requires_section() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    fs::write(&a, "x\n").unwrap();
    cmd()
        .args(["remove", a.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn legacy_form_emits_no_notice_when_piped() {
    // The deprecation nudge is TTY-only; under a captured (non-TTY) stderr it
    // must stay silent so scripts/pipes are unaffected.
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    fs::write(&a, SECTION_FILE).unwrap();
    cmd()
        .args([a.to_str().unwrap(), "-S", "feat", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("deprecated").not());
}

#[test]
fn subcommands_listed_in_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("insert"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn toggle_atomic_recursive_write_parity() {
    // Atomic multi-file mode reaches the real file-mutation + journal + backup
    // path through the subcommand bridge. Each run is isolated in its own cwd
    // (the journal lives in cwd) so they don't collide.
    fn build_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        for name in ["a.py", "b.py"] {
            fs::write(dir.path().join(name), SECTION_FILE).unwrap();
        }
        dir
    }

    let sub_dir = build_dir();
    cmd()
        .current_dir(sub_dir.path())
        .args(["toggle", ".", "-S", "feat", "--atomic", "-R"])
        .assert()
        .success();

    let legacy_dir = build_dir();
    cmd()
        .current_dir(legacy_dir.path())
        .args([".", "-S", "feat", "--atomic", "-R"])
        .assert()
        .success();

    for name in ["a.py", "b.py"] {
        assert_eq!(
            fs::read(sub_dir.path().join(name)).unwrap(),
            fs::read(legacy_dir.path().join(name)).unwrap(),
            "atomic parity differs for {name}"
        );
    }
}
