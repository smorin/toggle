use anyhow::{Context, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use toggle::cli::Cli;
use toggle::config::ToggleConfig;
use toggle::core;
use toggle::exit_codes::{ExitCode, UsageError};
use toggle::io;
use toggle::walk;

/// Bundled options passed through the toggle pipeline.
struct ToggleOptions<'a> {
    force: &'a Option<String>,
    mode: &'a str,
    temp_suffix: Option<&'a str>,
    dry_run: bool,
    backup: Option<&'a str>,
    config: Option<&'a ToggleConfig>,
    verbose: bool,
    eol: &'a str,
    no_dereference: bool,
    encoding: &'a str,
    json: bool,
    to_end: bool,
    comment_style_override: &'a [String],
    interactive: bool,
    recursive: bool,
}

/// Result of processing a single toggle operation.
struct ProcessResult {
    action: String,
    lines_changed: usize,
    section_id: Option<String>,
    desc: Option<String>,
}

/// JSON output entry for --json mode.
#[derive(serde::Serialize)]
struct ToggleResult {
    file: String,
    action: String,
    lines_changed: usize,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<String>,
}

/// JSON output for --list-sections mode.
#[derive(serde::Serialize)]
struct SectionListEntry {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<String>,
    files: Vec<SectionFileEntry>,
}

/// A single file location in --list-sections JSON output.
#[derive(serde::Serialize)]
struct SectionFileEntry {
    file: String,
    start_line: usize,
    end_line: usize,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Let clap print its error/help message
            let _ = e.print();
            // Use our custom Usage exit code instead of clap's default (2)
            std::process::exit(ExitCode::Usage.code());
        }
    };

    let result = run(&cli);
    let code = match &result {
        Ok(_) => ExitCode::Success,
        Err(e) => classify_error(e),
    };

    if let Err(e) = &result {
        if !cli.json {
            eprintln!("Error: {:#}", e);
        }
    }

    let exit_val = if cli.posix_exit {
        code.posix()
    } else {
        code.code()
    };
    std::process::exit(exit_val);
}

fn classify_error(err: &anyhow::Error) -> ExitCode {
    // Walk the error chain looking for specific typed errors
    for cause in err.chain() {
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return ExitCode::IoError;
        }
        if cause.downcast_ref::<UsageError>().is_some() {
            return ExitCode::Usage;
        }
    }

    ExitCode::ToggleError
}

