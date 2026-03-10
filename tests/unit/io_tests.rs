use toggle::io::{detect_protected_lines, is_symlink, normalize_eol, read_file_encoded};

#[test]
fn test_detect_shebang() {
    let content = "#!/usr/bin/env python3\nprint('hello')";
    let protected = detect_protected_lines(content);
    assert_eq!(protected, vec![0]);
}

#[test]
fn test_detect_encoding_pragma() {
    let content = "# -*- coding: utf-8 -*-\nprint('hello')";
    let protected = detect_protected_lines(content);
    assert_eq!(protected, vec![0]);
}

#[test]
fn test_detect_encoding_pragma_equals() {
    let content = "# coding=utf-8\nprint('hello')";
    let protected = detect_protected_lines(content);
    assert_eq!(protected, vec![0]);
}

#[test]
fn test_detect_both_shebang_and_encoding() {
    let content = "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\nprint('hello')";
    let protected = detect_protected_lines(content);
    assert!(protected.contains(&0));
    assert!(protected.contains(&1));
    assert_eq!(protected.len(), 2);
}

#[test]
fn test_detect_none() {
    let content = "print('hello')\nprint('world')";
    let protected = detect_protected_lines(content);
    assert!(protected.is_empty());
}

#[test]
fn test_detect_skips_blank_leading_lines() {
    let content = "\n\n#!/usr/bin/env python3\nprint('hello')";
    let protected = detect_protected_lines(content);
    assert_eq!(protected, vec![2]);
}

// ── normalize_eol ──

#[test]
fn test_normalize_eol_preserve() {
    let content = "line1\r\nline2\nline3\r\n";
    assert_eq!(normalize_eol(content, "preserve"), content);
}

#[test]
fn test_normalize_eol_lf_converts_crlf() {
    let content = "line1\r\nline2\r\nline3\r\n";
    assert_eq!(normalize_eol(content, "lf"), "line1\nline2\nline3\n");
}

#[test]
fn test_normalize_eol_lf_handles_bare_cr() {
    let content = "line1\rline2\n";
    assert_eq!(normalize_eol(content, "lf"), "line1\nline2\n");
}

#[test]
fn test_normalize_eol_crlf_converts_lf() {
    let content = "line1\nline2\nline3\n";
    assert_eq!(
        normalize_eol(content, "crlf"),
        "line1\r\nline2\r\nline3\r\n"
    );
}

#[test]
fn test_normalize_eol_crlf_no_double_convert() {
    // Content already has CRLF — should not double up
    let content = "line1\r\nline2\r\n";
    assert_eq!(normalize_eol(content, "crlf"), "line1\r\nline2\r\n");
}

// ── is_symlink ──

#[cfg(unix)]
#[test]
fn test_is_symlink_true() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("target.py");
    std::fs::write(&target, "hello").unwrap();
    let link = dir.path().join("link.py");
    symlink(&target, &link).unwrap();
    assert!(is_symlink(&link));
}

#[cfg(unix)]
#[test]
fn test_is_symlink_false() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("regular.py");
    std::fs::write(&file, "hello").unwrap();
    assert!(!is_symlink(&file));
}

// ── read_file_encoded ──

#[test]
fn test_read_file_encoded_utf8() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "hello world").unwrap();
    let content = read_file_encoded(&path, "utf-8").unwrap();
    assert_eq!(content, "hello world");
}

#[test]
fn test_read_file_encoded_latin1() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.py");
    // "café" in Latin-1: 63 61 66 e9
    std::fs::write(&path, [0x63, 0x61, 0x66, 0xe9]).unwrap();
    let content = read_file_encoded(&path, "latin-1").unwrap();
    assert_eq!(content, "caf\u{e9}");
}

#[test]
fn test_read_file_encoded_unsupported() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.py");
    std::fs::write(&path, "hello").unwrap();
    let result = read_file_encoded(&path, "bogus-codec");
    assert!(result.is_err());
}
