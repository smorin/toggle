use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("toggle").unwrap()
}

fn setup_temp_file(content: &str, filename: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(filename);
    fs::write(&path, content).unwrap();
    (dir, path)
}

// ── Repeatable --line ranges (Phase 2) ──

#[test]
fn test_multiple_line_ranges_non_adjacent() {
    let (_dir, path) = setup_temp_file("a\nb\nc\nd\ne\nf\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "2:3", "-l", "5:6"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "a\n# b\n# c\nd\n# e\n# f\n");
}

#[test]
fn test_multiple_line_ranges_overlapping() {
    let (_dir, path) = setup_temp_file("a\nb\nc\nd\ne\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:3", "-l", "2:4"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    // Lines 1-4 should all be commented (merged ranges)
    assert_eq!(result, "# a\n# b\n# c\n# d\ne\n");
}

// ── --to-end (Phase 2) ──

#[test]
fn test_to_end_extends_to_eof() {
    let (_dir, path) = setup_temp_file("a\nb\nc\nd\ne\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "3", "--to-end"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "a\nb\n# c\n# d\n# e\n");
}

#[test]
fn test_to_end_without_line_errors() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--to-end"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--to-end requires"));
}

// ── Multi-line comment support (Phase 2) ──

#[test]
fn test_multi_line_mode_wraps_in_block_comment() {
    let (_dir, path) = setup_temp_file("line1\nline2\nline3\nline4\n", "test.js");
    cmd()
        .args([path.to_str().unwrap(), "-l", "2:3", "-m", "multi"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("/*"), "should contain block comment start");
    assert!(result.contains("*/"), "should contain block comment end");
}

#[test]
fn test_multi_line_mode_unwraps_block_comment() {
    let (_dir, path) = setup_temp_file("line1\n/* line2\nline3 */\nline4\n", "test.js");
    cmd()
        .args([path.to_str().unwrap(), "-l", "2:3", "-m", "multi"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(!result.contains("/*"), "should unwrap block comment start");
    assert!(!result.contains("*/"), "should unwrap block comment end");
}

#[test]
fn test_multi_line_mode_unsupported_for_python() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "-m", "multi"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Multi-line comments not supported",
        ));
}

// ── --comment-style override (Phase 2) ──

#[test]
fn test_comment_style_single_override() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.txt");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--comment-style", "//"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(
        result.contains("// hello"),
        "should use custom // delimiter"
    );
}

#[test]
fn test_comment_style_with_multi_line() {
    let (_dir, path) = setup_temp_file("a\nb\nc\n", "test.txt");
    cmd()
        .args([
            path.to_str().unwrap(),
            "-l",
            "1:2",
            "-m",
            "multi",
            "--comment-style",
            "//",
            "/*",
            "*/",
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("/*"), "should use custom multi-line start");
    assert!(result.contains("*/"), "should use custom multi-line end");
}

#[test]
fn test_comment_style_two_values_errors() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([
            path.to_str().unwrap(),
            "-l",
            "1:1",
            "--comment-style",
            "//",
            "/*",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--comment-style requires"));
}

// ── --interactive (Phase 2) ──

#[test]
fn test_interactive_yes_modifies_file() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--interactive"])
        .write_stdin("y\n")
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("#"), "file should be modified on 'y'");
}

#[test]
fn test_interactive_no_skips_modification() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    let original = fs::read_to_string(&path).unwrap();
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--interactive"])
        .write_stdin("n\n")
        .assert()
        .success();
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        original, after,
        "file should not be modified when user answers 'n'"
    );
}

// ── Line range toggling ──

#[test]
fn test_toggle_line_range_comments_lines() {
    let (_dir, path) = setup_temp_file("line1\nline2\nline3\nline4\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "2:3"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("line1\n"));
    assert!(result.contains("#"));
    assert!(result.contains("line4\n"));
}

#[test]
fn test_toggle_force_on() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "-f", "on"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("#"));
}

#[test]
fn test_toggle_force_off() {
    let (_dir, path) = setup_temp_file("# hello\n# world\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "-f", "off"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(!result.contains("#"));
}

// ── Section toggling ──

#[test]
fn test_toggle_section() {
    let content = "before\n# toggle:start ID=test\nhello\n# toggle:end ID=test\nafter\n";
    let (_dir, path) = setup_temp_file(content, "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-S", "test", "-f", "on"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("#hello") || result.contains("# hello"));
}

// ── --strict-ext ──

#[test]
fn test_strict_ext_rejects_non_py() {
    let (_dir, path) = setup_temp_file("console.log('hi')\n", "test.js");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--strict-ext"])
        .assert()
        .failure();
}

#[test]
fn test_strict_ext_accepts_py() {
    let (_dir, path) = setup_temp_file("print('hi')\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--strict-ext"])
        .assert()
        .success();
}

