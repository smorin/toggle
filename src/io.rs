// File I/O operations for the Toggle CLI

use std::fs::File;
use std::io::{self, Read, Write};
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
        // Use explicit temp file name: file.py.tmp (append suffix, not replace extension)
        let mut temp_name = path.as_os_str().to_os_string();
        temp_name.push(".");
        temp_name.push(suffix);
        let temp_path = std::path::PathBuf::from(temp_name);
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
/// Only checks the first two non-blank lines (shebangs are only valid on line 1,
/// PEP 263 encoding pragmas on lines 1-2).
/// Returns 0-based line indices of protected lines.
pub fn detect_protected_lines(content: &str) -> Vec<usize> {
    let mut protected = Vec::new();
    let mut non_blank_seen = 0;

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        non_blank_seen += 1;
        if non_blank_seen > 2 {
            break;
        }

        // Shebang: must be first non-blank line
        if non_blank_seen == 1 && trimmed.starts_with("#!") {
            protected.push(i);
        }

        // PEP 263 encoding pragma: first or second non-blank line
        if trimmed.starts_with('#') && (trimmed.contains("coding:") || trimmed.contains("coding=")) {
            if !protected.contains(&i) {
                protected.push(i);
            }
        }
    }

    protected
}