fn run(cli: &Cli) -> Result<()> {
    let config = if let Some(config_path) = &cli.config {
        Some(ToggleConfig::load(config_path)?)
    } else {
        None
    };

    // CLI flags override config values
    let effective_mode = if cli.mode == "auto" {
        config
            .as_ref()
            .and_then(|c| c.global.as_ref())
            .and_then(|g| g.default_mode.as_deref())
            .unwrap_or("auto")
            .to_string()
    } else {
        cli.mode.clone()
    };

    let effective_force = if let Some(ref val) = cli.force {
        match val.as_str() {
            "on" | "off" => cli.force.clone(),
            "invert" => None,
            other => {
                return Err(UsageError(format!(
                    "Invalid --force value '{}': expected on, off, or invert",
                    other
                ))
                .into());
            }
        }
    } else {
        config
            .as_ref()
            .and_then(|c| c.global.as_ref())
            .and_then(|g| g.force_state.as_deref())
            .filter(|&s| s != "none")
            .map(String::from)
    };

    // Validate --encoding value (before --scan so it applies to all modes)
    if !io::is_valid_encoding(&cli.encoding) {
        return Err(UsageError(format!("Unsupported encoding: '{}'", cli.encoding)).into());
    }

    // Handle --scan mode early (read-only, no toggle options needed)
    if cli.scan {
        if !cli.lines.is_empty() {
            return Err(UsageError("--scan cannot be combined with --line".into()).into());
        }
        if !cli.sections.is_empty() {
            return Err(UsageError("--scan cannot be combined with --section".into()).into());
        }
        if cli.force.is_some() {
            return Err(UsageError("--scan cannot be combined with --force".into()).into());
        }
        return run_scan(cli);
    }

    // Validate --comment-style: must be 1 or 3 values
    if cli.comment_style.len() == 2 {
        return Err(UsageError(
            "--comment-style requires 1 value (single-line) or 3 values (single-line, multi-start, multi-end)".into(),
        )
        .into());
    }

    // Validate --to-end requires --line
    if cli.to_end && cli.lines.is_empty() {
        return Err(UsageError("--to-end requires at least one --line range".into()).into());
    }

    // Validate --list-sections conflicts
    if cli.list_sections && !cli.lines.is_empty() {
        return Err(UsageError("--list-sections cannot be combined with --line".into()).into());
    }
    if cli.list_sections && cli.force.is_some() {
        return Err(UsageError("--list-sections cannot be combined with --force".into()).into());
    }

    // Validate --eol value
    match cli.eol.as_str() {
        "preserve" | "lf" | "crlf" => {}
        other => {
            return Err(UsageError(format!(
                "Invalid --eol value '{}': must be preserve, lf, or crlf",
                other
            ))
            .into());
        }
    }

    let opts = ToggleOptions {
        force: &effective_force,
        mode: &effective_mode,
        temp_suffix: cli.temp_suffix.as_deref(),
        dry_run: cli.dry_run,
        backup: cli.backup.as_deref(),
        config: config.as_ref(),
        verbose: cli.verbose && !cli.json, // suppress verbose in JSON mode
        eol: &cli.eol,
        no_dereference: cli.no_dereference,
        encoding: &cli.encoding,
        json: cli.json,
        to_end: cli.to_end,
        comment_style_override: &cli.comment_style,
        interactive: cli.interactive,
        recursive: cli.recursive,
    };

    if cli.list_sections {
        run_list_sections(cli, &opts)
    } else if cli.json {
        run_json(cli, &opts)
    } else {
        run_normal(cli, &opts)
    }
}

/// Expand CLI paths into individual files, walking directories when recursive is set.
fn collect_files(paths: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    if !recursive {
        return paths.to_vec();
    }
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    // Always include the root directory (depth 0), only filter
                    // hidden entries in subdirectories
                    e.depth() == 0 || !e.file_name().to_str().is_some_and(|s| s.starts_with('.'))
                })
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    files.push(entry.into_path());
                }
            }
        } else {
            // Non-existent or special paths: include as-is to let process_path report errors
            files.push(path.clone());
        }
    }
    files
}

/// Check if a file has any sections matching the requested IDs.
/// Returns true if at least one section matches, or if no section filter is active.
fn file_has_matching_sections(path: &Path, section_ids: &[String], encoding: &str) -> bool {
    if section_ids.is_empty() {
        return true;
    }
    let content = match io::read_file_encoded(path, encoding) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let found = core::discover_sections(&content);
    found.iter().any(|s| section_ids.contains(&s.id))
}

fn run_normal(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    let files = collect_files(&cli.paths, cli.recursive);

    for path in &files {
        // In recursive mode with sections, skip files that don't contain matching sections
        if cli.recursive
            && !cli.sections.is_empty()
            && !file_has_matching_sections(path, &cli.sections, opts.encoding)
        {
            continue;
        }
        // In recursive mode, silently skip files with unsupported extensions
        if cli.recursive
            && opts.comment_style_override.is_empty()
            && core::get_comment_style(path, opts.mode, opts.config).is_err()
        {
            continue;
        }
        process_path(path, cli, opts)
            .with_context(|| format!("Failed to process {}", path.display()))?;
    }
    Ok(())
}

