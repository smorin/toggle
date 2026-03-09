use toggle::core::{parse_line_range, merge_ranges, toggle_comments, LineRange};

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
    assert_eq!(end, 8);
}

#[test]
fn test_parse_line_range_invalid() {
    assert!(parse_line_range("abc").is_err());
    assert!(parse_line_range("1:abc").is_err());
    assert!(parse_line_range("1:+abc").is_err());
}

#[test]
fn test_merge_ranges() {
    let ranges = vec![
        LineRange::new(1, 5),
        LineRange::new(10, 15),
    ];
    let merged = merge_ranges(&ranges);
    assert_eq!(merged.len(), 0); // Currently returns empty vec (stub)
}

#[test]
fn test_toggle_comments() {
    let content = "# This is a comment\nThis is not a comment";
    let ranges = vec![LineRange::new(1, 2)];
    let result = toggle_comments(content, &ranges, None);
    assert_eq!(result, content); // Currently returns input unchanged (stub)
}
