use std::path::Path;
use togl_lib::core::{
    find_and_toggle_section, get_comment_style, insert_section, merge_ranges, parse_line_range,
    scan_sections, supported_extensions, toggle_comments, CommentStyle, LineRange,
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

#[test]
fn test_parse_line_range_inverted_errors() {
    assert!(parse_line_range("5:3").is_err());
}

#[test]
fn test_parse_line_range_zero_errors() {
    assert!(parse_line_range("0").is_err());
    assert!(parse_line_range("0:5").is_err());
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

// ── get_comment_style ──

#[test]
fn test_get_comment_style_python() {
    let style = get_comment_style(std::path::Path::new("test.py"), "auto", None).unwrap();
    assert_eq!(style.single_line, "#");
}

#[test]
fn test_get_comment_style_javascript() {
    let style = get_comment_style(std::path::Path::new("test.js"), "auto", None).unwrap();
    assert_eq!(style.single_line, "//");
}

#[test]
fn test_get_comment_style_rust() {
    let style = get_comment_style(std::path::Path::new("test.rs"), "auto", None).unwrap();
    assert_eq!(style.single_line, "//");
}

#[test]
fn test_get_comment_style_shell() {
    let style = get_comment_style(std::path::Path::new("test.sh"), "auto", None).unwrap();
    assert_eq!(style.single_line, "#");
}

#[test]
fn test_get_comment_style_unsupported() {
    assert!(get_comment_style(std::path::Path::new("test.xyz"), "auto", None).is_err());
}

// ── Section toggle with trailing empty lines (Issue 23) ──

#[test]
fn test_section_toggle_preserves_trailing_empty_lines() {
    let style = CommentStyle {
        single_line: "#".to_string(),
        multi_line_start: None,
        multi_line_end: None,
    };
    let mut lines = vec![
        "# toggle:start ID=sec1".to_string(),
        "hello".to_string(),
        "".to_string(),
        "world".to_string(),
        "# toggle:end ID=sec1".to_string(),
    ];
    let result = find_and_toggle_section(&mut lines, "sec1", &None, &style).unwrap();
    assert!(result.modified, "section should be modified");
    // The empty line should remain empty (not be replaced by stale data)
    assert_eq!(lines[1], "# hello");
    assert_eq!(lines[2], "");
    assert_eq!(lines[3], "# world");
}

// ── supported_extensions ──

#[test]
fn test_supported_extensions_nonempty() {
    let exts = supported_extensions();
    assert!(exts.len() > 10);
    assert!(exts.contains(&"py"));
    assert!(exts.contains(&"rs"));
    assert!(exts.contains(&"js"));
}

// ── scan_sections ──

#[test]
fn test_scan_sections_finds_single_section() {
    let content =
        "# toggle:start ID=debug desc=\"Debug output\"\n# print('debug')\n# toggle:end ID=debug\n";
    let path = Path::new("test.py");
    let sections = scan_sections(path, content);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, "debug");
    assert_eq!(sections[0].start_line, 1);
    assert_eq!(sections[0].end_line, Some(3));
    assert_eq!(sections[0].description.as_deref(), Some("Debug output"));
    assert_eq!(sections[0].state, "commented");
}

#[test]
fn test_scan_sections_finds_multiple_sections() {
    let content = "\
# toggle:start ID=alpha
# commented_line
# toggle:end ID=alpha
# toggle:start ID=beta
active_line
# toggle:end ID=beta
";
    let path = Path::new("test.py");
    let sections = scan_sections(path, content);
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].id, "alpha");
    assert_eq!(sections[0].state, "commented");
    assert_eq!(sections[1].id, "beta");
    assert_eq!(sections[1].state, "uncommented");
}

#[test]
fn test_scan_sections_detects_mixed_state() {
    let content = "\
# toggle:start ID=mix
# commented
uncommented
# toggle:end ID=mix
";
    let path = Path::new("test.py");
    let sections = scan_sections(path, content);
    assert_eq!(sections[0].state, "mixed");
}

#[test]
fn test_scan_sections_unclosed_section() {
    let content = "# toggle:start ID=orphan\nsome code\n";
    let path = Path::new("test.py");
    let sections = scan_sections(path, content);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, "orphan");
    assert!(sections[0].end_line.is_none());
    assert_eq!(sections[0].state, "unknown");
}

#[test]
fn test_scan_sections_empty_section() {
    let content = "# toggle:start ID=empty\n# toggle:end ID=empty\n";
    let path = Path::new("test.py");
    let sections = scan_sections(path, content);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].state, "empty");
}