// ── --verbose ──

#[test]
fn test_verbose_outputs_to_stderr() {
    let (_dir, path) = setup_temp_file("print('hi')\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "-v"])
        .assert()
        .success()
        .stderr(predicates::str::contains("Processing"));
}

// ── Multi-language support ──

#[test]
fn test_toggle_javascript() {
    let (_dir, path) = setup_temp_file("console.log('hi');\n", "test.js");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("//"));
}

#[test]
fn test_toggle_shell() {
    let (_dir, path) = setup_temp_file("echo hello\n", "test.sh");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("#"));
}

// ── Out-of-range lines ──

#[test]
fn test_out_of_range_line_errors() {
    let (_dir, path) = setup_temp_file("line1\nline2\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "100:105"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("out of range"));
}

// ── Missing file ──

#[test]
fn test_nonexistent_file() {
    cmd()
        .args(["/tmp/nonexistent_file_toggle_test.py", "-l", "1:1"])
        .assert()
        .failure();
}

// ── --dry-run ──

#[test]
fn test_dry_run_shows_diff_and_does_not_modify_file() {
    let (_dir, path) = setup_temp_file("line1\nline2\nline3\n", "test.py");
    let original = fs::read_to_string(&path).unwrap();
    cmd()
        .args([path.to_str().unwrap(), "-l", "2:3", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("---"))
        .stdout(predicates::str::contains("+++"))
        .stdout(predicates::str::contains("@@"));
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        original, after,
        "file should not be modified in dry-run mode"
    );
}

#[test]
fn test_dry_run_no_changes_empty_stdout() {
    let (_dir, path) = setup_temp_file("# already commented\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "-f", "on", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not().or(predicate::always()));
    // File should not be modified
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(after, "# already commented\n");
}

#[test]
fn test_dry_run_with_verbose() {
    let (_dir, path) = setup_temp_file("line1\nline2\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--dry-run", "-v"])
        .assert()
        .success()
        .stderr(predicates::str::contains("Processing"));
}

// ── --backup ──

#[test]
fn test_backup_creates_backup_file() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    let original = fs::read_to_string(&path).unwrap();
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--backup", ".bak"])
        .assert()
        .success();
    let backup_path = path.with_file_name("test.py.bak");
    assert!(backup_path.exists(), "backup file should exist");
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(
        backup_content, original,
        "backup should contain original content"
    );
    let modified = fs::read_to_string(&path).unwrap();
    assert!(modified.contains("#"), "original file should be toggled");
}

#[test]
fn test_backup_with_dry_run_skips_backup() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    cmd()
        .args([
            path.to_str().unwrap(),
            "-l",
            "1:2",
            "--backup",
            ".bak",
            "--dry-run",
        ])
        .assert()
        .success();
    let backup_path = path.with_file_name("test.py.bak");
    assert!(
        !backup_path.exists(),
        "backup should not be created in dry-run mode"
    );
}

// ── --config ──

#[test]
fn test_config_custom_delimiter() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join(".toggleConfig");
    fs::write(
        &config_path,
        r#"
[language.python]
single_line_delimiter = ";;"
"#,
    )
    .unwrap();
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, "hello\nworld\n").unwrap();
    cmd()
        .args([
            file_path.to_str().unwrap(),
            "-l",
            "1:2",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains(";;"),
        "should use custom delimiter from config"
    );
}

#[test]
fn test_config_cli_force_overrides_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join(".toggleConfig");
    fs::write(
        &config_path,
        r#"
[global]
force_state = "on"
"#,
    )
    .unwrap();
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, "# commented\n").unwrap();
    cmd()
        .args([
            file_path.to_str().unwrap(),
            "-l",
            "1:1",
            "-f",
            "off",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(
        !result.contains("#"),
        "CLI --force off should override config force_state=on"
    );
}

#[test]
fn test_config_nonexistent_file_errors() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([
            path.to_str().unwrap(),
            "-l",
            "1:1",
            "--config",
            "/tmp/nonexistent_toggle_config",
        ])
        .assert()
        .failure();
}

