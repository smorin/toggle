use toggle::core::{
    check_if_commented, get_comment_style, merge_ranges, parse_line_range, toggle_comments,
    toggle_lines, CommentStyle, LineRange,
};

// ── parse_line_range ──

#[test]
fn test_line_range_creation() {
    let range = LineRange::new(5, 10);
    assert_eq!(range.start, 5);
    assert_eq!(range.end, 10);
}

#[test]
fn test_parse_line_range_start_end() {
    let (start, end) = parse_line_range("5:10").unwrap();
    assert_eq!(start, 5);
    assert_eq!(end, 10);
}

#[test]
fn test_parse_line_range_start_plus_count() {
    let (start, end) = parse_line_range("5:+3").unwrap();
    assert_eq!(start, 5);
    assert_eq!(end, 8);
}

#[test]
fn test_parse_line_range_single_line() {
    let (start, end) = parse_line_range("7").unwrap();
    assert_eq!(start, 7);
    assert_eq!(end, 7);
}

#[test]
fn test_parse_line_range_invalid() {
    assert!(parse_line_range("abc").is_err());
    assert!(parse_line_range("1:abc").is_err());
    assert!(parse_line_range("1:+abc").is_err());
}

// ── merge_ranges ──

#[test]
fn test_merge_ranges_empty() {
    let merged = merge_ranges(&[]);
    assert!(merged.is_empty());
}

#[test]
fn test_merge_ranges_single() {
    let merged = merge_ranges(&[LineRange::new(1, 5)]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].start, 1);
    assert_eq!(merged[0].end, 5);
}

#[test]
fn test_merge_ranges_non_overlapping() {
    let merged = merge_ranges(&[LineRange::new(1, 5), LineRange::new(10, 15)]);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].start, 1);
    assert_eq!(merged[0].end, 5);
    assert_eq!(merged[1].start, 10);
    assert_eq!(merged[1].end, 15);
}

#[test]
fn test_merge_ranges_overlapping() {
    let merged = merge_ranges(&[LineRange::new(1, 5), LineRange::new(3, 8)]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].start, 1);
    assert_eq!(merged[0].end, 8);
}

#[test]
fn test_merge_ranges_adjacent() {
    let merged = merge_ranges(&[LineRange::new(1, 5), LineRange::new(6, 10)]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].start, 1);
    assert_eq!(merged[0].end, 10);
}

#[test]
fn test_merge_ranges_unsorted() {
    let merged = merge_ranges(&[LineRange::new(10, 15), LineRange::new(1, 5)]);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].start, 1);
    assert_eq!(merged[0].end, 5);
    assert_eq!(merged[1].start, 10);
    assert_eq!(merged[1].end, 15);
}

#[test]
fn test_merge_ranges_prd_example() {
    // PRD: -l 3:5 -l 4:+4 -l 12:12 → [[3,8], [12,12]]
    let merged = merge_ranges(&[
        LineRange::new(3, 5),
        LineRange::new(4, 8),
        LineRange::new(12, 12),
    ]);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].start, 3);
    assert_eq!(merged[0].end, 8);
    assert_eq!(merged[1].start, 12);
    assert_eq!(merged[1].end, 12);
}

// ── toggle_comments ──

#[test]
fn test_toggle_comments_uncomment() {
    let content = "# This is a comment\n# Another comment";
    let ranges = vec![LineRange::new(1, 2)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "This is a comment\nAnother comment");
}

#[test]
fn test_toggle_comments_comment() {
    let content = "print('hello')\nprint('world')";
    let ranges = vec![LineRange::new(1, 2)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "# print('hello')\n# print('world')");
}

#[test]
fn test_toggle_comments_force_on() {
    let content = "print('hello')";
    let ranges = vec![LineRange::new(1, 1)];
    let result = toggle_comments(content, &ranges, Some("on"));
    assert_eq!(result, "# print('hello')");
}

#[test]
fn test_toggle_comments_force_off() {
    let content = "# print('hello')";
    let ranges = vec![LineRange::new(1, 1)];
    let result = toggle_comments(content, &ranges, Some("off"));
    assert_eq!(result, "print('hello')");
}

