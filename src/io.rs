// File I/O operations for the Toggle CLI

use similar::TextDiff;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Read file content as UTF-8.
pub fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Read file content with a specified encoding.
/// Supports any encoding label recognized by the Encoding Standard
/// (e.g., "utf-8", "latin-1", "iso-8859-1", "windows-1252", "ascii").
pub fn read_file_encoded(path: &Path, encoding: &str) -> io::Result<String> {
    if encoding.eq_ignore_ascii_case("utf-8") {
        return read_file(path);
    }
    let bytes = std::fs::read(path)?;
    let enc = resolve_encoding(encoding)?;
    let (decoded, _, had_errors) = enc.decode(&bytes);
    if had_errors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to decode file as {}", encoding),
        ));
    }
    Ok(decoded.into_owned())
}

/// Resolve an encoding label to an encoding_rs::Encoding.
/// Handles common aliases like "latin-1" that encoding_rs doesn't directly recognize.
fn resolve_encoding(label: &str) -> io::Result<&'static encoding_rs::Encoding> {
    // Try direct lookup first
    if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
        return Ok(enc);
    }
    // Handle common aliases not in the Encoding Standard
    let alias = match label.to_ascii_lowercase().as_str() {
        "latin-1" | "latin1" => Some("iso-8859-1"),
        "ascii" | "us-ascii" => Some("windows-1252"),
        _ => None,
    };
    if let Some(alias_label) = alias {
        if let Some(enc) = encoding_rs::Encoding::for_label(alias_label.as_bytes()) {
            return Ok(enc);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Unsupported encoding: {}", label),
    ))
}

/// Check if an encoding label is valid/supported.
pub fn is_valid_encoding(label: &str) -> bool {
    if label.eq_ignore_ascii_case("utf-8") {
        return true;
    }
    resolve_encoding(label).is_ok()
}

/// Encode a string into bytes using the specified encoding.
fn encode_string(content: &str, encoding: &str) -> io::Result<Vec<u8>> {
    if encoding.eq_ignore_ascii_case("utf-8") {
        return Ok(content.as_bytes().to_vec());
    }
    let enc = resolve_encoding(encoding)?;
    let (encoded, _, had_errors) = enc.encode(content);
    if had_errors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to encode content as {}", encoding),
        ));
    }
    Ok(encoded.into_owned())
}

/// Check if a path is a symbolic link.
pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Resolve symlink target to an absolute path.
/// If the symlink target is relative, resolves it against the symlink's parent directory.
fn resolve_symlink(path: &Path) -> io::Result<PathBuf> {
    let target = std::fs::read_link(path)?;
    if target.is_absolute() {
        Ok(target)
    } else {
        let parent = path.parent().unwrap_or(Path::new("."));
        Ok(parent.join(target))
    }
}

/// Write file content atomically using a temp file + rename.
/// If `temp_suffix` is provided, uses `path.<suffix>` as the temp file name.
/// Otherwise uses a NamedTempFile in the same directory.
/// If `no_dereference` is true and path is a symlink, writes to the symlink's
/// target instead of replacing the symlink.
pub fn write_file(path: &Path, content: &str, temp_suffix: Option<&str>) -> io::Result<()> {
    write_bytes_impl(path, content.as_bytes(), temp_suffix, false)
}

/// Write file with optional symlink-aware behavior.
pub fn write_file_no_deref(
    path: &Path,
    content: &str,
    temp_suffix: Option<&str>,
    no_dereference: bool,
) -> io::Result<()> {
    let bytes = content.as_bytes();
    write_bytes_impl(path, bytes, temp_suffix, no_dereference)
}

/// Write file with encoding and symlink support.
pub fn write_file_encoded(
    path: &Path,
    content: &str,
    temp_suffix: Option<&str>,
    no_dereference: bool,
    encoding: &str,
) -> io::Result<()> {
    let bytes = encode_string(content, encoding)?;
    write_bytes_impl(path, &bytes, temp_suffix, no_dereference)
}

fn write_bytes_impl(
    path: &Path,
    bytes: &[u8],
    temp_suffix: Option<&str>,
    no_dereference: bool,
) -> io::Result<()> {
    let write_path = if no_dereference && is_symlink(path) {
        resolve_symlink(path)?
    } else {
        path.to_path_buf()
    };
    let dir = write_path.parent().unwrap_or(Path::new("."));

    if let Some(suffix) = temp_suffix {
        // Use explicit temp file name: file.py.tmp (append suffix, not replace extension)
        let mut temp_name = write_path.as_os_str().to_os_string();
        temp_name.push(".");
        temp_name.push(suffix);
        let temp_path = std::path::PathBuf::from(temp_name);
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp_path, &write_path)?;
    } else {
        // Use tempfile crate for safe atomic write
        let mut tmp = NamedTempFile::new_in(dir)?;
        tmp.write_all(bytes)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&write_path).map_err(|e| e.error)?;
    }

    Ok(())
}

/// Print a unified diff between original and modified content.
/// No-ops if content is identical.
pub fn print_diff(path: &Path, original: &str, modified: &str) {
    if original == modified {
        return;
    }
    let diff = TextDiff::from_lines(original, modified);
    let path_str = path.display().to_string();
    print!(
        "{}",
        diff.unified_diff()
            .header(&format!("a/{}", path_str), &format!("b/{}", path_str))
    );
}

/// Create a backup copy of a file by appending the given extension.
/// e.g., create_backup("file.py", ".bak") creates "file.py.bak"
pub fn create_backup(path: &Path, extension: &str) -> io::Result<()> {
    let mut backup_path = path.as_os_str().to_os_string();
    backup_path.push(extension);
    std::fs::copy(path, PathBuf::from(backup_path))?;
    Ok(())
}

/// Normalize line endings in content.
/// - "preserve": return unchanged
/// - "lf": convert all line endings to \n
/// - "crlf": convert all line endings to \r\n
pub fn normalize_eol(content: &str, eol: &str) -> String {
    match eol {
        "lf" => content.replace("\r\n", "\n").replace('\r', "\n"),
        "crlf" => {
            // First normalize to LF, then convert to CRLF
            let lf = content.replace("\r\n", "\n").replace('\r', "\n");
            lf.replace('\n', "\r\n")
        }
        _ => content.to_string(), // "preserve" or any other value
    }
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
        if trimmed.starts_with('#')
            && (trimmed.contains("coding:") || trimmed.contains("coding="))
            && !protected.contains(&i)
        {
            protected.push(i);
        }
    }

    protected
}
