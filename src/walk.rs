// Directory traversal for recursive file discovery

use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::core::supported_extensions;
use crate::exit_codes::UsageError;

/// Configuration for directory walking
pub struct WalkOptions {
    pub skip_hidden: bool,
    pub max_depth: Option<usize>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            skip_hidden: true,
            max_depth: None,
        }
    }
}

/// Directories to always skip during recursive walks
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "__pycache__",
    "dist",
    "build",
    ".git",
    ".hg",
    ".svn",
];

/// Returns true if the directory entry should be skipped.
fn should_skip_dir(name: &str, skip_hidden: bool) -> bool {
    if skip_hidden && name.starts_with('.') {
        return true;
    }
    SKIP_DIRS.contains(&name)
}

/// Returns true if the file has a supported extension for toggling.
fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported_extensions().contains(&ext))
        .unwrap_or(false)
}

/// Collect files from the given paths.
///
/// - If a path is a file, it is included directly (regardless of extension).
/// - If a path is a directory and `recursive` is true, it is walked recursively,
///   filtering to supported file extensions and skipping hidden/ignored directories.
/// - If a path is a directory and `recursive` is false, an error is returned.
///
/// Results are sorted for deterministic output.
pub fn collect_files(
    paths: &[PathBuf],
    recursive: bool,
    opts: &WalkOptions,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() || !path.exists() {
            // Pass files (and nonexistent paths) through directly;
            // downstream I/O will produce appropriate per-file errors.
            files.push(path.clone());
        } else if path.is_dir() {
            if !recursive {
                return Err(UsageError(format!(
                    "'{}' is a directory; use --recursive (-R) to process directories",
                    path.display()
                ))
                .into());
            }
            walk_directory(path, opts, &mut files)?;
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

/// Walk a directory recursively, collecting supported files.
fn walk_directory(dir: &Path, opts: &WalkOptions, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut walker = WalkDir::new(dir).follow_links(false);

    if let Some(depth) = opts.max_depth {
        walker = walker.max_depth(depth);
    }

    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            // Allow the root directory through
            if e.depth() == 0 {
                return true;
            }
            // Skip filtered directories
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                return !should_skip_dir(name, opts.skip_hidden);
            }
            true
        })
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() && is_supported_file(entry.path()) {
                    files.push(entry.into_path());
                }
            }
            Err(e) => {
                // Skip unreadable entries but continue walking
                eprintln!("Warning: {}", e);
            }
        }
    }

    Ok(())
}