#[test]
fn test_scan_sections_javascript_comment_style() {
    let content = "\
// toggle:start ID=feature desc=\"JS feature\"
// console.log('debug');
// toggle:end ID=feature
";
    let path = Path::new("app.js");
    let sections = scan_sections(path, content);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, "feature");
    assert_eq!(sections[0].state, "commented");
}

// ── parse_id_parts ──

#[test]
fn parse_id_parts_solo() {
    assert_eq!(
        togl_lib::core::parse_id_parts("debug"),
        ("debug".to_string(), None)
    );
}

#[test]
fn parse_id_parts_variant() {
    assert_eq!(
        togl_lib::core::parse_id_parts("db:postgres"),
        ("db".to_string(), Some("postgres".to_string()))
    );
}

#[test]
fn parse_id_parts_empty_variant_treated_as_solo() {
    let (g, v) = togl_lib::core::parse_id_parts("db:");
    assert_eq!(g, "db");
    assert_eq!(v, Some("".to_string()));
}

#[test]
fn parse_id_parts_multiple_colons_uses_first() {
    assert_eq!(
        togl_lib::core::parse_id_parts("a:b:c"),
        ("a".to_string(), Some("b:c".to_string()))
    );
}

// ── discover_variants ──

const VARIANTS_FIXTURE: &str = r#"
# toggle:start ID=db:sqlite desc="SQLite backend"
import sqlite3
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres desc="Postgres backend"
# import psycopg2
# toggle:end ID=db:postgres

# toggle:start ID=debug
print("debug")
# toggle:end ID=debug
"#;

#[test]
fn discover_variants_returns_pair() {
    let v = togl_lib::core::discover_variants(VARIANTS_FIXTURE, "db");
    assert_eq!(v.len(), 2);
    let ids: Vec<&str> = v.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"db:sqlite"));
    assert!(ids.contains(&"db:postgres"));
}

#[test]
fn discover_variants_solo_only() {
    let v = togl_lib::core::discover_variants(VARIANTS_FIXTURE, "debug");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].id, "debug");
}

#[test]
fn discover_variants_no_match() {
    let v = togl_lib::core::discover_variants(VARIANTS_FIXTURE, "missing");
    assert!(v.is_empty());
}

#[test]
fn discover_variants_distinguishes_groups() {
    // Prefix collision guard: "db" must NOT match "debug".
    let v = togl_lib::core::discover_variants(VARIANTS_FIXTURE, "db");
    for s in &v {
        let (g, _) = togl_lib::core::parse_id_parts(&s.id);
        assert_eq!(g, "db");
    }
}

// ── toggle_variant_group / activate_variant ──

fn comment_style_py() -> togl_lib::core::CommentStyle {
    togl_lib::core::CommentStyle {
        single_line: "#".to_string(),
        multi_line_start: None,
        multi_line_end: None,
    }
}

#[test]
fn toggle_variant_group_pair_flip_swaps_states() {
    // Initial in fixture: db:sqlite uncommented, db:postgres commented.
    let result =
        togl_lib::core::toggle_variant_group(VARIANTS_FIXTURE, "db", &None, &comment_style_py())
            .unwrap();
    assert!(result.contains("# import sqlite3"));
    assert!(result.contains("\nimport psycopg2"));
}

#[test]
fn toggle_variant_group_force_on_comments_all() {
    let result = togl_lib::core::toggle_variant_group(
        VARIANTS_FIXTURE,
        "db",
        &Some("on".to_string()),
        &comment_style_py(),
    )
    .unwrap();
    assert!(result.contains("# import sqlite3"));
    assert!(result.contains("# import psycopg2"));
}

#[test]
fn toggle_variant_group_force_off_uncomments_all() {
    let result = togl_lib::core::toggle_variant_group(
        VARIANTS_FIXTURE,
        "db",
        &Some("off".to_string()),
        &comment_style_py(),
    )
    .unwrap();
    assert!(result.contains("\nimport sqlite3"));
    assert!(result.contains("\nimport psycopg2"));
}

#[test]
fn toggle_variant_group_errors_on_three_variants() {
    let three = r#"
# toggle:start ID=cache:redis
x = 1
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# y = 2
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# z = 3
# toggle:end ID=cache:inmemory
"#;
    let err = togl_lib::core::toggle_variant_group(three, "cache", &None, &comment_style_py())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("3 variants"), "got: {msg}");
    assert!(msg.contains("cache"));
}

#[test]
fn activate_variant_uncomments_target_and_comments_others() {
    let result =
        togl_lib::core::activate_variant(VARIANTS_FIXTURE, "db", "postgres", &comment_style_py())
            .unwrap();
    assert!(result.contains("\nimport psycopg2"));
    assert!(result.contains("# import sqlite3"));
}