fn run_json(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    let files = collect_files(&cli.paths, cli.recursive);
    let mut results: Vec<ToggleResult> = Vec::new();
    let mut had_error = false;

    for path in &files {
        // In recursive mode with sections, skip files that don't contain matching sections
        if cli.recursive
            && !cli.sections.is_empty()
            && !file_has_matching_sections(path, &cli.sections, opts.encoding)
        {
            continue;
        }
        // In recursive mode, silently skip files with unsupported extensions
        if cli.recursive
            && opts.comment_style_override.is_empty()
            && core::get_comment_style(path, opts.mode, opts.config).is_err()
        {
            continue;
        }

        match process_path(path, cli, opts) {
            Ok(proc_results) => {
                for pr in proc_results {
                    results.push(ToggleResult {
                        file: path.display().to_string(),
                        action: pr.action,
                        lines_changed: pr.lines_changed,
                        success: true,
                        error: None,
                        dry_run: opts.dry_run,
                        section_id: pr.section_id,
                        desc: pr.desc,
                    });
                }
            }
            Err(e) => {
                had_error = true;
                results.push(ToggleResult {
                    file: path.display().to_string(),
                    action: String::new(),
                    lines_changed: 0,
                    success: false,
                    error: Some(format!("{:#}", e)),
                    dry_run: opts.dry_run,
                    section_id: None,
                    desc: None,
                });
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string(&results).expect("Failed to serialize JSON")
    );

    if had_error {
        // Return a generic error so main() sets a non-zero exit code
        anyhow::bail!("One or more files failed to process");
    }
    Ok(())
}

type SectionAggregation = (Option<String>, Vec<(String, usize, usize)>);

fn run_list_sections(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    let files = collect_files(&cli.paths, cli.recursive);

    // Aggregate sections grouped by ID, preserving insertion order with BTreeMap
    let mut sections_by_id: BTreeMap<String, SectionAggregation> = BTreeMap::new();

    for path in &files {
        let content = match io::read_file_encoded(path, opts.encoding) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let found = core::discover_sections(&content);
        for section in found {
            let entry = sections_by_id
                .entry(section.id.clone())
                .or_insert_with(|| (section.desc.clone(), Vec::new()));
            // If we haven't captured a desc yet, use this one
            if entry.0.is_none() && section.desc.is_some() {
                entry.0 = section.desc.clone();
            }
            entry.1.push((
                path.display().to_string(),
                section.start_line,
                section.end_line,
            ));
        }
    }

    if cli.json {
        let entries: Vec<SectionListEntry> = sections_by_id
            .into_iter()
            .map(|(id, (desc, files))| SectionListEntry {
                id,
                desc,
                files: files
                    .into_iter()
                    .map(|(file, start, end)| SectionFileEntry {
                        file,
                        start_line: start,
                        end_line: end,
                    })
                    .collect(),
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&entries).expect("Failed to serialize JSON")
        );
    } else {
        for (id, (desc, locations)) in &sections_by_id {
            if let Some(d) = desc {
                println!("{} desc=\"{}\"", id, d);
            } else {
                println!("{}", id);
            }
            for (file, start, end) in locations {
                println!("  {}:{}-{}", file, start, end);
            }
        }
    }

    Ok(())
}

fn process_path(path: &Path, cli: &Cli, opts: &ToggleOptions) -> Result<Vec<ProcessResult>> {
    // If path is a directory, handle recursive traversal
    if path.is_dir() {
        if !opts.recursive {
            return Err(UsageError(format!(
                "'{}' is a directory; use -R/--recursive to process directories",
                path.display()
            ))
            .into());
        }
        let mut results = Vec::new();
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            // Skip files with unsupported extensions (unless --comment-style is set)
            if opts.comment_style_override.is_empty()
                && core::get_comment_style(file_path, opts.mode, opts.config).is_err()
            {
                continue;
            }
            match process_file(file_path, cli, opts) {
                Ok(mut file_results) => results.append(&mut file_results),
                Err(e) => {
                    if opts.verbose {
                        eprintln!("  Skipping {}: {}", file_path.display(), e);
                    }
                }
            }
        }
        return Ok(results);
    }

    process_file(path, cli, opts)
}

fn process_file(path: &Path, cli: &Cli, opts: &ToggleOptions) -> Result<Vec<ProcessResult>> {
    // --strict-ext: reject non-.py files
    if cli.strict_ext {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "py" {
            return Err(UsageError(format!(
                "File '{}' is not a .py file (rejected by --strict-ext)",
                path.display()
            ))
            .into());
        }
    }

    if opts.verbose {
        eprintln!("Processing {}:", path.display());
    }

    let mut results = Vec::new();

    if !cli.lines.is_empty() {
        if opts.verbose {
            for lr in &cli.lines {
                eprintln!("  Line range: {}", lr);
            }
        }
        let pr = toggle_line_ranges(path, &cli.lines, opts)?;
        results.push(pr);
    }

    for section in &cli.sections {
        if opts.verbose {
            eprintln!("  Section: {}", section);
        }
        let pr = toggle_section(path, section, opts)?;
        results.push(pr);
    }

    Ok(results)
}

/// Count the number of lines that differ between two strings.
fn count_changed_lines(original: &str, modified: &str) -> usize {
    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();
    let max_len = orig_lines.len().max(mod_lines.len());
    let mut changed = 0;
    for i in 0..max_len {
        let a = orig_lines.get(i).copied().unwrap_or("");
        let b = mod_lines.get(i).copied().unwrap_or("");
        if a != b {
            changed += 1;
        }
    }
    changed
}

/// Apply changes to a file: handles dry-run, interactive prompt, backup, and write.
/// Returns the number of lines changed.
fn apply_changes(
    path: &Path,
    original: &str,
    modified: &str,
    opts: &ToggleOptions,
) -> Result<usize> {
    let lines_changed = count_changed_lines(original, modified);

    if opts.dry_run {
        if !opts.json {
            io::print_diff(path, original, modified);
        }
        if opts.interactive && std::io::stdin().is_terminal() {
            // In dry-run + interactive, just show the diff (already done above)
            eprintln!("(dry-run mode, no changes will be written)");
        }
        return Ok(lines_changed);
    }

    // Interactive prompt
    if opts.interactive {
        // Show diff preview before prompting (only on TTY to avoid polluting piped output)
        if std::io::stdin().is_terminal() && !opts.json {
            io::print_diff(path, original, modified);
        }
        eprint!("Modify {}? [y/N] ", path.display());
        use std::io::Write;
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|e| anyhow::anyhow!("Failed to read interactive input: {}", e))?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            if opts.verbose {
                eprintln!("  Skipped {}", path.display());
            }
            return Ok(0);
        }
    }

    if let Some(ext) = opts.backup {
        io::create_backup(path, ext)?;
    }
    io::write_file_encoded(
        path,
        modified,
        opts.temp_suffix,
        opts.no_dereference,
        opts.encoding,
    )?;
    Ok(lines_changed)
}

