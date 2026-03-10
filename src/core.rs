// Toggle algorithm implementation

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use crate::config::ToggleConfig;
use crate::exit_codes::UsageError;

/// A discovered section marker with metadata.
#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub id: String,
    pub desc: Option<String>,
    pub start_line: usize, // 1-based
    pub end_line: usize,   // 1-based
}

/// Result of toggling a section, including parsed metadata.
pub struct SectionToggleResult {
    pub modified: bool,
    pub desc: Option<String>,
}

/// Extract the `desc="..."` value from a section marker line.
fn parse_section_desc(line: &str) -> Option<String> {
    let marker = "desc=\"";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract the section ID as a whitespace-delimited token after `ID=`.
fn parse_section_id(line: &str) -> Option<String> {
    let marker = "ID=";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let id = &rest[..end];
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Check if a line contains a `toggle:start` marker with an exact section ID match.
fn line_matches_start(line: &str, section_id: &str) -> bool {
    if !line.contains("toggle:start") {
        return false;
    }
    parse_section_id(line).as_deref() == Some(section_id)
}

/// Check if a line contains a `toggle:end` marker with an exact section ID match.
fn line_matches_end(line: &str, section_id: &str) -> bool {
    if !line.contains("toggle:end") {
        return false;
    }
    parse_section_id(line).as_deref() == Some(section_id)
}

/// Scan file content for all section marker pairs and return their metadata.
/// Unclosed sections are silently skipped (useful for discovery across many files).
pub fn discover_sections(content: &str) -> Vec<SectionInfo> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].contains("toggle:start") {
            if let Some(id) = parse_section_id(lines[i]) {
                let desc = parse_section_desc(lines[i]);
                let start_line = i + 1; // 1-based

                // Find matching end marker
                let mut end_line = None;
                for j in (i + 1)..lines.len() {
                    if line_matches_end(lines[j], &id) {
                        end_line = Some(j + 1); // 1-based
                        break;
                    }
                }

                if let Some(end_line) = end_line {
                    sections.push(SectionInfo {
                        id,
                        desc,
                        start_line,
                        end_line,
                    });
                    i = end_line; // skip past this section (end_line is 1-based, i is 0-based)
                    continue;
                }
            }
        }
        i += 1;
    }

    sections
}

/// Line range representation
#[derive(Debug, Clone)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    /// Create a new line range
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Comment style for a language
#[derive(Debug, Clone)]
pub struct CommentStyle {
    pub single_line: String,
    pub multi_line_start: Option<String>,
    pub multi_line_end: Option<String>,
}

/// Parse a line range specification.
/// Supports formats: "start:end", "start:+count", "single_line"
pub fn parse_line_range(range_spec: &str) -> Result<(usize, usize)> {
    if let Some((start, end)) = range_spec.split_once(':') {
        let start_line = start
            .parse::<usize>()
            .map_err(|_| UsageError(format!("Invalid start line: {}", start)))?;

        if start_line == 0 {
            return Err(UsageError("Start line must be >= 1, got 0".into()).into());
        }

        if let Some(stripped_end) = end.strip_prefix('+') {
            // Format: start:+count
            let count = stripped_end
                .parse::<usize>()
                .map_err(|_| UsageError(format!("Invalid line count: {}", stripped_end)))?;
            Ok((start_line, start_line + count))
        } else {
            // Format: start:end
            let end_line = end
                .parse::<usize>()
                .map_err(|_| UsageError(format!("Invalid end line: {}", end)))?;
            if end_line < start_line {
                return Err(UsageError(format!(
                    "End line {} is less than start line {}",
                    end_line, start_line
                ))
                .into());
            }
            Ok((start_line, end_line))
        }
    } else {
        // Single line
        let line = range_spec
            .parse::<usize>()
            .map_err(|_| UsageError(format!("Invalid line number: {}", range_spec)))?;
        if line == 0 {
            return Err(UsageError("Line number must be >= 1, got 0".into()).into());
        }
        Ok((line, line))
    }
}

