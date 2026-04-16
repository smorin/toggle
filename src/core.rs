// Toggle algorithm implementation

use anyhow::Result;
use std::path::Path;

use crate::config::ToggleConfig;
use crate::exit_codes::UsageError;

/// Returns the list of file extensions that toggle knows how to handle.
pub fn supported_extensions() -> &'static [&'static str] {
    &[
        "py", "sh", "rb", "yaml", "yml", "toml", "r", "ex", "exs", "pl", "pm", "js", "jsx", "ts",
        "tsx", "rs", "java", "c", "cpp", "go", "swift", "kt", "scala", "php", "lua", "hs", "sql",
    ]
}

/// A discovered section marker with metadata (used by discover_sections and find_and_toggle_section).
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

/// Information about a discovered toggle section for scan output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanSectionInfo {
    pub id: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub file: String,
    pub start_line: usize,
    pub end_line: Option<usize>,
    pub description: Option<String>,
    pub state: String,
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

/// Split a section ID into `(group, variant)` parts using the first `:` as separator.
/// Solo IDs (no colon) return `(id, None)`; variant IDs return `(group, Some(variant))`.
pub fn parse_id_parts(id: &str) -> (String, Option<String>) {
    match id.split_once(':') {
        Some((g, v)) => (g.to_string(), Some(v.to_string())),
        None => (id.to_string(), None),
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
                #[allow(clippy::needless_range_loop)]
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

/// Return all `SectionInfo` whose ID parses into the given group.
/// `discover_variants(content, "db")` matches both `db` (solo) and `db:postgres` (variant).
pub fn discover_variants(content: &str, group: &str) -> Vec<SectionInfo> {
    discover_sections(content)
        .into_iter()
        .filter(|s| parse_id_parts(&s.id).0 == group)
        .collect()
}

/// Scan file content for toggle:start / toggle:end markers.
/// Returns all sections found with state info. Does not modify anything.
pub fn scan_sections(path: &Path, content: &str) -> Vec<ScanSectionInfo> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();
    let file_str = path.display().to_string();

    // Determine comment style for state detection
    let comment_marker = get_comment_style(path, "auto", None)
        .map(|cs| cs.single_line)
        .unwrap_or_else(|_| "#".to_string());

    let mut i = 0;
    while i < lines.len() {
        if let Some(_pos) = lines[i].find("toggle:start ID=") {
            let id = parse_section_id(lines[i]).unwrap_or_default();
            if id.is_empty() {
                i += 1;
                continue;
            }

            // Extract optional description
            let description = parse_section_desc(lines[i]);

            let start_line = i + 1; // 1-based

            // Find matching end marker
            let mut end_line = None;
            #[allow(clippy::needless_range_loop)]
            for j in (i + 1)..lines.len() {
                if line_matches_end(lines[j], &id) {
                    end_line = Some(j + 1); // 1-based
                    break;
                }
            }

            // Determine state of content between markers
            let state = if let Some(end) = end_line {
                let content_start = i + 1;
                let content_end = end - 1; // back to 0-based for the end marker line
                detect_section_state(&lines[content_start..content_end], &comment_marker)
            } else {
                "unknown".to_string()
            };

            let (group, variant) = parse_id_parts(&id);
            sections.push(ScanSectionInfo {
                id,
                group,
                variant,
                file: file_str.clone(),
                start_line,
                end_line,
                description,
                state,
            });

            if let Some(end) = end_line {
                i = end;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    sections
}

/// Detect whether section content is commented, uncommented, or mixed.
fn detect_section_state(lines: &[&str], comment_marker: &str) -> String {
    let non_empty: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if non_empty.is_empty() {
        return "empty".to_string();
    }

    let commented_count = non_empty
        .iter()
        .filter(|l| l.trim_start().starts_with(comment_marker))
        .count();

    if commented_count == non_empty.len() {
        "commented".to_string()
    } else if commented_count == 0 {
        "uncommented".to_string()
    } else {
        "mixed".to_string()
    }
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

    match extension {
        // Hash-style comments (no multi-line)
        "py" | "sh" | "rb" | "yaml" | "yml" | "toml" | "r" | "ex" | "exs" | "pl" | "pm" => {
            Ok(CommentStyle {
                single_line: "#".to_string(),
                multi_line_start: None,
                multi_line_end: None,
            })
        }
        // Slash-style comments with /* */ multi-line
        "js" | "jsx" | "ts" | "tsx" | "rs" | "java" | "c" | "cpp" | "go" | "swift" | "kt"
        | "scala" | "php" => Ok(CommentStyle {
            single_line: "//".to_string(),
            multi_line_start: Some("/*".to_string()),
            multi_line_end: Some("*/".to_string()),
        }),
        // Dash-style comments
        "lua" => Ok(CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("--[[".to_string()),
            multi_line_end: Some("]]".to_string()),
        }),
        "hs" => Ok(CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("{-".to_string()),
            multi_line_end: Some("-}".to_string()),
        }),
        "sql" => Ok(CommentStyle {
            single_line: "--".to_string(),
            multi_line_start: Some("/*".to_string()),
            multi_line_end: Some("*/".to_string()),
        }),
        _ => Err(UsageError(format!(
            "Unsupported file extension: .{}; use --comment-style or --config with a [global] single_line_delimiter",
            extension
        ))
        .into()),
    }
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

/// Toggle every variant of `group` in `content`.
/// - `force = None` and exactly 2 variants → pair-flip (each variant inverted).
/// - `force = None` and 1 variant → solo invert (existing per-section behavior).
/// - `force = None` and 3+ variants → error per PRD §0.13.3.
/// - `force = Some("on" | "off")` → apply force to every variant regardless of count.
pub fn toggle_variant_group(
    content: &str,
    group: &str,
    force: &Option<String>,
    comment_style: &CommentStyle,
) -> Result<String> {
    let variants = discover_variants(content, group);
    if variants.is_empty() {
        return Err(UsageError(format!("no section or group '{group}' found")).into());
    }
    if force.is_none() && variants.len() >= 3 {
        return Err(UsageError(format!(
            "group '{group}' has {} variants; specify one with -S {group}:<name>",
            variants.len()
        ))
        .into());
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    for v in &variants {
        find_and_toggle_section(&mut lines, &v.id, force, comment_style)?;
    }

    let mut joined = lines.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}

/// Inferred type of a section group (PRD §0.14.1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionType {
    Solo,
    Pair,
    Group,
}

/// Per-group summary across one or more files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupSummary {
    pub group: String,
    pub section_type: SectionType,
    pub variant_count: usize,
    pub file_count: usize,
    pub state: String,
    pub variants: Vec<String>,
}

/// Group flat scan results into per-group summaries with inferred type.
pub fn summarize_scan(sections: &[ScanSectionInfo]) -> Vec<GroupSummary> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut groups: BTreeMap<String, Vec<&ScanSectionInfo>> = BTreeMap::new();
    for s in sections {
        groups.entry(s.group.clone()).or_default().push(s);
    }

    groups
        .into_iter()
        .map(|(group, items)| {
            let mut variants: Vec<String> =
                items.iter().filter_map(|s| s.variant.clone()).collect();
            variants.sort();
            variants.dedup();

            let section_type = match variants.len() {
                0 | 1 => SectionType::Solo,
                2 => SectionType::Pair,
                _ => SectionType::Group,
            };

            let files: BTreeSet<&String> = items.iter().map(|s| &s.file).collect();
            let states: BTreeSet<&String> = items.iter().map(|s| &s.state).collect();
            let state = if states.len() == 1 {
                states.into_iter().next().unwrap().clone()
            } else {
                "mixed".to_string()
            };

            GroupSummary {
                group,
                section_type,
                variant_count: variants.len(),
                file_count: files.len(),
                state,
                variants,
            }
        })
        .collect()
}

/// JSON file reference for `--scan --json` output (PRD §0.14.4).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanJsonFile {
    pub path: String,
    pub start: usize,
    pub end: Option<usize>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
}

