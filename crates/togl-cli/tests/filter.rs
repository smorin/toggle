//! Tests for stdin/stdout filter mode (Option 3).
//!
//! Filter mode writes a single transformed stream to stdout and never modifies a
//! file, for the writer operations (toggle/insert/remove) only. The input is
//! either stdin (`-`, `--stdin`, or `--stdout` with no path) or one real file
//! (`file --stdout`, whose real extension drives the comment style). The core
//! guarantees:
//!   * stdin≡file: the filter output equals the in-place file result, byte for byte.
//!   * file→stdout: `file --stdout` emits that result and leaves the file untouched.
//!   * no-op byte identity: an input that triggers no change is emitted verbatim,
//!     including exact trailing-newline handling.
//!   * the rejection set: flags/inputs that collide with single-stream stdout output
//!     are refused.

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
fn scan_rejects_stdin_flag() {
    // Read-only ops are not filter-mode writers.
    cmd()
        .args(["--scan", "--stdin"])
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}

// ── file → stdout (`--stdout` with a real file path) ──

#[test]
fn file_to_stdout_equals_in_place_result() {
    // `file --stdout` emits the same bytes the in-place transform would write.
    let dir = TempDir::new().unwrap();
    let read_only = dir.path().join("ro.py");
    fs::write(&read_only, SECTION_FILE).unwrap();
    let out = cmd()
        .args([read_only.to_str().unwrap(), "--stdout", "-S", "feat"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let in_place = file_result(SECTION_FILE, &["-S", "feat"]);
    assert_eq!(out, in_place);
}

#[test]
fn file_to_stdout_leaves_file_unmodified() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("keep.py");
    fs::write(&p, SECTION_FILE).unwrap();
    cmd()
        .args([p.to_str().unwrap(), "--stdout", "-S", "feat"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&p).unwrap(), SECTION_FILE);
}

#[test]
fn file_to_stdout_uses_real_extension_comment_style() {
    // The point of file→stdout: comment style comes from the real extension.
    // A `.js` file must use `//`, not the synthetic-stdin Python `#` default.
    let dir = TempDir::new().unwrap();
    let js = dir.path().join("app.js");
    fs::write(
        &js,
        "// toggle:start ID=feat\nconsole.log(1)\n// toggle:end ID=feat\n",
    )
    .unwrap();
    let out = cmd()
        .args([js.to_str().unwrap(), "--stdout", "-S", "feat"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(
        s.contains("// console.log(1)"),
        "expected // comment, got:\n{s}"
    );
}

#[test]
fn rejects_dash_mixed_with_file_path() {
    cmd()
        .args(["toggle", "-", "somefile.py", "--stdout", "-S", "feat"])
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}

#[test]
fn rejects_stdin_flag_with_file_path() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("f.py");
    fs::write(&p, SECTION_FILE).unwrap();
    cmd()
        .args(["toggle", p.to_str().unwrap(), "--stdin", "-S", "feat"])
        .write_stdin(SECTION_FILE)
        .assert()
        .failure();
}

#[test]
fn rejects_multiple_files_with_stdout() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.py");
    let b = dir.path().join("b.py");
    fs::write(&a, SECTION_FILE).unwrap();
    fs::write(&b, SECTION_FILE).unwrap();
    cmd()
        .args([
            "toggle",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--stdout",
            "-S",
            "feat",
        ])
        .assert()
        .failure();
}

#[test]
fn rejects_directory_with_stdout() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([
            "toggle",
            dir.path().to_str().unwrap(),
            "--stdout",
            "-S",
            "feat",
        ])
        .assert()
        .failure();
}
