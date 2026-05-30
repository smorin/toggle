//! Tests for stdin/stdout filter mode (Option 3).
//!
//! Filter mode is a single stdin→stdout transform with three spellings: a `-`
//! path, `--stdin`, or `--stdout`. It applies to the writer operations
//! (toggle/insert/remove) only. The core guarantees:
//!   * stdin≡file: the filter output equals the in-place file result, byte for byte.
//!   * no-op byte identity: an input that triggers no change is emitted verbatim,
//!     including exact trailing-newline handling.
//!   * the rejection set: flags that collide with stdout output are refused.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("toggle").unwrap()
}

const SECTION_FILE: &str = "# toggle:start ID=feat\nprint(\"hi\")\n# toggle:end ID=feat\nafter\n";

/// Run a filter-mode command feeding `input` on stdin; return stdout bytes.
fn filter_stdout(args: &[&str], input: &str) -> Vec<u8> {
    cmd()
        .args(args)
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone()
}

/// Apply `legacy_args` (a flat-flag write op) to a file containing `input`,
/// returning the file's bytes afterward.
fn file_result(input: &str, legacy_args: &[&str]) -> Vec<u8> {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("f.py");
    fs::write(&p, input).unwrap();
    let mut args: Vec<&str> = vec![p.to_str().unwrap()];
    args.extend_from_slice(legacy_args);
    cmd().args(&args).assert().success();
    fs::read(&p).unwrap()
}

// ── stdin≡file parity (filter stdout == in-place file bytes) ──

#[test]
fn toggle_filter_equals_file() {
    let via_stdin = filter_stdout(&["toggle", "-", "-S", "feat"], SECTION_FILE);
    let via_file = file_result(SECTION_FILE, &["-S", "feat"]);
    assert_eq!(via_stdin, via_file);
}

#[test]
fn remove_filter_equals_file() {
    let via_stdin = filter_stdout(&["remove", "-", "-S", "feat"], SECTION_FILE);
    let via_file = file_result(SECTION_FILE, &["--remove", "-S", "feat"]);
    assert_eq!(via_stdin, via_file);
}

#[test]
fn insert_filter_equals_file() {
    let input = "a\nb\nc\n";
    let via_stdin = filter_stdout(
        &["insert", "-", "-S", "new", "-l", "2:2", "--desc", "d"],
        input,
    );
    let via_file = file_result(
        input,
        &["--insert", "-S", "new", "-l", "2:2", "--desc", "d"],
    );
    assert_eq!(via_stdin, via_file);
}

// ── no-op byte identity (verbatim passthrough, exact trailing newline) ──

#[test]
fn noop_preserves_trailing_newline() {
    let input = "x\ny\nz\n";
    let out = filter_stdout(&["toggle", "-", "-S", "absent"], input);
    assert_eq!(out, input.as_bytes());
}

#[test]
fn noop_preserves_missing_trailing_newline() {
    let input = "x\ny\nz"; // no trailing newline
    let out = filter_stdout(&["toggle", "-", "-S", "absent"], input);
    assert_eq!(out, input.as_bytes());
}

// ── spelling equivalence: `-`, --stdin, --stdout ──

#[test]
fn stdin_and_stdout_aliases_match_dash() {
    let dash = filter_stdout(&["toggle", "-", "-S", "feat"], SECTION_FILE);
    let stdin = filter_stdout(&["toggle", "--stdin", "-S", "feat"], SECTION_FILE);
    let stdout = filter_stdout(&["toggle", "--stdout", "-S", "feat"], SECTION_FILE);
    assert_eq!(dash, stdin);
    assert_eq!(dash, stdout);
}

#[test]
fn insert_via_stdin_flag_without_path() {
    // --stdin lets insert omit the positional path entirely.
    let out = filter_stdout(
        &["insert", "--stdin", "-S", "new", "-l", "2:2"],
        "a\nb\nc\n",
    );
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("toggle:start ID=new"));
    assert!(s.contains("toggle:end ID=new"));
}

// ── rejection set ──

fn assert_filter_rejected(args: &[&str]) {
    cmd()
        .args(args)
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}

#[test]
fn rejects_json_in_filter_mode() {
    assert_filter_rejected(&["toggle", "-", "-S", "feat", "--json"]);
}

#[test]
fn rejects_dry_run_in_filter_mode() {
    assert_filter_rejected(&["toggle", "-", "-S", "feat", "--dry-run"]);
}

#[test]
fn rejects_atomic_in_filter_mode() {
    assert_filter_rejected(&["toggle", "-", "-S", "feat", "--atomic"]);
}

#[test]
fn rejects_backup_in_filter_mode() {
    assert_filter_rejected(&["toggle", "-", "-S", "feat", "--backup", ".bak"]);
}

#[test]
fn rejects_recursive_in_filter_mode() {
    assert_filter_rejected(&["toggle", "-", "-S", "feat", "-R"]);
}

#[test]
fn rejects_real_path_with_stdout() {
    // A real file path plus --stdout is the matrix the design declined: error.
    cmd()
        .args(["toggle", "somefile.py", "--stdout", "-S", "feat"])
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}

#[test]
fn scan_rejects_stdin_flag() {
    // Read-only ops are not filter-mode writers.
    cmd()
        .args(["--scan", "--stdin"])
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}
