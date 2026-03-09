// File I/O operations for the Toggle CLI

use anyhow::Result;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

/// Read file content with encoding detection
pub fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Write file content atomically using a temp file + rename.
/// If `temp_suffix` is provided, uses `path.<suffix>` as the temp file name.
/// Otherwise uses a NamedTempFile in the same directory.
pub fn write_file(path: &Path, content: &str, temp_suffix: Option<&str>) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));

    if let Some(suffix) = temp_suffix {
        // Use explicit temp file name
        let temp_path = path.with_extension(suffix);
        let mut file = File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp_path, path)?;
    } else {
        // Use tempfile crate for safe atomic write
        let mut tmp = NamedTempFile::new_in(dir)?;
        tmp.write_all(content.as_bytes())?;
        tmp.as_file().sync_all()?;
        tmp.persist(path).map_err(|e| e.error)?;
    }

    Ok(())
}

/// Function to detect if a file has UTF-8 BOM
pub fn has_utf8_bom(content: &[u8]) -> bool {
    content.starts_with(&[0xEF, 0xBB, 0xBF])
}

/// Detect lines that should never be toggled: shebang and encoding pragma.
/// Returns 0-based line indices of protected lines.
pub fn detect_protected_lines(content: &str) -> Vec<usize> {
    let mut protected = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Shebang: first non-blank line starting with #!
        if trimmed.starts_with("#!") && !protected.iter().any(|&idx| {
            content.lines().nth(idx).map_or(false, |l| l.trim().starts_with("#!"))
        }) {
            protected.push(i);
        }

        // PEP 263 encoding pragma: first non-blank line matching #.*coding[:=]
        if trimmed.starts_with('#') && (trimmed.contains("coding:") || trimmed.contains("coding=")) {
            if !protected.iter().any(|&idx| {
                content.lines().nth(idx).map_or(false, |l| {
                    let t = l.trim();
                    t.starts_with('#') && (t.contains("coding:") || t.contains("coding="))
                })
            }) {
                protected.push(i);
            }
        }
    }

    protected
}

/// Read a file into a Vec of lines
pub fn read_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<io::Result<_>>()?;
    Ok(lines)
}

/// Write a Vec of lines back to a file
pub fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    let mut file = File::create(path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }
    Ok(())
}
