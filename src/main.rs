use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::Path;

use toggle::cli::Cli;
use toggle::core;
use toggle::exit_codes::ExitCode;
use toggle::io;

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
        eprintln!("Error: {:#}", e);
    }

    let exit_val = if cli.posix_exit { code.posix() } else { code.code() };
    std::process::exit(exit_val);
}

fn classify_error(err: &anyhow::Error) -> ExitCode {
    // Walk the error chain looking for specific error types
    for cause in err.chain() {
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return ExitCode::IoError;
        }
    }

    // Check error message for usage-related patterns
    let msg = format!("{:#}", err);
    if msg.contains("Invalid start line")
        || msg.contains("Invalid end line")
        || msg.contains("Invalid line number")
        || msg.contains("Invalid line count")
        || msg.contains("End line")
        || msg.contains("Line number must be")
        || msg.contains("Start line must be")
        || msg.contains("is not a .py file")
        || msg.contains("Unsupported file extension")
    {
        return ExitCode::Usage;
    }

    ExitCode::ToggleError
}

fn run(cli: &Cli) -> Result<()> {
    for path in &cli.paths {
        process_path(path, cli)
            .with_context(|| format!("Failed to process {}", path.display()))?;
    }
    Ok(())
}

fn process_path(path: &Path, cli: &Cli) -> Result<()> {
    // --strict-ext: reject non-.py files
    if cli.strict_ext {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "py" {
            return Err(anyhow!(
                "File '{}' is not a .py file (rejected by --strict-ext)",
                path.display()
            ));
        }
    }

    if cli.verbose {
        eprintln!("Processing {}:", path.display());
    }

    if let Some(line_range) = &cli.line {
        if cli.verbose {
            eprintln!("  Line range: {}", line_range);
        }
        toggle_line_range(path, line_range, &cli.force, &cli.mode, cli.temp_suffix.as_deref())?;
    }

    for section in &cli.sections {
        if cli.verbose {
            eprintln!("  Section: {}", section);
        }
        toggle_section(path, section, &cli.force, &cli.mode, cli.verbose, cli.temp_suffix.as_deref())?;
    }

    Ok(())
}

fn toggle_line_range(
    path: &Path,
    line_range: &str,
    force: &Option<String>,
    mode: &str,
    temp_suffix: Option<&str>,
) -> Result<()> {
    let comment_style = core::get_comment_style(path, mode)?;
    let (start_line, end_line) = core::parse_line_range(line_range)?;
    let content = io::read_file(path)?;

    let range = core::LineRange::new(start_line, end_line);
    let force_mode = force.as_deref();
    let result = core::toggle_comments_with_marker(
        &content,
        &[range],
        force_mode,
        &comment_style.single_line,
    );

    io::write_file(path, &result, temp_suffix)?;

    Ok(())
}

fn toggle_section(
    path: &Path,
    section_id: &str,
    force: &Option<String>,
    mode: &str,
    verbose: bool,
    temp_suffix: Option<&str>,
) -> Result<()> {
    let comment_style = core::get_comment_style(path, mode)?;

    if verbose {
        eprintln!("  Looking for section with ID={}", section_id);
        eprintln!(
            "  Using comment style: {} for single-line comments",
            comment_style.single_line
        );
    }

    let original_content = io::read_file(path)?;
    let mut lines: Vec<String> = original_content.lines().map(String::from).collect();

    if verbose {
        eprintln!("  File has {} lines", lines.len());
    }

    let modified = core::find_and_toggle_section(&mut lines, section_id, force, &comment_style)?;

    if modified {
        if verbose {
            eprintln!("  File modified, writing changes back");
        }
        let mut content = lines.join("\n");
        if original_content.ends_with('\n') {
            content.push('\n');
        }
        io::write_file(path, &content, temp_suffix)?;
    } else if verbose {
        eprintln!("  No changes made to file");
    }

    Ok(())
}