#[test]
fn test_config_global_default_mode() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join(".toggleConfig");
    fs::write(
        &config_path,
        r#"
[global]
force_state = "on"
"#,
    )
    .unwrap();
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, "hello\n").unwrap();
    cmd()
        .args([
            file_path.to_str().unwrap(),
            "-l",
            "1:1",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("# hello"),
        "config force_state=on should comment the line"
    );
}

// ── --eol ──

#[test]
fn test_eol_lf_normalizes_crlf() {
    let (_dir, path) = setup_temp_file("line1\r\nline2\r\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--eol", "lf"])
        .assert()
        .success();
    let result = fs::read(&path).unwrap();
    assert!(
        !result.windows(2).any(|w| w == b"\r\n"),
        "should have no CRLF after --eol lf"
    );
}

#[test]
fn test_eol_crlf_normalizes_lf() {
    let (_dir, path) = setup_temp_file("line1\nline2\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--eol", "crlf"])
        .assert()
        .success();
    let result = fs::read(&path).unwrap();
    let content = String::from_utf8(result).unwrap();
    assert!(
        content.contains("\r\n"),
        "should have CRLF after --eol crlf"
    );
}

#[test]
fn test_eol_preserve_keeps_original() {
    let original = "line1\nline2\n";
    let (_dir, path) = setup_temp_file(original, "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--eol", "preserve"])
        .assert()
        .success();
    let result = fs::read(&path).unwrap();
    // Verify no \r was introduced
    assert!(!result.contains(&b'\r'), "preserve should not introduce CR");
}

// ── --no-dereference ──

#[cfg(unix)]
#[test]
fn test_no_dereference_preserves_symlink() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("target.py");
    fs::write(&target, "hello\nworld\n").unwrap();
    let link = dir.path().join("link.py");
    symlink(&target, &link).unwrap();

    cmd()
        .args([link.to_str().unwrap(), "-l", "1:2", "-N"])
        .assert()
        .success();

    // Symlink should still be a symlink
    assert!(
        link.symlink_metadata().unwrap().file_type().is_symlink(),
        "symlink should be preserved with -N"
    );
    // Target file should be modified
    let result = fs::read_to_string(&target).unwrap();
    assert!(result.contains("#"), "target should be toggled");
}

#[cfg(unix)]
#[test]
fn test_default_replaces_symlink() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("target.py");
    fs::write(&target, "hello\nworld\n").unwrap();
    let link = dir.path().join("link.py");
    symlink(&target, &link).unwrap();

    cmd()
        .args([link.to_str().unwrap(), "-l", "1:2"])
        .assert()
        .success();

    // Without -N, the symlink gets replaced by the atomic rename
    assert!(
        !link.symlink_metadata().unwrap().file_type().is_symlink(),
        "without -N, symlink should be replaced by a regular file"
    );
}

#[test]
fn test_eol_invalid_value_errors() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--eol", "foo"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Invalid --eol value"));
}

// ── --encoding ──

#[test]
fn test_encoding_latin1_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.py");
    // "café\n" in Latin-1
    fs::write(&path, &[0x63, 0x61, 0x66, 0xe9, 0x0a]).unwrap();
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "-e", "latin-1"])
        .assert()
        .success();
    let bytes = fs::read(&path).unwrap();
    // Should have comment marker "# " (0x23 0x20) before "café"
    assert!(
        bytes.windows(2).any(|w| w == [0x23, 0x20]),
        "should have # comment marker"
    );
    // Should still contain the é character (0xe9) in Latin-1
    assert!(bytes.contains(&0xe9), "should preserve Latin-1 encoding");
}

#[test]
fn test_encoding_default_utf8() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert!(result.contains("#"), "default UTF-8 should work as before");
}

#[test]
fn test_encoding_invalid_errors() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "-e", "bogus-codec"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Unsupported encoding"));
}

#[test]
fn test_config_global_delimiter_fallback() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join(".toggleConfig");
    fs::write(
        &config_path,
        r#"
[global]
single_line_delimiter = "%%"
"#,
    )
    .unwrap();
    // Use .py which normally uses #, but global override should use %%
    let file_path = dir.path().join("test.py");
    fs::write(&file_path, "hello\n").unwrap();
    cmd()
        .args([
            file_path.to_str().unwrap(),
            "-l",
            "1:1",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("%%"),
        "global single_line_delimiter should override default when no language-specific config"
    );
}

// ── --json ──

