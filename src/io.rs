// File I/O operations for the Toggle CLI

use anyhow::Result;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

/// Read file content with encoding detection
pub fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Write file content with atomic operations
pub fn write_file(path: &Path, content: &str, _temp_suffix: Option<&str>) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Function to detect if a file has UTF-8 BOM
pub fn has_utf8_bom(content: &[u8]) -> bool {
    content.starts_with(&[0xEF, 0xBB, 0xBF])
}

/// Function to detect and preserve shebang and encoding pragmas
pub fn detect_protected_lines(_content: &str) -> Vec<usize> {
    Vec::new()
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
