use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;

use toggle::cli::Cli;
use toggle::config::ToggleConfig;
use toggle::core;
use toggle::exit_codes::{ExitCode, UsageError};
use toggle::io;

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
}

/// Result of processing a single toggle operation.
struct ProcessResult {
    action: String,
    lines_changed: usize,
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

    let effective_force = if cli.force.is_some() {
        cli.force.clone()
    } else {
        config
            .as_ref()
            .and_then(|c| c.global.as_ref())
            .and_then(|g| g.force_state.as_deref())
            .filter(|&s| s != "none")
            .map(String::from)
    };

    // Validate --encoding value
    if !io::is_valid_encoding(&cli.encoding) {
        return Err(UsageError(format!("Unsupported encoding: '{}'", cli.encoding)).into());
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
    };

    if cli.json {
        run_json(cli, &opts)
    } else {
        run_normal(cli, &opts)
    }
}

fn run_normal(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    for path in &cli.paths {
        process_path(path, cli, opts)
            .with_context(|| format!("Failed to process {}", path.display()))?;
    }
    Ok(())
}

fn run_json(cli: &Cli, opts: &ToggleOptions) -> Result<()> {
    let mut results: Vec<ToggleResult> = Vec::new();
    let mut had_error = false;

    for path in &cli.paths {
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

fn process_path(path: &Path, cli: &Cli, opts: &ToggleOptions) -> Result<Vec<ProcessResult>> {
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

    if let Some(line_range) = &cli.line {
        if opts.verbose {
            eprintln!("  Line range: {}", line_range);
        }
        let pr = toggle_line_range(path, line_range, opts)?;
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

fn toggle_line_range(path: &Path, line_range: &str, opts: &ToggleOptions) -> Result<ProcessResult> {
    let comment_style = core::get_comment_style(path, opts.mode, opts.config)?;
    let (start_line, end_line) = core::parse_line_range(line_range)?;
    let content = io::read_file_encoded(path, opts.encoding)?;

    let line_count = content.lines().count();
    if start_line > line_count {
        return Err(UsageError(format!(
            "Start line {} is out of range (file has {} lines)",
            start_line, line_count
        ))
        .into());
    }

    let range = core::LineRange::new(start_line, end_line);
    let force_mode = opts.force.as_deref();
    let toggled = core::toggle_comments_with_marker(
        &content,
        &[range],
        force_mode,
        &comment_style.single_line,
    );
    let result = io::normalize_eol(&toggled, opts.eol);
    let lines_changed = count_changed_lines(&content, &result);

    if opts.dry_run {
        if !opts.json {
            io::print_diff(path, &content, &result);
        }
    } else {
        if let Some(ext) = opts.backup {
            io::create_backup(path, ext)?;
        }
        io::write_file_encoded(
            path,
            &result,
            opts.temp_suffix,
            opts.no_dereference,
            opts.encoding,
        )?;
    }

    Ok(ProcessResult {
        action: "toggle_line_range".to_string(),
        lines_changed,
    })
}

fn toggle_section(path: &Path, section_id: &str, opts: &ToggleOptions) -> Result<ProcessResult> {
    let comment_style = core::get_comment_style(path, opts.mode, opts.config)?;

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

    let modified =
        core::find_and_toggle_section(&mut lines, section_id, opts.force, &comment_style)?;

    let mut lines_changed = 0;

    if modified {
        if opts.verbose {
            eprintln!("  File modified, writing changes back");
        }
        let mut joined = lines.join("\n");
        if original_content.ends_with('\n') {
            joined.push('\n');
        }
        let content = io::normalize_eol(&joined, opts.eol);
        lines_changed = count_changed_lines(&original_content, &content);
        if opts.dry_run {
            if !opts.json {
                io::print_diff(path, &original_content, &content);
            }
        } else {
            if let Some(ext) = opts.backup {
                io::create_backup(path, ext)?;
            }
            io::write_file_encoded(
                path,
                &content,
                opts.temp_suffix,
                opts.no_dereference,
                opts.encoding,
            )?;
        }
    } else if opts.verbose {
        eprintln!("  No changes made to file");
    }

    Ok(ProcessResult {
        action: "toggle_section".to_string(),
        lines_changed,
    })
}
