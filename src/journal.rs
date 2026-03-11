// Write-ahead journal for atomic multi-file operations.
//
// The journal records the state of an atomic batch operation so that
// if the process is interrupted (SIGTERM, SIGKILL, power loss), a
// subsequent run can detect the incomplete transaction and either
// roll back or complete it.

use crate::platform;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Name of the journal file placed in CWD (or fallback directory).
pub const JOURNAL_FILENAME: &str = ".toggle-atomic.journal";

/// Name of the lock file for concurrent execution prevention.
pub const LOCK_FILENAME: &str = ".toggle-atomic.lock";

/// Status of the atomic batch operation recorded in the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalStatus {
    /// Phase 1 complete: all temp files written, no originals touched yet.
    Staged,
    /// Phase 2 in progress: renaming temp files over originals.
    /// Some renames may have completed (check per-entry flags).
    Committing,
}

/// A single file entry in the journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// The final target path this file will be written to.
    pub target_path: PathBuf,
    /// Path to the staged temp file containing the new content.
    pub temp_path: PathBuf,
    /// Path to the backup (hard-link) of the original file.
    /// None if --no-backup was used.
    pub backup_path: Option<PathBuf>,
    /// SHA-256 hex digest of the temp file content for integrity verification.
    pub content_sha256: String,
    /// Whether this entry's rename (temp -> target) has completed.
    pub rename_completed: bool,
}

/// The write-ahead journal persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Current status of the atomic operation.
    pub status: JournalStatus,
    /// ISO 8601 timestamp of journal creation.
    pub created_at: String,
    /// Whether backups were enabled for this operation.
    pub backup_enabled: bool,
    /// Per-file entries tracking the state of each staged write.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// Create a new journal in Staged state.
    pub fn new(entries: Vec<JournalEntry>, backup_enabled: bool) -> Self {
        let now = chrono_lite_now();
        Self {
            version: 1,
            status: JournalStatus::Staged,
            created_at: now,
            backup_enabled,
            entries,
        }
    }

    /// Transition the journal to Committing state.
    pub fn transition_to_committing(&mut self) {
        self.status = JournalStatus::Committing;
    }

    /// Mark a specific entry as rename-completed.
    pub fn mark_entry_completed(&mut self, index: usize) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.rename_completed = true;
        }
    }
}

/// Compute SHA-256 hex digest of a byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 hex digest of a file on disk.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let data = std::fs::read(path)?;
    Ok(sha256_hex(&data))
}

/// Determine the journal directory. Prefers CWD; falls back to the first
/// target file's parent if CWD is not writable.
pub fn journal_dir(targets: &[PathBuf]) -> io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    // Test writability by attempting to create a temp file
    match NamedTempFile::new_in(&cwd) {
        Ok(_) => Ok(cwd),
        Err(_) => {
            // Fallback: first target's parent directory
            if let Some(first) = targets.first() {
                if let Some(parent) = first.parent() {
                    eprintln!(
                        "Warning: CWD is not writable. Using '{}' for journal.",
                        parent.display()
                    );
                    return Ok(parent.to_path_buf());
                }
            }
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Cannot create journal: CWD is not writable and no target files specified",
            ))
        }
    }
}

/// Persist the journal atomically: write to temp file, fsync, rename.
/// This ensures a half-written journal can never be read.
pub fn persist_journal(journal: &Journal, journal_path: &Path) -> io::Result<()> {
    let dir = journal_path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    let json = serde_json::to_string_pretty(journal)
        .map_err(|e| io::Error::other(format!("Failed to serialize journal: {}", e)))?;
    tmp.write_all(json.as_bytes())?;
    platform::durable_sync(tmp.as_file())?;
    tmp.persist(journal_path).map_err(|e| e.error)?;
    Ok(())
}