#[test]
fn activate_variant_unknown_variant_errors() {
    let err =
        togl_lib::core::activate_variant(VARIANTS_FIXTURE, "db", "mysql", &comment_style_py())
            .unwrap_err();
    assert!(format!("{err}").contains("mysql"));
}

// ── summarize_scan / SectionType (PRD §0.14.1) ──

#[test]
fn summarize_scan_infers_types() {
    use togl_lib::core::SectionType;
    let content = r#"
# toggle:start ID=db:sqlite
x = 1
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# y = 2
# toggle:end ID=db:postgres

# toggle:start ID=cache:redis
z = 3
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# a = 4
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# b = 5
# toggle:end ID=cache:inmemory

# toggle:start ID=debug
c = 6
# toggle:end ID=debug
"#;
    let sections = scan_sections(Path::new("t.py"), content);
    let summary = togl_lib::core::summarize_scan(&sections);

    let by_group = |g: &str| {
        summary
            .iter()
            .find(|s| s.group == g)
            .unwrap_or_else(|| panic!("missing group {g}"))
            .clone()
    };
    assert_eq!(by_group("db").section_type, SectionType::Pair);
    assert_eq!(by_group("db").variant_count, 2);
    assert_eq!(by_group("cache").section_type, SectionType::Group);
    assert_eq!(by_group("cache").variant_count, 3);
    assert_eq!(by_group("debug").section_type, SectionType::Solo);
}

// ── ScanSectionInfo group/variant fields (PRD §0.14.1) ──

#[test]
fn scan_sections_populates_group_and_variant() {
    let content = r#"
# toggle:start ID=db:sqlite
import sqlite3
# toggle:end ID=db:sqlite

# toggle:start ID=debug
print("x")
# toggle:end ID=debug
"#;
    let sections = scan_sections(Path::new("test.py"), content);
    let sqlite = sections.iter().find(|s| s.id == "db:sqlite").unwrap();
    assert_eq!(sqlite.group, "db");
    assert_eq!(sqlite.variant.as_deref(), Some("sqlite"));

    let debug = sections.iter().find(|s| s.id == "debug").unwrap();
    assert_eq!(debug.group, "debug");
    assert_eq!(debug.variant, None);
}

// ── validate_sections (PRD §0.14.3) ──

fn scan_one(
    path: &str,
    content: &str,
) -> (std::path::PathBuf, Vec<togl_lib::core::ScanSectionInfo>) {
    let p = std::path::PathBuf::from(path);
    let v = togl_lib::core::scan_sections(&p, content);
    (p, v)
}

#[test]
fn validate_flags_unclosed_marker() {
    let (p, v) = scan_one("a.py", "# toggle:start ID=foo\nx = 1\n");
    let issues = togl_lib::core::validate_sections(&[(p, v)], false);
    assert!(
        issues.iter().any(|i| i.message.contains("unclosed")),
        "issues: {issues:?}"
    );
}

#[test]
fn validate_flags_pair_mismatch_when_pair_inferred() {
    let three = r#"
# toggle:start ID=cache:redis
x = 1
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# y = 2
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# z = 3
# toggle:end ID=cache:inmemory
"#;
    let (p, v) = scan_one("a.py", three);
    let issues = togl_lib::core::validate_sections(&[(p, v)], true);
    assert!(
        issues
            .iter()
            .any(|i| i.group == "cache" && i.message.contains("expected 2")),
        "issues: {issues:?}"
    );
}

#[test]
fn validate_flags_duplicate_id_in_file() {
    let dup = r#"
# toggle:start ID=foo
x = 1
# toggle:end ID=foo

# toggle:start ID=foo
y = 2
# toggle:end ID=foo
"#;
    let (p, v) = scan_one("a.py", dup);
    let issues = togl_lib::core::validate_sections(&[(p, v)], false);
    assert!(
        issues.iter().any(|i| i.message.contains("duplicate")),
        "issues: {issues:?}"
    );
}

#[test]
fn validate_flags_cross_file_variant_mismatch() {
    let a = r#"
# toggle:start ID=db:sqlite
x = 1
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# y = 2
# toggle:end ID=db:postgres
"#;
    let b = r#"
# toggle:start ID=db:sqlite
z = 3
# toggle:end ID=db:sqlite
"#;
    let issues =
        togl_lib::core::validate_sections(&[scan_one("a.py", a), scan_one("b.py", b)], false);
    assert!(
        issues
            .iter()
            .any(|i| i.group == "db" && i.message.contains("missing")),
        "issues: {issues:?}"
    );
}

