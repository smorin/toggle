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
    /// EC04: Internal panic
    Internal = 4,
}

impl ExitCode {
    /// Map to sysexits.h values for --posix-exit
    pub fn posix(self) -> i32 {
        match self {
            Self::Success => 0,    // EX_OK
            Self::Usage => 64,     // EX_USAGE
            Self::IoError => 74,   // EX_IOERR
            Self::ToggleError => 70, // EX_SOFTWARE
            Self::Internal => 71,  // EX_OSERR
        }
    }

    /// Get the numeric value
    pub fn code(self) -> i32 {
        self as i32
    }
}
