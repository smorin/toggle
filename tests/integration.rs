use assert_cmd::Command;
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

// ── Missing file ──

#[test]
fn test_nonexistent_file() {
    cmd()
        .args(["/tmp/nonexistent_file_toggle_test.py", "-l", "1:1"])
        .assert()
        .failure();
}