/// Resolve comment style for a file, applying --comment-style override if present.
fn resolve_comment_style(path: &Path, opts: &ToggleOptions) -> Result<core::CommentStyle> {
    if !opts.comment_style_override.is_empty() {
        let single = opts.comment_style_override[0].clone();
        let (ms, me) = if opts.comment_style_override.len() == 3 {
            (
                Some(opts.comment_style_override[1].clone()),
                Some(opts.comment_style_override[2].clone()),
            )
        } else {
            (None, None)
        };
        return Ok(core::CommentStyle {
            single_line: single,
            multi_line_start: ms,
            multi_line_end: me,
        });
    }
    core::get_comment_style(path, opts.mode, opts.config)
}

fn toggle_line_ranges(
    path: &Path,
    line_range_specs: &[String],
    opts: &ToggleOptions,
) -> Result<ProcessResult> {
    let comment_style = resolve_comment_style(path, opts)?;
    let content = io::read_file_encoded(path, opts.encoding)?;
    let line_count = content.lines().count();

    // Parse all range specs into LineRange values
    let mut ranges = Vec::new();
    for spec in line_range_specs {
        let (start_line, end_line) = core::parse_line_range(spec)?;
        if start_line > line_count {
            return Err(UsageError(format!(
                "Start line {} is out of range (file has {} lines)",
                start_line, line_count
            ))
            .into());
        }
        ranges.push(core::LineRange::new(start_line, end_line));
    }

    // --to-end: extend the last range's end to the file's line count
    if opts.to_end {
        if let Some(last) = ranges.last_mut() {
            last.end = line_count;
        }
    }

    // Validate end lines against file length (after --to-end extension)
    for range in &ranges {
        if range.end > line_count {
            return Err(UsageError(format!(
                "End line {} is out of range (file has {} lines)",
                range.end, line_count
            ))
            .into());
        }
    }

    let merged = core::merge_ranges(&ranges);
    let force_mode = opts.force.as_deref();
    let toggled = if opts.mode == "multi" {
        let (ms, me) = match (
            &comment_style.multi_line_start,
            &comment_style.multi_line_end,
        ) {
            (Some(s), Some(e)) => (s.as_str(), e.as_str()),
            _ => {
                return Err(UsageError(format!(
                    "Multi-line comments not supported for {}",
                    path.display()
                ))
                .into());
            }
        };
        core::toggle_comments_multi(&content, &merged, force_mode, ms, me)
    } else {
        core::toggle_comments_with_marker(&content, &merged, force_mode, &comment_style.single_line)
    };
    let result = io::normalize_eol(&toggled, opts.eol);
    let lines_changed = apply_changes(path, &content, &result, opts)?;

    Ok(ProcessResult {
        action: "toggle_line_range".to_string(),
        lines_changed,
        section_id: None,
        desc: None,
    })
}