// ── build_scan_json (PRD §0.14.4) ──

#[test]
fn build_scan_json_emits_solo_and_grouped_entries() {
    use togl_lib::core::{build_scan_json, ScanJsonEntry, SectionType};
    let content = r#"
# toggle:start ID=db:sqlite
x = 1
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres
# y = 2
# toggle:end ID=db:postgres

# toggle:start ID=debug desc="Debug output"
print("d")
# toggle:end ID=debug
"#;
    let sections = scan_sections(Path::new("src/app.py"), content);
    let root = build_scan_json(&sections);

    assert_eq!(root.sections.len(), 2);

    let debug = root
        .sections
        .iter()
        .find(|e| matches!(e, ScanJsonEntry::Solo { id, .. } if id == "debug"));
    let debug = debug.expect("debug solo entry");
    match debug {
        ScanJsonEntry::Solo {
            section_type,
            files,
            ..
        } => {
            assert_eq!(section_type, &SectionType::Solo);
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].path, "src/app.py");
            assert_eq!(files[0].state, "uncommented");
            assert_eq!(files[0].desc.as_deref(), Some("Debug output"));
        }
        _ => panic!("expected Solo"),
    }

    let db = root
        .sections
        .iter()
        .find(|e| matches!(e, ScanJsonEntry::Group { group, .. } if group == "db"));
    let db = db.expect("db group entry");
    match db {
        ScanJsonEntry::Group {
            section_type,
            variants,
            ..
        } => {
            assert_eq!(section_type, &SectionType::Pair);
            assert_eq!(variants.len(), 2);
            let sqlite = variants.iter().find(|v| v.id == "db:sqlite").unwrap();
            assert_eq!(sqlite.state, "uncommented");
            assert_eq!(sqlite.files.len(), 1);
            let postgres = variants.iter().find(|v| v.id == "db:postgres").unwrap();
            assert_eq!(postgres.state, "commented");
        }
        _ => panic!("expected Group"),
    }
}

// ── insert_section ──

#[test]
fn test_insert_section_basic() {
    let content = "a\nb\nc\nd\n";
    // Wrap lines 2..3 (1-based inclusive) with ID=feat
    let result = insert_section(content, "feat", None, 2, 3, "#").unwrap();
    assert_eq!(
        result,
        "a\n# toggle:start ID=feat\nb\nc\n# toggle:end ID=feat\nd\n"
    );
}

#[test]
fn test_insert_section_with_desc() {
    let content = "a\nb\n";
    let result = insert_section(content, "feat", Some("hello world"), 1, 2, "//").unwrap();
    assert_eq!(
        result,
        "// toggle:start ID=feat desc=\"hello world\"\na\nb\n// toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_section_matches_indentation() {
    let content = "def f():\n    x = 1\n    y = 2\n";
    // Wrap the two indented lines (2..3)
    let result = insert_section(content, "feat", None, 2, 3, "#").unwrap();
    assert_eq!(
        result,
        "def f():\n    # toggle:start ID=feat\n    x = 1\n    y = 2\n    # toggle:end ID=feat\n"
    );
}

#[test]
fn test_insert_section_no_trailing_newline() {
    let content = "a\nb"; // no trailing newline
    let result = insert_section(content, "feat", None, 1, 2, "#").unwrap();
    assert_eq!(result, "# toggle:start ID=feat\na\nb\n# toggle:end ID=feat");
}

#[test]
fn test_insert_section_rejects_duplicate_id() {
    let content = "# toggle:start ID=feat\nx\n# toggle:end ID=feat\ny\n";
    let err = insert_section(content, "feat", None, 4, 4, "#");
    assert!(err.is_err());
}

#[test]
fn test_insert_section_rejects_out_of_bounds() {
    let content = "a\nb\n";
    assert!(insert_section(content, "feat", None, 1, 5, "#").is_err());
}

#[test]
fn test_insert_section_rejects_bad_id() {
    let content = "a\nb\n";
    assert!(insert_section(content, "a b", None, 1, 2, "#").is_err());
    assert!(insert_section(content, "", None, 1, 2, "#").is_err());
}

#[test]
fn test_insert_section_rejects_quote_in_desc() {
    let content = "a\nb\n";
    assert!(insert_section(content, "feat", Some("has \" quote"), 1, 2, "#").is_err());
}

#[test]
fn test_insert_section_rejects_inverted_range() {
    let content = "a\nb\nc\n";
    assert!(insert_section(content, "feat", None, 3, 1, "#").is_err());
}