/// Merge multiple line ranges into a minimal list of non-overlapping ranges.
/// Sorts ascending by start, then coalesces overlapping/adjacent intervals.
pub fn merge_ranges(ranges: &[LineRange]) -> Vec<LineRange> {
    if ranges.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<LineRange> = ranges.to_vec();
    sorted.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));

    let mut merged = vec![sorted[0].clone()];

    for range in &sorted[1..] {
        let last = merged.last_mut().unwrap();
        if range.start <= last.end + 1 {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range.clone());
        }
    }

    merged
}

/// Toggle comments in the specified line ranges.
/// `marker`: comment prefix (e.g. `"#"`, `"//"`, `"--"`). Defaults to `"#"` if `None`.
/// `force_mode`: `Some("on")` = always comment, `Some("off")` = always uncomment, `None` = invert.
pub fn toggle_comments(content: &str, ranges: &[LineRange], force_mode: Option<&str>) -> String {
    toggle_comments_with_marker(content, ranges, force_mode, "#")
}

/// Toggle comments with an explicit comment marker.
pub fn toggle_comments_with_marker(
    content: &str,
    ranges: &[LineRange],
    force_mode: Option<&str>,
    marker: &str,
) -> String {
    let protected = crate::io::detect_protected_lines(content);
    toggle_comments_inner(content, ranges, force_mode, marker, &protected)
}

/// Toggle comments with explicit protected lines (empty vec to skip protection).
fn toggle_comments_inner(
    content: &str,
    ranges: &[LineRange],
    force_mode: Option<&str>,
    marker: &str,
    protected: &[usize],
) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let merged = merge_ranges(ranges);

    for range in &merged {
        // Convert 1-based inclusive range to 0-based indices
        let start = range.start.saturating_sub(1);
        let end = range.end.min(lines.len());

        if start >= lines.len() {
            continue;
        }

        // Determine current state for invert mode: check if majority of
        // non-empty, non-protected lines are commented
        let should_comment = match force_mode {
            Some("on") => true,
            Some("off") => false,
            _ => {
                // Invert: check if lines are currently commented
                let commented_count = lines[start..end]
                    .iter()
                    .enumerate()
                    .filter(|(i, line)| {
                        let abs_idx = start + i;
                        !protected.contains(&abs_idx) && !line.trim().is_empty()
                    })
                    .filter(|(_, line)| {
                        let trimmed = line.trim_start();
                        trimmed.starts_with(marker)
                    })
                    .count();
                let total_non_empty = lines[start..end]
                    .iter()
                    .enumerate()
                    .filter(|(i, line)| {
                        let abs_idx = start + i;
                        !protected.contains(&abs_idx) && !line.trim().is_empty()
                    })
                    .count();
                // If all non-empty lines are commented, uncomment (false); otherwise comment (true)
                !(commented_count > 0 && commented_count == total_non_empty)
            }
        };

        #[allow(clippy::needless_range_loop)]
        for idx in start..end {
            if protected.contains(&idx) {
                continue;
            }

            let line = &lines[idx];
            if line.trim().is_empty() {
                continue;
            }

            let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            let rest = &line[leading_ws.len()..];

            if should_comment {
                // Strip existing comment marker first to avoid double-commenting
                let marker_space = format!("{} ", marker);
                let stripped = if let Some(s) = rest.strip_prefix(&marker_space) {
                    s
                } else if let Some(s) = rest.strip_prefix(marker) {
                    s
                } else {
                    rest
                };
                lines[idx] = format!("{}{} {}", leading_ws, marker, stripped);
            } else {
                // Uncomment: remove marker and optional following space
                let marker_space = format!("{} ", marker);
                if let Some(s) = rest.strip_prefix(&marker_space) {
                    lines[idx] = format!("{}{}", leading_ws, s);
                } else if let Some(s) = rest.strip_prefix(marker) {
                    lines[idx] = format!("{}{}", leading_ws, s);
                }
            }
        }
    }

    // Preserve trailing newline if original had one
    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Toggle comments using multi-line/block comment delimiters.
