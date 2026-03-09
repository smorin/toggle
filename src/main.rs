use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::Path;

use toggle::cli::Cli;
use toggle::core;
use toggle::exit_codes::ExitCode;
use toggle::io;

fn main() {
    let cli = Cli::parse();

    let result = run(&cli);
    let code = match &result {
        Ok(_) => ExitCode::Success,
        Err(_) => ExitCode::ToggleError,
    };

    if let Err(e) = &result {
        eprintln!("Error: {:#}", e);
    }

    let exit_val = if cli.posix_exit { code.posix() } else { code.code() };
    std::process::exit(exit_val);
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
                "File '{}' is not a .py file (use --strict-ext to enforce Python-only)",
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
        toggle_line_range(path, line_range, &cli.force, &cli.mode)?;
    }

    for section in &cli.sections {
        if cli.verbose {
            eprintln!("  Section: {}", section);
        }
        toggle_section(path, section, &cli.force, &cli.mode)?;
    }

    Ok(())
}

fn toggle_line_range(
    path: &Path,
    line_range: &str,
    force: &Option<String>,
    mode: &str,
) -> Result<()> {
    let comment_style = core::get_comment_style(path, mode)?;
    let (start_line, end_line) = core::parse_line_range(line_range)?;

    let mut lines = io::read_lines(path)?;

    if start_line == 0 || start_line > lines.len() {
        return Err(anyhow!(
            "Start line {} is out of range (1-{})",
            start_line,
            lines.len()
        ));
    }

    let end_line = std::cmp::min(end_line, lines.len());

    let force_state = parse_force_state(force);
    let start_idx = start_line - 1;

    core::toggle_lines(&mut lines, start_idx, end_line, force_state, &comment_style)?;

    io::write_lines(path, &lines)?;

    Ok(())
}

fn toggle_section(
    path: &Path,
    section_id: &str,
    force: &Option<String>,
    mode: &str,
) -> Result<()> {
    println!("  Looking for section with ID={}", section_id);

    let comment_style = core::get_comment_style(path, mode)?;
    println!(
        "  Using comment style: {} for single-line comments",
        comment_style.single_line
    );

    let mut lines = io::read_lines(path)?;
    println!("  File has {} lines", lines.len());

    let modified = core::find_and_toggle_section(&mut lines, section_id, force, &comment_style)?;

    if modified {
        println!("  File modified, writing changes back");
        io::write_lines(path, &lines)?;
    } else {
        println!("  No changes made to file");
    }

    Ok(())
}

fn parse_force_state(force: &Option<String>) -> Option<bool> {
    match force {
        Some(force_str) if force_str == "on" => Some(true),
        Some(force_str) if force_str == "off" => Some(false),
        _ => None,
    }
}