#[test]
fn test_toggle_comments_preserves_indentation() {
    let content = "    # indented comment";
    let ranges = vec![LineRange::new(1, 1)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "    indented comment");
}

#[test]
fn test_toggle_comments_empty() {
    let content = "";
    let ranges = vec![LineRange::new(1, 1)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "");
}

#[test]
fn test_toggle_comments_range_boundary() {
    let content = "line1\nline2\nline3\nline4";
    let ranges = vec![LineRange::new(2, 3)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "line1\n# line2\n# line3\nline4");
}

#[test]
fn test_toggle_comments_preserves_trailing_newline() {
    let content = "# hello\n";
    let ranges = vec![LineRange::new(1, 1)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, "hello\n");
}

#[test]
fn test_toggle_comments_skips_shebang() {
    let content = "#!/usr/bin/env python3\n# regular comment";
    let ranges = vec![LineRange::new(1, 2)];
    let result = toggle_comments(content, &ranges, None);
    // Shebang is protected, only second line toggled
    assert_eq!(result, "#!/usr/bin/env python3\nregular comment");
}

// ── check_if_commented ──

#[test]
fn test_check_if_commented_all_commented() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let lines = vec!["# comment".to_string(), "# another".to_string()];
    assert!(check_if_commented(&lines, &style));
}

#[test]
fn test_check_if_commented_not_commented() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let lines = vec!["code".to_string(), "more code".to_string()];
    assert!(!check_if_commented(&lines, &style));
}

#[test]
fn test_check_if_commented_blank_lines() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let lines = vec!["".to_string(), "  ".to_string()];
    assert!(!check_if_commented(&lines, &style));
}

#[test]
fn test_check_if_commented_first_nonblank_determines() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let lines = vec!["".to_string(), "# comment".to_string(), "code".to_string()];
    assert!(check_if_commented(&lines, &style));
}

// ── toggle_lines ──

#[test]
fn test_toggle_lines_comment() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let mut lines = vec!["hello".to_string(), "world".to_string()];
    toggle_lines(&mut lines, 0, 2, None, &style).unwrap();
    assert_eq!(lines[0], "# hello");
    assert_eq!(lines[1], "# world");
}

#[test]
fn test_toggle_lines_uncomment() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let mut lines = vec!["# hello".to_string(), "# world".to_string()];
    toggle_lines(&mut lines, 0, 2, None, &style).unwrap();
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "world");
}

#[test]
fn test_toggle_lines_force_on() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let mut lines = vec!["hello".to_string()];
    toggle_lines(&mut lines, 0, 1, Some(true), &style).unwrap();
    assert_eq!(lines[0], "# hello");
}

#[test]
fn test_toggle_lines_force_off() {
    let style = CommentStyle {
        single_line: "#".to_string(),
    };
    let mut lines = vec!["# hello".to_string()];
    toggle_lines(&mut lines, 0, 1, Some(false), &style).unwrap();
    assert_eq!(lines[0], "hello");
}

// ── get_comment_style ──

#[test]
fn test_get_comment_style_python() {
    let style = get_comment_style(std::path::Path::new("test.py"), "auto").unwrap();
    assert_eq!(style.single_line, "#");
}

#[test]
fn test_get_comment_style_javascript() {
    let style = get_comment_style(std::path::Path::new("test.js"), "auto").unwrap();
    assert_eq!(style.single_line, "//");
}

#[test]
fn test_get_comment_style_rust() {
    let style = get_comment_style(std::path::Path::new("test.rs"), "auto").unwrap();
    assert_eq!(style.single_line, "//");
}

#[test]
fn test_get_comment_style_shell() {
    let style = get_comment_style(std::path::Path::new("test.sh"), "auto").unwrap();
    assert_eq!(style.single_line, "#");
}

#[test]
fn test_get_comment_style_unsupported() {
    assert!(get_comment_style(std::path::Path::new("test.xyz"), "auto").is_err());
}
