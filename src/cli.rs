// Command-line interface for the Toggle CLI

use clap::Parser;
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "toggle", author, version, about)]
pub struct Cli {
    /// File or directory paths to process
    pub paths: Vec<PathBuf>,

    /// Line range in format <start_line>:<end_line> or <start_line>:+<count> (repeatable)
    #[arg(short = 'l', long = "line", action = clap::ArgAction::Append)]
    pub lines: Vec<String>,

    /// Section ID to toggle. Use `group:variant` (e.g. `db:postgres`) for variant ops:
    /// `-S group` flips a 2-variant pair; `-S group:variant` activates one variant
    /// and comments siblings; `-S group --force on/off` applies to every variant.
    #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
    pub sections: Vec<String>,

    /// Recursively walk directories
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// List all section IDs found in files (discovery mode, no toggling)
    #[arg(long = "list-sections")]
    pub list_sections: bool,

    /// Force toggle state (on/off/invert)
    #[arg(short = 'f', long = "force", visible_short_alias = 'F')]
    pub force: Option<String>,

    /// Comment mode (auto/single/multi)
    #[arg(short = 'm', long = "mode", default_value = "auto")]
    pub mode: String,

    /// Human-readable log lines to stderr
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Machine-readable single-line JSON to stdout
    #[arg(long = "json")]
    pub json: bool,

    /// Extension for atomic temp file
    #[arg(short = 't', long = "temp-suffix")]
    pub temp_suffix: Option<String>,

    /// Override file codec (UTF-8 only in Phase 0)
    #[arg(short = 'e', long = "encoding", default_value = "utf-8")]
    pub encoding: String,

    /// Error if target is not .py
    #[arg(long = "strict-ext")]
    pub strict_ext: bool,

    /// Operate on symlink itself instead of target
    #[arg(short = 'N', long = "no-dereference")]
    pub no_dereference: bool,

    /// EOL normalization: preserve, lf, or crlf
    #[arg(long = "eol", default_value = "preserve")]
    pub eol: String,

    /// Map exit codes to sysexits.h values
    #[arg(short = 'x', long = "posix-exit")]
    pub posix_exit: bool,

    /// Override comment style: SINGLE [MULTI_START MULTI_END]
    #[arg(long = "comment-style", num_args = 1..=3, value_names = ["SINGLE", "MULTI_START", "MULTI_END"])]
    pub comment_style: Vec<String>,

    /// Prompt before modifying each file
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Show diff of changes without writing files
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Create backup with given extension before modifying (e.g. --backup .bak)
    #[arg(long = "backup")]
    pub backup: Option<String>,

    /// Extend the last --line range to the end of file
    #[arg(long = "to-end")]
    pub to_end: bool,

    /// Scan for section IDs without modifying files
    #[arg(long = "scan")]
    pub scan: bool,

    /// Validate section integrity without modifying files. Requires --scan.
    #[arg(long = "check")]
    pub check: bool,

    /// Enforce exactly 2 variants in the targeted group; error otherwise.
    /// Pre-execution check — no file modifications occur on failure.
    #[arg(long = "pair")]
    pub pair: bool,

    /// Path to .toggleConfig TOML file
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Enable atomic multi-file mode: all files succeed or none are modified.
    /// Implies --backup unless --no-backup is explicitly passed.
    #[arg(long = "atomic")]
    pub atomic: bool,

    /// Disable backup creation in atomic mode. Only valid with --atomic.
    /// WARNING: Without backups, rollback is not possible if the rename phase fails.
    #[arg(long = "no-backup")]
    pub no_backup: bool,

    /// Recover from an interrupted atomic operation. Default: rollback.
    #[arg(long = "recover")]
    pub recover: bool,

    /// Complete an interrupted atomic commit instead of rolling back.
    /// Must be combined with --recover.
    #[arg(long = "recover-forward")]
    pub recover_forward: bool,

    /// Generate shell completions for the given shell to stdout.
    /// Example: `toggle --completions bash > /etc/bash_completion.d/toggle`
    #[arg(long = "completions", value_name = "SHELL")]
    pub completions: Option<Shell>,

    /// Generate a roff-formatted man page to stdout.
    /// Example: `toggle --man > toggle.1 && man ./toggle.1`
    #[arg(long = "man")]
    pub man: bool,
}