/// For each merged range, wraps the content in start/end delimiters (commenting)
/// or strips them (uncommenting). Force mode works the same as single-line.
pub fn toggle_comments_multi(
    content: &str,
    ranges: &[LineRange],
    force_mode: Option<&str>,
    start_delim: &str,
    end_delim: &str,
) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let merged = merge_ranges(ranges);

    for range in &merged {
        let start = range.start.saturating_sub(1);
        let end = range.end.min(lines.len());

        if start >= lines.len() || start >= end {
            continue;
        }

        // Detect if the range is already block-commented:
        // first non-blank line starts with start_delim, last non-blank line ends with end_delim
        let first_trimmed = lines[start].trim_start();
        let last_trimmed = lines[end - 1].trim_end();
        let is_commented =
            first_trimmed.starts_with(start_delim) && last_trimmed.ends_with(end_delim);

        let should_comment = match force_mode {
            Some("on") => true,
            Some("off") => false,
            _ => !is_commented,
        };

        if should_comment && !is_commented {
            // Wrap: prepend start_delim to first line, append end_delim to last line
            let first_line = &lines[start];
            let leading_ws: String = first_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            let rest = &first_line[leading_ws.len()..];
            lines[start] = format!("{}{} {}", leading_ws, start_delim, rest);

            let last_line = &lines[end - 1];
            lines[end - 1] = format!("{} {}", last_line, end_delim);
        } else if !should_comment && is_commented {
            // Unwrap: strip start_delim from first line, strip end_delim from last line
            let first_line = &lines[start];
            let leading_ws: String = first_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            let rest = &first_line[leading_ws.len()..];
            let stripped_start = if let Some(s) = rest.strip_prefix(start_delim) {
                let s = s.strip_prefix(' ').unwrap_or(s);
                format!("{}{}", leading_ws, s)
            } else {
                first_line.clone()
            };
            lines[start] = stripped_start;

            let last_line = &lines[end - 1];
            let stripped_end = if let Some(s) = last_line.strip_suffix(end_delim) {
                let s = s.strip_suffix(' ').unwrap_or(s);
                s.to_string()
            } else {
                last_line.clone()
            };
            lines[end - 1] = stripped_end;
        }
    }

    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Map file extension to language name for config lookup
fn ext_to_language(ext: &str) -> &str {
    match ext {
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "rs" => "rust",
        "rb" => "ruby",
        "sh" => "shell",
        "yaml" | "yml" => "yaml",
        "r" => "r",
        "ex" | "exs" => "elixir",
        "pl" | "pm" => "perl",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "go" => "go",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "php" => "php",
        "lua" => "lua",
        "hs" => "haskell",
        "sql" => "sql",
        "toml" => "toml",
        other => other,
    }
}

