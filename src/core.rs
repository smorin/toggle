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
        Ok((line, line + 1))
    }
}

/// Merge multiple line ranges into a minimal list of non-overlapping ranges
pub fn merge_ranges(ranges: &[LineRange]) -> Vec<LineRange> {
    // Placeholder for range merging algorithm
    // Will implement the actual algorithm in a future task
    let _ = ranges;
    Vec::new()
}

/// Toggle comments in the specified line ranges
pub fn toggle_comments(content: &str, ranges: &[LineRange], force_mode: Option<&str>) -> String {
    // Placeholder for comment toggling logic
    // Will implement the actual algorithm in a future task
    let _ = (ranges, force_mode);
    content.to_string()
}

/// Get the comment style for a file based on its extension
pub fn get_comment_style(path: &Path, _mode: &str) -> Result<CommentStyle> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let mut comment_styles = HashMap::new();
    comment_styles.insert("py", CommentStyle { single_line: "#".to_string() });
    comment_styles.insert("js", CommentStyle { single_line: "//".to_string() });
    comment_styles.insert("rs", CommentStyle { single_line: "//".to_string() });
    comment_styles.insert("java", CommentStyle { single_line: "//".to_string() });
    comment_styles.insert("c", CommentStyle { single_line: "//".to_string() });
    comment_styles.insert("cpp", CommentStyle { single_line: "//".to_string() });
    comment_styles.insert("sh", CommentStyle { single_line: "#".to_string() });
    comment_styles.insert("rb", CommentStyle { single_line: "#".to_string() });

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
    println!(
        "  Current section state: {}",
        if is_commented { "commented" } else { "uncommented" }
    );

    let should_comment = match force_state {
        Some(true) => true,
        Some(false) => false,
        None => !is_commented,
    };

    println!(
        "  Will {}",
        if should_comment { "comment" } else { "uncomment" }
    );

    if should_comment {
        if is_commented {
            for line in lines[start..end].iter_mut() {
                if line.starts_with(&format!("{} ", comment_style.single_line)) {
                    *line = line[comment_style.single_line.len() + 1..].to_string();
                } else if line.starts_with(&comment_style.single_line) {
                    *line = line[comment_style.single_line.len()..].to_string();
                }
            }
            println!("  Uncommented first to avoid double-commenting");
        }

        for line in lines[start..end].iter_mut() {
            *line = format!("{}{}", comment_style.single_line, line);
        }
        println!("  Commented lines {}-{}", start + 1, end);
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
        println!("  Uncommented lines {}-{}", start + 1, end);
    } else {
        println!("  No changes needed (already in desired state)");
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
            println!("  Found start marker at line {}: {}", i + 1, lines[i]);
            let section_start = i + 1;

            let end_marker = format!("toggle:end ID={}", section_id);
            let mut section_end = lines.len();

            for (j, line) in lines.iter().enumerate().skip(i + 1) {
                if line.contains(&end_marker) {
                    section_end = j;
                    println!("  Found end marker at line {}: {}", j + 1, line);
                    break;
                }
            }

            if section_end > section_start {
                println!(
                    "  Section spans content lines {}-{} (excluding markers)",
                    section_start + 1,
                    section_end
                );

                for (index_in_slice, line) in lines[section_start..section_end].iter().enumerate() {
                    let original_line_index = section_start + index_in_slice;
                    println!("  Content line {}: '{}'", original_line_index + 1, line);
                }

                let force_state = match force {
                    Some(force_str) if force_str == "on" => Some(true),
                    Some(force_str) if force_str == "off" => Some(false),
                    _ => None,
                };

                toggle_lines(lines, section_start, section_end, force_state, comment_style)?;
                modified = true;

                i = section_end;
            } else {
                println!(
                    "  Warning: Could not find end marker for section {}",
                    section_id
                );
            }
        }

        i += 1;
    }

    Ok(modified)
}
