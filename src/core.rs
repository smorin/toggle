// Toggle algorithm implementation

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

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
}

/// Parse a line range specification.
/// Supports formats: "start:end", "start:+count", "single_line"
pub fn parse_line_range(range_spec: &str) -> Result<(usize, usize)> {
    if let Some((start, end)) = range_spec.split_once(':') {
        let start_line = start
            .parse::<usize>()
            .map_err(|_| anyhow!("Invalid start line: {}", start))?;

        if let Some(stripped_end) = end.strip_prefix('+') {
            // Format: start:+count
            let count = stripped_end
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid line count: {}", stripped_end))?;
            Ok((start_line, start_line + count))
        } else {
            // Format: start:end
            let end_line = end
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid end line: {}", end))?;
            Ok((start_line, end_line))
        }
    } else {
        // Single line
        let line = range_spec
            .parse::<usize>()
            .map_err(|_| anyhow!("Invalid line number: {}", range_spec))?;
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
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let protected = crate::io::detect_protected_lines(content);
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
                // Comment: insert marker + space at first non-whitespace
                lines[idx] = format!("{}{} {}", leading_ws, marker, rest);
            } else {
                // Uncomment: remove marker and optional following space
                let marker_space = format!("{} ", marker);
                if rest.starts_with(&marker_space) {
                    lines[idx] = format!("{}{}", leading_ws, &rest[marker_space.len()..]);
                } else if rest.starts_with(marker) {
                    lines[idx] = format!("{}{}", leading_ws, &rest[marker.len()..]);
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

/// Get the comment style for a file based on its extension
pub fn get_comment_style(path: &Path, _mode: &str) -> Result<CommentStyle> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let mut comment_styles = HashMap::new();
    // Hash-style comments
    for ext in &["py", "sh", "rb", "yaml", "yml", "toml", "r", "ex", "exs", "pl", "pm"] {
        comment_styles.insert(*ext, CommentStyle { single_line: "#".to_string() });
    }
    // Slash-style comments
    for ext in &["js", "jsx", "ts", "tsx", "rs", "java", "c", "cpp", "go", "swift", "kt", "scala", "php"] {
        comment_styles.insert(*ext, CommentStyle { single_line: "//".to_string() });
    }
    // Dash-style comments
    for ext in &["lua", "hs", "sql"] {
        comment_styles.insert(*ext, CommentStyle { single_line: "--".to_string() });
    }

    comment_styles
        .get(extension)
        .cloned()
        .ok_or_else(|| anyhow!("Unsupported file extension: .{}", extension))
}

/// Check if lines are already commented
pub fn check_if_commented(lines: &[String], comment_style: &CommentStyle) -> bool {
    for line in lines {
        let trimmed_line = line.trim_start();
        if !trimmed_line.is_empty() {
            return trimmed_line.starts_with(&comment_style.single_line);
        }
    }
    false
}

/// Toggle comment state on a slice of lines
pub fn toggle_lines(
    lines: &mut [String],
    start: usize,
    end: usize,
    force_state: Option<bool>,
    comment_style: &CommentStyle,
) -> Result<()> {
    let is_commented = check_if_commented(&lines[start..end], comment_style);

    let should_comment = match force_state {
        Some(true) => true,
        Some(false) => false,
        None => !is_commented,
    };

    if should_comment {
        if is_commented {
            for line in lines[start..end].iter_mut() {
                if line.starts_with(&format!("{} ", comment_style.single_line)) {
                    *line = line[comment_style.single_line.len() + 1..].to_string();
                } else if line.starts_with(&comment_style.single_line) {
                    *line = line[comment_style.single_line.len()..].to_string();
                }
            }
        }

        for line in lines[start..end].iter_mut() {
            *line = format!("{}{}", comment_style.single_line, line);
        }
    } else if !should_comment && is_commented {
        let prefix = format!("{} ", comment_style.single_line);
        let prefix_len = prefix.len();
        for line in lines[start..end].iter_mut() {
            if line.starts_with(&prefix) {
                *line = line[prefix_len..].to_string();
            } else if line.starts_with(&comment_style.single_line) {
                *line = line[comment_style.single_line.len()..].to_string();
            }
        }
    }

    Ok(())
}

/// Find section markers and toggle the content between them.
/// Returns true if the file was modified.
pub fn find_and_toggle_section(
    lines: &mut Vec<String>,
    section_id: &str,
    force: &Option<String>,
    comment_style: &CommentStyle,
) -> Result<bool> {
    let mut i = 0;
    let mut modified = false;

    while i < lines.len() {
        let start_marker = format!("toggle:start ID={}", section_id);

        if lines[i].contains(&start_marker) {
            let section_start = i + 1;

            let end_marker = format!("toggle:end ID={}", section_id);
            let mut section_end = lines.len();

            for (j, line) in lines.iter().enumerate().skip(i + 1) {
                if line.contains(&end_marker) {
                    section_end = j;
                    break;
                }
            }

            if section_end > section_start {
                let force_state = match force {
                    Some(force_str) if force_str == "on" => Some(true),
                    Some(force_str) if force_str == "off" => Some(false),
                    _ => None,
                };

                toggle_lines(lines, section_start, section_end, force_state, comment_style)?;
                modified = true;

                i = section_end;
            }
        }

        i += 1;
    }

    Ok(modified)
}