fn toggle_section(path: &Path, section_id: &str, opts: &ToggleOptions) -> Result<ProcessResult> {
    let comment_style = resolve_comment_style(path, opts)?;

    if opts.verbose {
        eprintln!("  Looking for section with ID={}", section_id);
        eprintln!(
            "  Using comment style: {} for single-line comments",
            comment_style.single_line
        );
    }

    let original_content = io::read_file_encoded(path, opts.encoding)?;
    let mut lines: Vec<String> = original_content.lines().map(String::from).collect();

    if opts.verbose {
        eprintln!("  File has {} lines", lines.len());
    }

    let result = core::find_and_toggle_section(&mut lines, section_id, opts.force, &comment_style)?;

    if opts.verbose {
        if let Some(ref d) = result.desc {
            eprintln!("  Section desc: {}", d);
        }
    }

    let mut lines_changed = 0;

    if result.modified {
        if opts.verbose {
            eprintln!("  File modified, writing changes back");
        }
        let mut joined = lines.join("\n");
        if original_content.ends_with('\n') {
            joined.push('\n');
        }
        let content = io::normalize_eol(&joined, opts.eol);
        lines_changed = apply_changes(path, &original_content, &content, opts)?;
    } else if opts.verbose {
        eprintln!("  No changes made to file");
    }

    Ok(ProcessResult {
        action: "toggle_section".to_string(),
        lines_changed,
        section_id: Some(section_id.to_string()),
        desc: result.desc,
    })
}

fn run_scan(cli: &Cli) -> Result<()> {
    let walk_opts = walk::WalkOptions {
        verbose: cli.verbose,
        ..walk::WalkOptions::default()
    };
    // --scan is implicitly recursive
    let files = walk::collect_files(&cli.paths, true, &walk_opts)?;

    // Warn about paths that don't exist on disk
    for path in &cli.paths {
        if !path.exists() {
            eprintln!("Warning: '{}' does not exist", path.display());
        }
    }

    let mut all_sections: Vec<core::ScanSectionInfo> = Vec::new();
    for file_path in &files {
        match io::read_file_encoded(file_path, &cli.encoding) {
            Ok(content) => {
                let sections = core::scan_sections(file_path, &content);
                all_sections.extend(sections);
            }
            Err(e) => {
                if cli.verbose {
                    eprintln!("Warning: skipping {}: {}", file_path.display(), e);
                }
            }
        }
    }

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&all_sections).expect("Failed to serialize JSON")
        );
    } else {
        print_scan_results(&all_sections);
    }
    Ok(())
}

fn print_scan_results(sections: &[core::ScanSectionInfo]) {
    if sections.is_empty() {
        println!("No toggle sections found.");
        return;
    }

    // Print header
    println!(
        "{:<20} {:<45} {:<12} {:<14} Description",
        "Section ID", "File", "Lines", "State"
    );
    println!("{}", "\u{2500}".repeat(100));

    for s in sections {
        let end = s.end_line.map_or("???".to_string(), |e| e.to_string());
        let lines_str = format!("{}-{}", s.start_line, end);
        let desc = s.description.as_deref().unwrap_or("");
        println!(
            "{:<20} {:<45} {:<12} {:<14} {}",
            s.id, s.file, lines_str, s.state, desc
        );
    }
}
