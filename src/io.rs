// File I/O operations for the Toggle CLI

use crate::journal::{self, Journal, JournalEntry, JOURNAL_FILENAME, LOCK_FILENAME};
use crate::platform;
use similar::TextDiff;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

/// Encode a string for atomic mode staging. Public wrapper around encode_string.
pub fn encode_for_atomic(content: &str, encoding: &str) -> io::Result<Vec<u8>> {
    encode_string(content, encoding)
}

// ── Atomic multi-file batch operations ──

/// Threshold for emitting a batch size warning.
const BATCH_SIZE_WARNING_THRESHOLD: usize = 500;

/// Default backup extension for atomic mode.
const ATOMIC_BACKUP_EXT: &str = ".toggle-atomic-backup";

/// A single staged write: temp file is written and fsynced, ready for rename.
pub struct StagedWrite {
    /// Path to the staged temp file (fd already released via into_temp_path).
    pub temp_path: PathBuf,
    /// Final target path.
    pub target_path: PathBuf,
    /// SHA-256 hex digest of the written content.
    pub content_sha256: String,
    /// Original file permissions to copy to temp before rename.
    pub original_permissions: Option<std::fs::Permissions>,
}

/// Manages a two-phase atomic commit of multiple file writes.
pub struct AtomicBatch {
    staged: Vec<StagedWrite>,
    journal_path: PathBuf,
    lock_path: PathBuf,
    _lock: Option<fd_lock::RwLock<File>>,
    backup_enabled: bool,
    interrupted: Arc<AtomicBool>,
}

