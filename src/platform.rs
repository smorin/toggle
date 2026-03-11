// Platform-specific helpers for atomic file operations

use std::fs::File;
use std::io;
use std::path::Path;

/// Perform a durable fsync that guarantees data reaches persistent storage.
/// On macOS, uses F_FULLFSYNC (fcntl) because fsync() only flushes to the
/// disk write cache, not to the physical media.
/// On other platforms, uses fdatasync (sync_data) which is cheaper than
/// sync_all since it skips timestamp metadata updates.
#[cfg(target_os = "macos")]
pub fn durable_sync(file: &File) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_FULLFSYNC) };
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn durable_sync(file: &File) -> io::Result<()> {
    file.sync_data()
}

/// Fsync a directory to ensure metadata (directory entries) is persisted.
/// This is important after rename operations to ensure durability.
/// On Windows, this is a no-op since NTFS journals directory metadata.
#[cfg(unix)]
pub fn sync_dir(path: &Path) -> io::Result<()> {
    let dir = File::open(path)?;
    durable_sync(&dir)
}

#[cfg(not(unix))]
pub fn sync_dir(_path: &Path) -> io::Result<()> {
    // Windows NTFS journals directory metadata; explicit sync not needed.
    Ok(())
}

/// Perform an atomic rename with platform-specific retry logic.
/// On Windows, antivirus and search indexer can briefly lock files,
/// so we retry with backoff on sharing violations.
#[cfg(windows)]
pub fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    use std::thread;
    use std::time::Duration;

    let delays = [50, 100, 200];
    let mut last_err = None;

    for (attempt, delay_ms) in std::iter::once(&0u64).chain(delays.iter()).enumerate() {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(*delay_ms));
        }
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                // ERROR_SHARING_VIOLATION = 32
                if e.raw_os_error() == Some(32) {
                    last_err = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(last_err
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "rename failed after retries")))
}

#[cfg(not(windows))]
pub fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)
}

/// Resolve symlinks to their canonical target path.
/// Atomic operations should operate on the real file, not the symlink entry.
pub fn resolve_symlinks(path: &Path) -> io::Result<std::path::PathBuf> {
    std::fs::canonicalize(path)
}