/// Best-effort journal update (e.g., progress tracking during commit loop).
/// Does not fsync — used for rename_completed updates where losing the
/// latest progress on crash is acceptable.
pub fn persist_journal_best_effort(journal: &Journal, journal_path: &Path) {
    let dir = journal_path.parent().unwrap_or(Path::new("."));
    if let Ok(mut tmp) = NamedTempFile::new_in(dir) {
        if let Ok(json) = serde_json::to_string_pretty(journal) {
            if tmp.write_all(json.as_bytes()).is_ok() {
                let _ = tmp.persist(journal_path);
            }
        }
    }
}

/// Read and parse a journal from disk. Returns None if the file doesn't exist.
/// Returns an error if the file exists but is corrupt/unparseable.
pub fn read_journal(journal_path: &Path) -> io::Result<Option<Journal>> {
    match std::fs::read_to_string(journal_path) {
        Ok(content) => {
            let journal: Journal = serde_json::from_str(&content).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Journal file is corrupted ({}). Manual inspection required: {}",
                        e,
                        journal_path.display()
                    ),
                )
            })?;
            Ok(Some(journal))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Delete the journal file.
pub fn delete_journal(journal_path: &Path) -> io::Result<()> {
    match std::fs::remove_file(journal_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Recover from a Staged journal: delete all temp files, delete journal.
/// No originals were touched, so this is always safe.
pub fn recover_staged(journal: &Journal, journal_path: &Path) -> io::Result<()> {
    eprintln!("Recovering from Staged state: cleaning up temp files...");
    for entry in &journal.entries {
        if entry.temp_path.exists() {
            if let Err(e) = std::fs::remove_file(&entry.temp_path) {
                eprintln!(
                    "Warning: failed to delete temp file '{}': {}",
                    entry.temp_path.display(),
                    e
                );
            }
        }
    }
    delete_journal(journal_path)?;
    eprintln!("Recovery complete. No original files were modified.");
    Ok(())
}

/// Recover from a Committing journal by rolling back: restore backups for
/// completed renames, delete remaining temp files, delete journal.
pub fn recover_rollback(journal: &Journal, journal_path: &Path) -> io::Result<()> {
    eprintln!("Recovering from Committing state: rolling back...");

    // Report current state
    let completed: Vec<_> = journal
        .entries
        .iter()
        .filter(|e| e.rename_completed)
        .collect();
    let pending: Vec<_> = journal
        .entries
        .iter()
        .filter(|e| !e.rename_completed)
        .collect();

    if !completed.is_empty() {
        eprintln!("  {} file(s) were renamed:", completed.len());
        for e in &completed {
            eprintln!("    {}", e.target_path.display());
        }
    }
    if !pending.is_empty() {
        eprintln!("  {} file(s) were NOT renamed:", pending.len());
        for e in &pending {
            eprintln!("    {}", e.target_path.display());
        }
    }

    if !journal.backup_enabled {
        eprintln!("Error: --no-backup was used. Cannot roll back completed renames automatically.");
        eprintln!("Manual intervention required for the files listed above.");
        // Still clean up temp files
        for entry in &pending {
            if entry.temp_path.exists() {
                let _ = std::fs::remove_file(&entry.temp_path);
            }
        }
        delete_journal(journal_path)?;
        return Err(io::Error::other(
            "Rollback impossible without backups. See output above for affected files.",
        ));
    }

    // Restore completed renames from backups (reverse order for safety)
    let mut errors = Vec::new();
    for entry in completed.iter().rev() {
        if let Some(ref backup_path) = entry.backup_path {
            if backup_path.exists() {
                if let Err(e) = platform::rename_with_retry(backup_path, &entry.target_path) {
                    errors.push(format!(
                        "Failed to restore '{}' from backup '{}': {}",
                        entry.target_path.display(),
                        backup_path.display(),
                        e
                    ));
                } else {
                    eprintln!("  Restored: {}", entry.target_path.display());
                }
            } else {
                errors.push(format!(
                    "Backup file missing for '{}': expected '{}'",
                    entry.target_path.display(),
                    backup_path.display()
                ));
            }
        }
    }

    // Delete remaining temp files
    for entry in &pending {
        if entry.temp_path.exists() {
            let _ = std::fs::remove_file(&entry.temp_path);
        }
    }

    if !errors.is_empty() {
        eprintln!("Rollback completed with errors:");
        for err in &errors {
            eprintln!("  {}", err);
        }
        // Keep journal if rollback partially failed
        return Err(io::Error::other(format!(
            "{} rollback error(s) occurred. Journal preserved.",
            errors.len()
        )));
    }

    // Clean up backup files for entries that were successfully restored
    for entry in &completed {
        if let Some(ref backup_path) = entry.backup_path {
            let _ = std::fs::remove_file(backup_path);
        }
    }

    delete_journal(journal_path)?;
    eprintln!("Rollback complete. All files restored to pre-operation state.");
    Ok(())
}

/// Forward recovery: complete the interrupted commit by renaming remaining
/// temp files to their targets.
pub fn recover_forward(journal: &Journal, journal_path: &Path) -> io::Result<()> {
    eprintln!("Forward recovery: completing interrupted commit...");

    let pending: Vec<(usize, &JournalEntry)> = journal
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.rename_completed)
        .collect();

    if pending.is_empty() {
        eprintln!("All renames were already completed. Cleaning up.");
        // Clean up backup files
        for entry in &journal.entries {
            if let Some(ref backup_path) = entry.backup_path {
                let _ = std::fs::remove_file(backup_path);
            }
        }
        delete_journal(journal_path)?;
        return Ok(());
    }

    eprintln!("  {} file(s) remaining to rename.", pending.len());

    let mut updated_journal = journal.clone();
    let mut errors = Vec::new();

    for (idx, entry) in &pending {
        // Verify temp file exists
        if !entry.temp_path.exists() {
            errors.push(format!(
                "Temp file missing for '{}': expected '{}'",
                entry.target_path.display(),
                entry.temp_path.display()
            ));
            continue;
        }

        // Verify SHA-256 integrity
        match sha256_file(&entry.temp_path) {
            Ok(hash) if hash == entry.content_sha256 => {}
            Ok(hash) => {
                errors.push(format!(
                    "SHA-256 mismatch for '{}': expected {}, got {}",
                    entry.temp_path.display(),
                    entry.content_sha256,
                    hash
                ));
                continue;
            }
            Err(e) => {
                errors.push(format!(
                    "Cannot read temp file '{}': {}",
                    entry.temp_path.display(),
                    e
                ));
                continue;
            }
        }

        // Copy permissions from target if it exists
        if entry.target_path.exists() {
            if let Ok(meta) = std::fs::metadata(&entry.target_path) {
                let _ = std::fs::set_permissions(&entry.temp_path, meta.permissions());
            }
        }

        // Perform the rename
        match platform::rename_with_retry(&entry.temp_path, &entry.target_path) {
            Ok(()) => {
                eprintln!("  Renamed: {}", entry.target_path.display());
                updated_journal.mark_entry_completed(*idx);
                persist_journal_best_effort(&updated_journal, journal_path);
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to rename '{}' -> '{}': {}",
                    entry.temp_path.display(),
                    entry.target_path.display(),
                    e
                ));
                // Stop on first rename failure during forward recovery
                break;
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("Forward recovery incomplete:");
        for err in &errors {
            eprintln!("  {}", err);
        }
        persist_journal(&updated_journal, journal_path)?;
        return Err(io::Error::other(format!(
            "{} error(s) during forward recovery. Journal preserved for retry.",
            errors.len()
        )));
    }

    // All renames succeeded. Clean up backups and journal.
    for entry in &journal.entries {
        if let Some(ref backup_path) = entry.backup_path {
            let _ = std::fs::remove_file(backup_path);
        }
    }
    delete_journal(journal_path)?;
    eprintln!("Forward recovery complete. All files updated.");
    Ok(())
}

/// Perform recovery based on journal state and user flags.
pub fn perform_recovery(journal_path: &Path, forward: bool) -> io::Result<()> {
    let journal = match read_journal(journal_path)? {
        Some(j) => j,
        None => {
            eprintln!("No journal found. Nothing to recover.");
            return Ok(());
        }
    };

    match journal.status {
        JournalStatus::Staged => {
            if forward {
                eprintln!(
                    "Warning: --recover-forward has no effect in Staged state. \
                     No renames occurred. Rolling back."
                );
            }
            recover_staged(&journal, journal_path)
        }
        JournalStatus::Committing => {
            if forward {
                recover_forward(&journal, journal_path)
            } else {
                recover_rollback(&journal, journal_path)
            }
        }
    }
}

/// Simple ISO 8601 timestamp without external chrono dependency.
fn chrono_lite_now() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}s-since-epoch", d.as_secs()),
        Err(_) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_journal_roundtrip() {
        let dir = TempDir::new().unwrap();
        let journal_path = dir.path().join(JOURNAL_FILENAME);

        let journal = Journal::new(
            vec![JournalEntry {
                target_path: PathBuf::from("/tmp/test.py"),
                temp_path: PathBuf::from("/tmp/.tmpXXXX"),
                backup_path: Some(PathBuf::from("/tmp/test.py.bak")),
                content_sha256: "abc123".to_string(),
                rename_completed: false,
            }],
            true,
        );

        persist_journal(&journal, &journal_path).unwrap();
        let loaded = read_journal(&journal_path).unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.status, JournalStatus::Staged);
        assert!(loaded.backup_enabled);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].target_path, PathBuf::from("/tmp/test.py"));
        assert_eq!(loaded.entries[0].content_sha256, "abc123");
        assert!(!loaded.entries[0].rename_completed);
    }

    #[test]
    fn test_journal_not_found() {
        let result = read_journal(Path::new("/nonexistent/.toggle-atomic.journal")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_journal_corrupt() {
        let dir = TempDir::new().unwrap();
        let journal_path = dir.path().join(JOURNAL_FILENAME);
        std::fs::write(&journal_path, "not valid json {{{").unwrap();
        let result = read_journal(&journal_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_staged_cleans_temps() {
        let dir = TempDir::new().unwrap();
        let temp_file = dir.path().join("temp_staged");
        std::fs::write(&temp_file, "staged content").unwrap();
        let journal_path = dir.path().join(JOURNAL_FILENAME);

        let journal = Journal::new(
            vec![JournalEntry {
                target_path: dir.path().join("target.py"),
                temp_path: temp_file.clone(),
                backup_path: None,
                content_sha256: "xxx".to_string(),
                rename_completed: false,
            }],
            false,
        );
        persist_journal(&journal, &journal_path).unwrap();

        recover_staged(&journal, &journal_path).unwrap();
        assert!(!temp_file.exists());
        assert!(!journal_path.exists());
    }

    #[test]
    fn test_status_transitions() {
        let mut journal = Journal::new(vec![], true);
        assert_eq!(journal.status, JournalStatus::Staged);
        journal.transition_to_committing();
        assert_eq!(journal.status, JournalStatus::Committing);
    }

    #[test]
    fn test_journal_with_unicode_paths() {
        let dir = TempDir::new().unwrap();
        let journal_path = dir.path().join(JOURNAL_FILENAME);

        let journal = Journal::new(
            vec![JournalEntry {
                target_path: PathBuf::from("/tmp/café/données.py"),
                temp_path: PathBuf::from("/tmp/café/.tmpXXXX"),
                backup_path: Some(PathBuf::from("/tmp/café/données.py.bak")),
                content_sha256: "abc".to_string(),
                rename_completed: false,
            }],
            true,
        );

        persist_journal(&journal, &journal_path).unwrap();
        let loaded = read_journal(&journal_path).unwrap().unwrap();
        assert_eq!(
            loaded.entries[0].target_path,
            PathBuf::from("/tmp/café/données.py")
        );
    }
}
