use std::fmt;

/// Typed error for bad CLI input / range errors (maps to ExitCode::Usage).
/// Use this instead of bare `anyhow!()` for usage errors so that
/// `classify_error` can downcast instead of matching on message strings.
#[derive(Debug)]
pub struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

/// Exit codes per Phase 0 PRD §0.8
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    /// EC00: Success
    Success = 0,
    /// EC01: Bad CLI / range
    Usage = 1,
    /// EC02: File R/W error
    IoError = 2,
    /// EC03: Toggle logic issue
    ToggleError = 3,
    /// EC04: Internal panic (reserved for future panic hook, not yet wired)
    #[allow(dead_code)]
    Internal = 4,
}

impl ExitCode {
    /// Map to sysexits.h values for --posix-exit
    pub fn posix(self) -> i32 {
        match self {
            Self::Success => 0,      // EX_OK
            Self::Usage => 64,       // EX_USAGE
            Self::IoError => 74,     // EX_IOERR
            Self::ToggleError => 70, // EX_SOFTWARE
            Self::Internal => 71,    // EX_OSERR
        }
    }

    /// Get the numeric value
    pub fn code(self) -> i32 {
        self as i32
    }
}
