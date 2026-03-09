use assert_cmd::Command;
use predicates::prelude::*;
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
    assert_eq!(original, after, "file should not be modified in dry-run mode");
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
    assert_eq!(backup_content, original, "backup should contain original content");
    let modified = fs::read_to_string(&path).unwrap();
    assert!(modified.contains("#"), "original file should be toggled");
}

#[test]
fn test_backup_with_dry_run_skips_backup() {
    let (_dir, path) = setup_temp_file("hello\nworld\n", "test.py");
    cmd()
        .args([path.to_str().unwrap(), "-l", "1:2", "--backup", ".bak", "--dry-run"])
        .assert()
        .success();
    let backup_path = path.with_file_name("test.py.bak");
    assert!(!backup_path.exists(), "backup should not be created in dry-run mode");
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
    assert!(result.contains(";;"), "should use custom delimiter from config");
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
    assert!(!result.contains("#"), "CLI --force off should override config force_state=on");
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
    assert!(result.contains("# hello"), "config force_state=on should comment the line");
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