/// One variant inside a pair/group entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanJsonVariant {
    pub id: String,
    pub state: String,
    pub files: Vec<ScanJsonFile>,
}

/// One top-level entry in the scan JSON tree (solo or grouped).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum ScanJsonEntry {
    Solo {
        id: String,
        #[serde(rename = "type")]
        section_type: SectionType,
        files: Vec<ScanJsonFile>,
    },
    Group {
        group: String,
        #[serde(rename = "type")]
        section_type: SectionType,
        variants: Vec<ScanJsonVariant>,
    },
}

/// Root of the nested scan JSON tree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanJsonRoot {
    pub sections: Vec<ScanJsonEntry>,
}

/// Build the nested scan JSON tree from flat scan rows (PRD §0.14.4).
pub fn build_scan_json(sections: &[ScanSectionInfo]) -> ScanJsonRoot {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&ScanSectionInfo>> = BTreeMap::new();
    for s in sections {
        groups.entry(s.group.clone()).or_default().push(s);
    }

    let mut entries = Vec::new();
    for (group, items) in groups {
        let mut variant_ids: Vec<String> = items.iter().filter_map(|s| s.variant.clone()).collect();
        variant_ids.sort();
        variant_ids.dedup();

        let section_type = match variant_ids.len() {
            0 | 1 => SectionType::Solo,
            2 => SectionType::Pair,
            _ => SectionType::Group,
        };

        if matches!(section_type, SectionType::Solo) {
            let files = items
                .iter()
                .map(|s| ScanJsonFile {
                    path: s.file.clone(),
                    start: s.start_line,
                    end: s.end_line,
                    state: s.state.clone(),
                    desc: s.description.clone(),
                })
                .collect();
            entries.push(ScanJsonEntry::Solo {
                id: group,
                section_type,
                files,
            });
        } else {
            let mut by_id: BTreeMap<String, Vec<&ScanSectionInfo>> = BTreeMap::new();
            for s in &items {
                by_id.entry(s.id.clone()).or_default().push(s);
            }
            let variants = by_id
                .into_iter()
                .map(|(id, recs)| {
                    let state = recs[0].state.clone();
                    let files = recs
                        .iter()
                        .map(|s| ScanJsonFile {
                            path: s.file.clone(),
                            start: s.start_line,
                            end: s.end_line,
                            state: s.state.clone(),
                            desc: s.description.clone(),
                        })
                        .collect();
                    ScanJsonVariant { id, state, files }
                })
                .collect();
            entries.push(ScanJsonEntry::Group {
                group,
                section_type,
                variants,
            });
        }
    }

    ScanJsonRoot { sections: entries }
}

/// Activate `group:variant`: uncomment that variant, comment every other variant of the group.
pub fn activate_variant(
    content: &str,
    group: &str,
    variant: &str,
    comment_style: &CommentStyle,
) -> Result<String> {
    let target_id = format!("{group}:{variant}");
    let variants = discover_variants(content, group);
    if !variants.iter().any(|s| s.id == target_id) {
        return Err(UsageError(format!("variant '{target_id}' not found")).into());
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    for v in &variants {
        let force = if v.id == target_id {
            Some("off".to_string())
        } else {
            Some("on".to_string())
        };
        find_and_toggle_section(&mut lines, &v.id, &force, comment_style)?;
    }

    let mut joined = lines.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}