/// Get the comment style for a file based on its extension.
/// If a config is provided, language-specific overrides take priority,
/// then global overrides, then the hardcoded defaults.
pub fn get_comment_style(
    path: &Path,
    _mode: &str,
    config: Option<&ToggleConfig>,
) -> Result<CommentStyle> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    // Check config overrides first
    if let Some(cfg) = config {
        let lang = ext_to_language(extension);
        // Language-specific override
        if let Some(delimiter) = cfg.get_language_delimiter(lang) {
            let multi = cfg.get_language_multi_line_delimiters(lang);
            return Ok(CommentStyle {
                single_line: delimiter.to_string(),
                multi_line_start: multi.map(|(s, _)| s.to_string()),
                multi_line_end: multi.map(|(_, e)| e.to_string()),
            });
        }
        // Global override
        if let Some(delimiter) = cfg
            .global
            .as_ref()
            .and_then(|g| g.single_line_delimiter.as_deref())
        {
            let global = cfg.global.as_ref();
            return Ok(CommentStyle {
                single_line: delimiter.to_string(),
                multi_line_start: global
                    .and_then(|g| g.multi_line_delimiter_start.as_deref())
                    .map(String::from),
                multi_line_end: global
                    .and_then(|g| g.multi_line_delimiter_end.as_deref())
                    .map(String::from),
            });
        }
    }

    let mut comment_styles = HashMap::new();
    // Hash-style comments (no multi-line)
    for ext in &[
        "py", "sh", "rb", "yaml", "yml", "toml", "r", "ex", "exs", "pl", "pm",
    ] {
        comment_styles.insert(
            *ext,
            CommentStyle {
                single_line: "#".to_string(),
                multi_line_start: None,
                multi_line_end: None,
            },
        );
    }
    // Slash-style comments with /* */ multi-line
    for ext in &[
        "js", "jsx", "ts", "tsx", "rs", "java", "c", "cpp", "go", "swift", "kt", "scala", "php",
    ] {
        comment_styles.insert(
            *ext,
            CommentStyle {
                single_line: "//".to_string(),
                multi_line_start: Some("/*".to_string()),
                multi_line_end: Some("*/".to_string()),
            },
        );
    }
    // Dash-style comments
    comment_styles.insert(
        "lua",
        CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("--[[".to_string()),
            multi_line_end: Some("]]".to_string()),
        },
    );
    comment_styles.insert(
        "hs",
        CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("{-".to_string()),
            multi_line_end: Some("-}".to_string()),
        },
    );
    comment_styles.insert(
        "sql",
        CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("/*".to_string()),
            multi_line_end: Some("*/".to_string()),
        },
    );

    comment_styles
        .get(extension)
        .cloned()
        .ok_or_else(|| UsageError(format!("Unsupported file extension: .{}", extension)).into())
}

/// Find section markers and toggle the content between them.
/// Returns a `SectionToggleResult` with modification status and parsed desc.
pub fn find_and_toggle_section(
    lines: &mut [String],
    section_id: &str,
    force: &Option<String>,
    comment_style: &CommentStyle,
) -> Result<SectionToggleResult> {
    let mut i = 0;
    let mut modified = false;
    let mut desc = None;

    while i < lines.len() {
        if line_matches_start(&lines[i], section_id) {
            if desc.is_none() {
                desc = parse_section_desc(&lines[i]);
            }
            let section_start = i + 1;

            let mut section_end = None;

            for (j, line) in lines.iter().enumerate().skip(i + 1) {
                if line_matches_end(line, section_id) {
                    section_end = Some(j);
                    break;
                }
            }

            let section_end = match section_end {
                Some(end) => end,
                None => {
                    return Err(UsageError(format!("Unclosed section ID={}", section_id)).into());
                }
            };

            if section_end > section_start {
                let force_mode = force.as_deref();

                // Build content string from section lines and toggle via
                // toggle_comments_with_marker for consistent behavior (skip blanks,
                // preserve indentation)
                let section_content = lines[section_start..section_end].join("\n");
                let range = LineRange::new(1, section_end - section_start);
                // Pass empty protected set — section content is user-specified
                // and should not have false shebang/pragma detection
                let toggled = toggle_comments_inner(
                    &section_content,
                    &[range],
                    force_mode,
                    &comment_style.single_line,
                    &[],
                );

                // Splice toggled lines back in.
                // Use split('\n') instead of lines() to preserve trailing empty
                // elements that lines() would drop (lossy roundtrip fix).
                let mut toggled_lines: Vec<&str> = toggled.split('\n').collect();
                // toggle_comments_inner appends '\n' when input ends with '\n',
                // which produces a spurious trailing empty element via split.
                if toggled_lines.last() == Some(&"") && toggled.ends_with('\n') {
                    toggled_lines.pop();
                }
                let section_len = section_end - section_start;
                assert_eq!(
                    toggled_lines.len(),
                    section_len,
                    "Toggled line count ({}) must match section span ({})",
                    toggled_lines.len(),
                    section_len,
                );
                for (offset, new_line) in toggled_lines.iter().enumerate() {
                    if offset < section_len {
                        lines[section_start + offset] = (*new_line).to_string();
                    }
                }

                modified = true;
                i = section_end;
            }
        }

        i += 1;
    }

    Ok(SectionToggleResult { modified, desc })
}
