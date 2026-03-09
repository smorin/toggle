use toggle::io::detect_protected_lines;

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