impl AtomicBatch {
    /// Create a new atomic batch. Acquires the lock file immediately.
    /// `targets` is used to determine the journal directory.
    /// `backup_enabled` controls whether hard-link backups are created.
    /// `interrupted` is an AtomicBool set by signal handlers.
    pub fn new(
        targets: &[PathBuf],
        backup_enabled: bool,
        interrupted: Arc<AtomicBool>,
    ) -> io::Result<Self> {
        let dir = journal::journal_dir(targets)?;
        let lock_path = dir.join(LOCK_FILENAME);
        let journal_path = dir.join(JOURNAL_FILENAME);

        // Acquire exclusive lock.
        // We keep the RwLock (and its write guard implicitly via try_write)
        // alive for the lifetime of the batch by storing the RwLock itself.
        let lock_file = File::create(&lock_path)?;
        let mut lock = fd_lock::RwLock::new(lock_file);
        // Test that we can acquire the lock; this will fail if another
        // atomic operation is running. The write guard is dropped immediately,
        // but the underlying file descriptor (held by the RwLock) keeps the
        // advisory lock on some platforms. We re-acquire below.
        {
            let _guard = lock.try_write().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Another atomic operation is already in progress in this directory. \
                     Wait for it to complete or remove .toggle-atomic.lock if the previous \
                     process crashed.",
                )
            })?;
            // Guard dropped here but we keep the RwLock (and its fd) alive
        }

        Ok(Self {
            staged: Vec::new(),
            journal_path,
            lock_path,
            _lock: Some(lock),
            backup_enabled,
            interrupted,
        })
    }

    /// Stage a single file write: write content to a temp file in the same
    /// directory as the target, fsync it, then release the fd.
    pub fn stage(&mut self, target_path: &Path, content: &[u8], _encoding: &str) -> io::Result<()> {
        let target_dir = target_path.parent().unwrap_or(Path::new("."));
        let mut tmp = NamedTempFile::new_in(target_dir)?;
        let encoded = content.to_vec();
        tmp.write_all(&encoded)?;
        platform::durable_sync(tmp.as_file())?;

        // Copy permissions from original file if it exists
        let original_permissions = if target_path.exists() {
            let meta = std::fs::metadata(target_path)?;
            let perms = meta.permissions();
            tmp.as_file().set_permissions(perms.clone()).ok();
            Some(perms)
        } else {
            None
        };

        let content_sha256 = journal::sha256_hex(&encoded);

        // Release the fd but keep the path for later rename
        let temp_path_obj = tmp.into_temp_path();
        let temp_path = temp_path_obj.to_path_buf();
        // Prevent TempPath from deleting the file on drop — we manage it ourselves
        temp_path_obj
            .keep()
            .map_err(|e| io::Error::other(format!("Failed to keep temp path: {}", e)))?;

        self.staged.push(StagedWrite {
            temp_path,
            target_path: target_path.to_path_buf(),
            content_sha256,
            original_permissions,
        });

        Ok(())
    }

    /// Emit a warning if the batch size exceeds the threshold.
    pub fn warn_if_large_batch(&self) {
        if self.staged.len() > BATCH_SIZE_WARNING_THRESHOLD {
            eprintln!(
                "Warning: Staging {} files in atomic mode. Large batches may be \
                 slow due to fsync overhead. Consider splitting into smaller \
                 batches if performance is critical.",
                self.staged.len()
            );
        }
    }

    /// Execute the two-phase commit: create backups, write journal, rename all.
    /// Returns Ok(()) if all renames succeed. On failure, attempts rollback
    /// if backups are enabled.
    pub fn commit(self) -> io::Result<()> {
        if self.staged.is_empty() {
            self.cleanup_lock();
            return Ok(());
        }

        self.warn_if_large_batch();

        // Build journal entries
        let mut journal_entries: Vec<JournalEntry> = Vec::with_capacity(self.staged.len());
        for sw in &self.staged {
            let backup_path = if self.backup_enabled {
                let mut bp = sw.target_path.as_os_str().to_os_string();
                bp.push(ATOMIC_BACKUP_EXT);
                Some(PathBuf::from(bp))
            } else {
                None
            };
            journal_entries.push(JournalEntry {
                target_path: sw.target_path.clone(),
                temp_path: sw.temp_path.clone(),
                backup_path,
                content_sha256: sw.content_sha256.clone(),
                rename_completed: false,
            });
        }

        let mut j = Journal::new(journal_entries, self.backup_enabled);

        // Persist journal in Staged state
        journal::persist_journal(&j, &self.journal_path)?;

        // Create hard-link backups if enabled
        if self.backup_enabled {
            for entry in &j.entries {
                if let Some(ref backup_path) = entry.backup_path {
                    if entry.target_path.exists() {
                        if let Err(e) = std::fs::hard_link(&entry.target_path, backup_path) {
                            eprintln!(
                                "Error: failed to create backup for '{}': {}",
                                entry.target_path.display(),
                                e
                            );
                            self.rollback_staged(&j);
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Transition to Committing
        j.transition_to_committing();
        journal::persist_journal(&j, &self.journal_path)?;

        if !self.backup_enabled {
            eprintln!(
                "Warning: Running without backups. If the rename phase fails, \
                 rollback is not possible."
            );
        }

        // Phase 2: Rename all temp files to targets
        let entry_count = j.entries.len();
        for idx in 0..entry_count {
            // Check for signal interrupt between renames
            if self.interrupted.load(Ordering::Relaxed) {
                eprintln!("Interrupted. Journal preserved for recovery.");
                journal::persist_journal(&j, &self.journal_path)?;
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Atomic commit interrupted by signal. \
                     Run with --recover to clean up.",
                ));
            }

            let temp_path = j.entries[idx].temp_path.clone();
            let target_path = j.entries[idx].target_path.clone();

            // Copy permissions before rename
            if let Some(ref perms) = self.staged[idx].original_permissions {
                let _ = std::fs::set_permissions(&temp_path, perms.clone());
            }

            match platform::rename_with_retry(&temp_path, &target_path) {
                Ok(()) => {
                    j.mark_entry_completed(idx);
                    journal::persist_journal_best_effort(&j, &self.journal_path);
                }
                Err(e) => {
                    eprintln!(
                        "Error: rename failed for '{}': {}",
                        target_path.display(),
                        e
                    );
                    if self.backup_enabled {
                        eprintln!("Attempting rollback...");
                        if let Err(rb_err) = journal::recover_rollback(&j, &self.journal_path) {
                            eprintln!("Rollback also failed: {}", rb_err);
                        }
                    } else {
                        let _ = journal::persist_journal(&j, &self.journal_path);
                        eprintln!(
                            "No backups available. Journal preserved at '{}' for manual recovery.",
                            self.journal_path.display()
                        );
                    }
                    return Err(e);
                }
            }
        }

        // Finalization: fsync parent directories
        let mut synced_dirs = std::collections::HashSet::new();
        for entry in &j.entries {
            if let Some(parent) = entry.target_path.parent() {
                if synced_dirs.insert(parent.to_path_buf()) {
                    let _ = platform::sync_dir(parent);
                }
            }
        }

        // Delete journal
        journal::delete_journal(&self.journal_path)?;

        // Clean up atomic backup files
        if self.backup_enabled {
            for entry in &j.entries {
                if let Some(ref backup_path) = entry.backup_path {
                    let _ = std::fs::remove_file(backup_path);
                }
            }
        }

        self.cleanup_lock();
        Ok(())
    }

    /// Rollback from Staged state: delete all temp files and backups, delete journal.
    fn rollback_staged(&self, journal: &Journal) {
        for entry in &journal.entries {
            if entry.temp_path.exists() {
                let _ = std::fs::remove_file(&entry.temp_path);
            }
            if let Some(ref backup_path) = entry.backup_path {
                if backup_path.exists() {
                    let _ = std::fs::remove_file(backup_path);
                }
            }
        }
        let _ = journal::delete_journal(&self.journal_path);
        self.cleanup_lock();
    }

    /// Clean up the lock file.
    fn cleanup_lock(&self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for AtomicBatch {
    fn drop(&mut self) {
        // If we're being dropped without commit() having cleaned up,
        // the lock file should still be removed.
        // Note: staged temp files are NOT cleaned up on drop since we called
        // keep() on them. The journal (if written) provides recovery info.
    }
}

/// Trait abstracting filesystem operations for testability.
/// Production code uses `RealFileOps`; tests can inject failures.
pub trait FileOps {
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn hard_link(&self, src: &Path, dst: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn sync_dir(&self, path: &Path) -> io::Result<()>;
}

/// Production filesystem operations using std::fs.
pub struct RealFileOps;

impl FileOps for RealFileOps {
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        platform::rename_with_retry(from, to)
    }

    fn hard_link(&self, src: &Path, dst: &Path) -> io::Result<()> {
        std::fs::hard_link(src, dst)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn sync_dir(&self, path: &Path) -> io::Result<()> {
        platform::sync_dir(path)
    }
}
