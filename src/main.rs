use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// File or directory paths to process
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Line range in format <start_line>:<end_line> or <start_line>:+<count>
    #[arg(short = 'l', long = "line")]
    line: Option<String>,

    /// Section ID to toggle
    #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
    sections: Vec<String>,

    /// Force toggle state (on/off)
    #[arg(short = 'f', long = "force")]
    force: Option<String>,

    /// Comment mode (auto/single/multi)
    #[arg(short = 'm', long = "mode", default_value = "auto")]
    mode: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Process each path
    for path in &cli.paths {
        process_path(path, &cli)
            .with_context(|| format!("Failed to process {}", path.display()))?;
    }

    Ok(())
}

fn process_path(path: &Path, cli: &Cli) -> Result<()> {
    println!("Processing {}:", path.display());

    if let Some(line_range) = &cli.line {
        println!("  Line range: {}", line_range);
        toggle_line_range(path, line_range, &cli.force, &cli.mode)?;
    }

    for section in &cli.sections {
        println!("  Section: {}", section);
        toggle_section(path, section, &cli.force, &cli.mode)?;
    }

    if let Some(force) = &cli.force {
        println!("  Force: {}", force);
    }

    println!("  Mode: {}", cli.mode);

    Ok(())
}

fn toggle_line_range(
    path: &Path,
    line_range: &str,
    force: &Option<String>,
    mode: &str,
) -> Result<()> {
    // Determine comment style based on file extension
    let comment_style = get_comment_style(path, mode)?;

    // Parse line range
    let (start_line, end_line) = parse_line_range(line_range)?;

    // Read file line by line
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<std::io::Result<_>>()?;

    // Validate range
    if start_line == 0 || start_line > lines.len() {
        return Err(anyhow!(
            "Start line {} is out of range (1-{})",
            start_line,
            lines.len()
        ));
    }

    let end_line = std::cmp::min(end_line, lines.len());

    // Force state (on = comment, off = uncomment) or toggle
    let force_state = match force {
        Some(force_str) if force_str == "on" => Some(true),
        Some(force_str) if force_str == "off" => Some(false),
        _ => None,
    };

    // Convert to 0-based indexing
    let start_idx = start_line - 1;
    let end_idx = end_line;

    // Toggle the lines
    toggle_lines(&mut lines, start_idx, end_idx, force_state, &comment_style)?;

    // Write the file back
    let mut file = File::create(path)?;
    for line in &lines {
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

fn parse_line_range(range: &str) -> Result<(usize, usize)> {
    if let Some((start, end)) = range.split_once(':') {
        let start_line = start
            .parse::<usize>()
            .map_err(|_| anyhow!("Invalid start line: {}", start))?;

        if let Some(stripped_end) = end.strip_prefix('+') {
            // Format: start:+count
            let count_str = stripped_end;
            let count = count_str
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid line count: {}", count_str))?;
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
        let line = range
            .parse::<usize>()
            .map_err(|_| anyhow!("Invalid line number: {}", range))?;
        Ok((line, line + 1))
    }
}

fn toggle_section(path: &Path, section_id: &str, force: &Option<String>, mode: &str) -> Result<()> {
    println!("  Looking for section with ID={}", section_id);

    // Determine comment style based on file extension
    let comment_style = get_comment_style(path, mode)?;
    println!(
        "  Using comment style: {} for single-line comments",
        comment_style.single_line
    );

    // Read file line by line
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<std::io::Result<_>>()?;
    println!("  File has {} lines", lines.len());

    // Find section markers and toggle content
    let mut i = 0;
    let mut modified = false;

    while i < lines.len() {
        let line = &lines[i];
        let start_marker = format!("toggle:start ID={}", section_id);

        if line.contains(&start_marker) {
            println!("  Found start marker at line {}: {}", i + 1, line);
            let section_start = i + 1; // Start after the marker

            // Find the end marker
            let mut section_end = lines.len(); // Default to EOF
            let end_marker = format!("toggle:end ID={}", section_id);

            for (j, line) in lines.iter().enumerate().skip(i + 1) {
                if line.contains(&end_marker) {
                    section_end = j; // End before the marker
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

                // Debug: Print content lines that will be toggled
                for (index_in_slice, line) in lines[section_start..section_end].iter().enumerate() {
                    let original_line_index = section_start + index_in_slice;
                    println!("  Content line {}: '{}'", original_line_index + 1, line);
                }

                // Force state (on = comment, off = uncomment) or toggle
                let force_state = match force {
                    Some(force_str) if force_str == "on" => Some(true),
                    Some(force_str) if force_str == "off" => Some(false),
                    _ => None,
                };

                // Toggle the section
                toggle_lines(
                    &mut lines,
                    section_start,
                    section_end,
                    force_state,
                    &comment_style,
                )?;
                modified = true;

                // Skip to after section end
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

    // Write the file back if modified
    if modified {
        println!("  File modified, writing changes back");
        let mut file = File::create(path)?;
        for line in &lines {
            writeln!(file, "{}", line)?;
        }
    } else {
        println!("  No changes made to file");
    }

    Ok(())
}

fn toggle_lines(
    lines: &mut [String],
    start: usize,
    end: usize,
    force_state: Option<bool>,
    comment_style: &CommentStyle,
) -> Result<()> {
    // Determine if the section is already commented
    let is_commented = check_if_commented(&lines[start..end], comment_style);
    println!(
        "  Current section state: {}",
        if is_commented {
            "commented"
        } else {
            "uncommented"
        }
    );

    // Determine if we should comment or uncomment
    let should_comment = match force_state {
        Some(true) => true,    // Force comment (on)
        Some(false) => false,  // Force uncomment (off)
        None => !is_commented, // Toggle current state
    };

    println!(
        "  Will {}",
        if should_comment {
            "comment"
        } else {
            "uncomment"
        }
    );

    if should_comment {
        // Always comment if force=on or toggle from uncommented
        // First uncomment if already commented to avoid double-commenting
        if is_commented {
            // Uncomment first to avoid double-commenting
            for line in lines[start..end].iter_mut() {
                if line.starts_with(&format!("{} ", comment_style.single_line)) {
                    *line = line[comment_style.single_line.len() + 1..].to_string();
                } else if line.starts_with(&comment_style.single_line) {
                    *line = line[comment_style.single_line.len()..].to_string();
                }
            }
            println!("  Uncommented first to avoid double-commenting");
        }

        // Now comment all lines
        for line in lines[start..end].iter_mut() {
            *line = format!("{}{}", comment_style.single_line, line);
        }
        println!("  Commented lines {}-{}", start + 1, end);
    } else if !should_comment && is_commented {
        // Uncomment the lines
        let prefix = format!("{} ", comment_style.single_line);
        let prefix_len = prefix.len();
        for line in lines[start..end].iter_mut() {
            if line.starts_with(&prefix) {
                *line = line[prefix_len..].to_string();
            } else if line.starts_with(&comment_style.single_line) {
                // Handle case where there's no space after the comment marker
                *line = line[comment_style.single_line.len()..].to_string();
            }
        }
        println!("  Uncommented lines {}-{}", start + 1, end);
    } else {
        println!("  No changes needed (already in desired state)");
    }

    Ok(())
}

fn check_if_commented(lines: &[String], comment_style: &CommentStyle) -> bool {
    // Skip empty lines and look for actual content
    let mut first_content_line_commented = false;
    // Check the first non-empty line within the provided slice
    for line in lines {
        let trimmed_line = line.trim_start();
        if !trimmed_line.is_empty() {
            first_content_line_commented = trimmed_line.starts_with(&comment_style.single_line);
            break;
        }
    }

    first_content_line_commented
}

#[derive(Debug, Clone)]
struct CommentStyle {
    single_line: String,
}

fn get_comment_style(path: &Path, _mode: &str) -> Result<CommentStyle> {
    // Get file extension
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    // Map of file extensions to comment styles
    let mut comment_styles = HashMap::new();
    comment_styles.insert(
        "py",
        CommentStyle {
            single_line: "#".to_string(),
        },
    );
    comment_styles.insert(
        "js",
        CommentStyle {
            single_line: "//".to_string(),
        },
    );
    comment_styles.insert(
        "rs",
        CommentStyle {
            single_line: "//".to_string(),
        },
    );
    comment_styles.insert(
        "java",
        CommentStyle {
            single_line: "//".to_string(),
        },
    );
    comment_styles.insert(
        "c",
        CommentStyle {
            single_line: "//".to_string(),
        },
    );
    comment_styles.insert(
        "cpp",
        CommentStyle {
            single_line: "//".to_string(),
        },
    );
    comment_styles.insert(
        "sh",
        CommentStyle {
            single_line: "#".to_string(),
        },
    );
    comment_styles.insert(
        "rb",
        CommentStyle {
            single_line: "#".to_string(),
        },
    );

    // Get comment style based on extension
    comment_styles
        .get(extension)
        .cloned()
        .ok_or_else(|| anyhow!("Unsupported file extension: .{}", extension))
}