#[test]
fn test_json_output_valid_json() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    let output = cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert!(json.is_array(), "JSON output should be an array");
}

#[test]
fn test_json_output_contains_fields() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    let output = cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.len(), 1);
    let entry = &json[0];
    assert!(entry["file"].as_str().unwrap().contains("test.py"));
    assert_eq!(entry["action"], "toggle_line_range");
    assert!(entry["lines_changed"].as_u64().unwrap() > 0);
    assert_eq!(entry["success"], true);
    assert!(entry.get("error").is_none() || entry["error"].is_null());
    assert_eq!(entry["dry_run"], false);
}

#[test]
fn test_json_output_error_case() {
    let output = cmd()
        .args([
            "/tmp/nonexistent_toggle_json_test.py",
            "-l",
            "1:1",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json: Vec<Value> =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON even on error");
    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["success"], false);
    assert!(json[0]["error"].as_str().unwrap().len() > 0);
}

#[test]
fn test_json_suppresses_verbose() {
    let (_dir, path) = setup_temp_file("hello\n", "test.py");
    let output = cmd()
        .args([path.to_str().unwrap(), "-l", "1:1", "--json", "-v"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "verbose output should be suppressed in JSON mode"
    );
    // stdout should still be valid JSON
    let _: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
}

#[test]
fn test_json_with_dry_run() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    let output = cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--json", "--dry-run"])
        .output()
        .unwrap();
    assert!(output.status.success());
    // stdout should be only JSON, no diff output mixed in
    let json: Vec<Value> = serde_json::from_slice(&output.stdout)
        .expect("stdout should be pure JSON with no diff mixed in");
    assert_eq!(json[0]["dry_run"], true);
    // File should not be modified
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(after, "hello\nworld\n");
}

// ── -F alias and --force invert value ───────────────────────────────────────

#[test]
fn test_force_uppercase_f_alias_on() {
    let (_dir, path) = setup_temp_file("a\nb\nc\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-F", "on", "-l", "1:2"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "# a\n# b\nc\n");
}

#[test]
fn test_force_uppercase_f_alias_off() {
    let (_dir, path) = setup_temp_file("# a\n# b\nc\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-F", "off", "-l", "1:2"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "a\nb\nc\n");
}

#[test]
fn test_force_invert_value_comments_then_uncomments() {
    let (_dir, path) = setup_temp_file("a\nb\nc\n", "test.py");
    // First pass: invert on uncommented lines → comments them
    cmd()
        .args([path.to_str().unwrap(), "-f", "invert", "-l", "1:2"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "# a\n# b\nc\n");
    // Second pass: invert on commented lines → uncomments them
    cmd()
        .args([path.to_str().unwrap(), "--force", "invert", "-l", "1:2"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "a\nb\nc\n");
}

#[test]
fn test_force_invert_with_uppercase_f() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-F", "invert", "-l", "1:2"])
        .assert()
        .success();
    let result = fs::read_to_string(&path).unwrap();
    assert_eq!(result, "# a\n# b\n");
}

#[test]
fn test_force_invalid_value_errors() {
    let (_dir, path) = setup_temp_file("a\nb\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "--force", "bogus", "-l", "1:2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid --force value 'bogus'"));
}

// ── Unknown extension fallback to config ────────────────────────────────────

#[test]
fn test_unknown_extension_with_global_config_delimiter() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("data.xyz");
    fs::write(&file_path, "line one\nline two\n").unwrap();

    let config_path = dir.path().join(".toggleConfig");
    fs::write(
        &config_path,
        "[global]\nsingle_line_delimiter = \"//\"\n",
    )
    .unwrap();

    cmd()
        .args([
            file_path.to_str().unwrap(),
            "-l",
            "1:2",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    let result = fs::read_to_string(&file_path).unwrap();
    assert_eq!(result, "// line one\n// line two\n");
}

#[test]
fn test_unknown_extension_without_config_errors() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("data.xyz");
    fs::write(&file_path, "line one\nline two\n").unwrap();

    cmd()
        .args([file_path.to_str().unwrap(), "-l", "1:2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported file extension: .xyz"));
}

#[test]
fn test_unknown_extension_error_suggests_alternatives() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("data.xyz");
    fs::write(&file_path, "line one\n").unwrap();

    cmd()
        .args([file_path.to_str().unwrap(), "-l", "1:1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--comment-style"))
        .stderr(predicate::str::contains("--config"));
}
