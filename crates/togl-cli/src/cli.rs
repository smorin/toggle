// Command-line interface for the Toggle CLI

use clap::Parser;
use clap_complete::Shell;
use std::path::PathBuf;

/// Output detail level for `--list-sections` (P07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ListFields {
    /// Section IDs and descriptions only.
    Ids,
    /// IDs plus each file path (no line numbers).
    Files,
    /// IDs plus `file:start-end` (current default behavior).
    Lines,
}

/// What `--remove` strips from a matched section (P06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RemoveMode {
    /// Delete only the two marker lines; keep the body.
    Markers,
    /// Delete the markers and fully-commented body lines; keep live code.
    Commented,
    /// Delete the entire span (markers + body, including live code).
    All,
}

#[derive(Parser)]
#[command(author, version, about, args_conflicts_with_subcommands = true)]
pub struct Cli {
    /// Subcommand form (e.g. `togl scan PATH`). Optional: when omitted, the
    /// legacy flat flags below are used. Subcommands are translated to the
    /// equivalent flat flags and run through the same pipeline (see
    /// `Commands::to_legacy_argv`), so behavior is identical by construction.
    #[command(subcommand)]
    pub command: Option<Commands>,

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
    #[arg(long = "list-sections", group = "operation")]
    pub list_sections: bool,

    /// Detail level for --list-sections text output [default: lines]
    #[arg(long = "fields", default_value = "lines")]
    pub fields: ListFields,

    /// Remove a named section (requires -S <ID>). Recursive with -R. See --remove-mode.
    #[arg(long = "remove", group = "operation")]
    pub remove: bool,

    /// What --remove strips: markers | commented | all [default: commented]
    #[arg(long = "remove-mode", default_value = "commented")]
    pub remove_mode: RemoveMode,

    /// With --remove, exit non-zero if -S <ID> matched no sections.
    #[arg(long = "require-match", requires = "remove")]
    pub require_match: bool,

    /// Insert a toggle:start/end marker pair around a single -l range (single file).
    /// Requires exactly one -S <ID> and one -l <range>. Leaves the body uncommented.
    #[arg(long = "insert", group = "operation")]
    pub insert: bool,

    /// Description for the inserted section marker (use with --insert).
    #[arg(long = "desc", requires = "insert")]
    pub desc: Option<String>,

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
    #[arg(long = "to-end", requires = "lines")]
    pub to_end: bool,

    /// Scan for section IDs without modifying files
    #[arg(long = "scan", group = "operation")]
    pub scan: bool,

    /// Validate section integrity without modifying files. Requires --scan.
    #[arg(long = "check", requires = "scan")]
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
    #[arg(long = "no-backup", requires = "atomic")]
    pub no_backup: bool,

    /// Recover from an interrupted atomic operation. Default: rollback.
    #[arg(long = "recover")]
    pub recover: bool,

    /// Complete an interrupted atomic commit instead of rolling back.
    /// Must be combined with --recover.
    #[arg(long = "recover-forward", requires = "recover")]
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

// ── Subcommand front-end (additive over the legacy flat flags) ──
//
// Each subcommand offers a scoped, ergonomic surface (only the flags that apply
// to that operation). Rather than duplicate the run-pipeline logic, every
// subcommand translates itself into the equivalent *legacy* argv via
// `to_legacy_argv`, which `main()` re-parses through the very same
// `build_command()` path it already uses. clap therefore remains the single
// source of truth for defaults, validation, and binary-name aliasing — so the
// subcommand and flat-flag forms cannot drift apart.

use std::ffi::OsString;

/// Flags shared by every subcommand. Flattened into each variant so the legacy
/// definitions on `Cli` stay untouched (additive, not a restructure).
#[derive(clap::Args, Debug)]
pub struct GlobalArgs {
    /// Human-readable log lines to stderr
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Machine-readable single-line JSON to stdout
    #[arg(long = "json")]
    pub json: bool,

    /// Comment mode (auto/single/multi)
    #[arg(short = 'm', long = "mode", default_value = "auto")]
    pub mode: String,

    /// Override file codec (UTF-8 only in Phase 0)
    #[arg(short = 'e', long = "encoding", default_value = "utf-8")]
    pub encoding: String,

    /// EOL normalization: preserve, lf, or crlf
    #[arg(long = "eol", default_value = "preserve")]
    pub eol: String,

    /// Map exit codes to sysexits.h values
    #[arg(short = 'x', long = "posix-exit")]
    pub posix_exit: bool,

    /// Operate on symlink itself instead of target
    #[arg(short = 'N', long = "no-dereference")]
    pub no_dereference: bool,

    /// Error if target is not .py
    #[arg(long = "strict-ext")]
    pub strict_ext: bool,

    /// Prompt before modifying each file
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Show diff of changes without writing files
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Extension for atomic temp file
    #[arg(short = 't', long = "temp-suffix")]
    pub temp_suffix: Option<String>,

    /// Create backup with given extension before modifying (e.g. --backup .bak)
    #[arg(long = "backup")]
    pub backup: Option<String>,

    /// Path to .toggleConfig TOML file
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Override comment style: SINGLE [MULTI_START MULTI_END]
    #[arg(long = "comment-style", num_args = 1..=3, value_names = ["SINGLE", "MULTI_START", "MULTI_END"])]
    pub comment_style: Vec<String>,
}

impl GlobalArgs {
    /// Emit the legacy flags for the globals the user effectively chose.
    /// Defaulted string values are emitted only when non-default, keeping the
    /// synthesized argv minimal (and behavior-identical either way).
    fn push_argv(&self, out: &mut Vec<OsString>) {
        if self.verbose {
            out.push("--verbose".into());
        }
        if self.json {
            out.push("--json".into());
        }
        if self.mode != "auto" {
            out.push("--mode".into());
            out.push((&self.mode).into());
        }
        if self.encoding != "utf-8" {
            out.push("--encoding".into());
            out.push((&self.encoding).into());
        }
        if self.eol != "preserve" {
            out.push("--eol".into());
            out.push((&self.eol).into());
        }
        if self.posix_exit {
            out.push("--posix-exit".into());
        }
        if self.no_dereference {
            out.push("--no-dereference".into());
        }
        if self.strict_ext {
            out.push("--strict-ext".into());
        }
        if self.interactive {
            out.push("--interactive".into());
        }
        if self.dry_run {
            out.push("--dry-run".into());
        }
        if let Some(t) = &self.temp_suffix {
            out.push("--temp-suffix".into());
            out.push(t.into());
        }
        if let Some(b) = &self.backup {
            out.push("--backup".into());
            out.push(b.into());
        }
        if let Some(c) = &self.config {
            out.push("--config".into());
            out.push(c.into());
        }
        if !self.comment_style.is_empty() {
            out.push("--comment-style".into());
            for v in &self.comment_style {
                out.push(v.into());
            }
        }
    }
}

/// Ergonomic subcommand surface. Each maps to a legacy operation mode.
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Toggle comments on sections or line ranges (default operation).
    Toggle {
        /// File or directory paths to process
        paths: Vec<PathBuf>,
        /// Section ID(s) to toggle (repeatable). Use `group:variant` for variants.
        #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
        sections: Vec<String>,
        /// Line range <start>:<end> or <start>:+<count> (repeatable)
        #[arg(short = 'l', long = "line", action = clap::ArgAction::Append)]
        lines: Vec<String>,
        /// Recursively walk directories
        #[arg(short = 'R', long = "recursive")]
        recursive: bool,
        /// Force toggle state (on/off/invert)
        #[arg(short = 'f', long = "force", visible_short_alias = 'F')]
        force: Option<String>,
        /// Extend the last --line range to the end of file
        #[arg(long = "to-end")]
        to_end: bool,
        /// Require exactly 2 variants in the targeted group; error otherwise.
        #[arg(long = "pair")]
        pair: bool,
        /// Enable atomic multi-file mode: all files succeed or none are modified.
        #[arg(long = "atomic")]
        atomic: bool,
        /// Disable backup creation in atomic mode (only valid with --atomic).
        #[arg(long = "no-backup")]
        no_backup: bool,
        #[command(flatten)]
        global: GlobalArgs,
    },
    /// Scan for section IDs without modifying files (read-only).
    Scan {
        /// File or directory paths to scan
        paths: Vec<PathBuf>,
        /// Limit/expand to specific section ID(s) for the detailed group view.
        #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
        sections: Vec<String>,
        /// Use the recursive summary view.
        #[arg(short = 'R', long = "recursive")]
        recursive: bool,
        /// Require exactly 2 variants in the targeted group.
        #[arg(long = "pair")]
        pair: bool,
        #[command(flatten)]
        global: GlobalArgs,
    },
    /// Validate section integrity without modifying files (scan + check).
    Check {
        /// File or directory paths to validate
        paths: Vec<PathBuf>,
        /// Limit validation to specific section ID(s).
        #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
        sections: Vec<String>,
        /// Enforce exactly 2 variants in each targeted group.
        #[arg(long = "pair")]
        pair: bool,
        #[command(flatten)]
        global: GlobalArgs,
    },
    /// List all section IDs found in files (discovery mode).
    List {
        /// File or directory paths to list
        paths: Vec<PathBuf>,
        /// Recursively walk directories
        #[arg(short = 'R', long = "recursive")]
        recursive: bool,
        /// Detail level: ids | files | lines
        #[arg(long = "fields", default_value = "lines")]
        fields: ListFields,
        #[command(flatten)]
        global: GlobalArgs,
    },
    /// Insert a toggle:start/end marker pair around a single line range.
    Insert {
        /// Single file to modify
        path: PathBuf,
        /// Section ID for the inserted marker (exactly one)
        #[arg(short = 'S', long = "section")]
        section: String,
        /// Line range to wrap (exactly one)
        #[arg(short = 'l', long = "line")]
        line: String,
        /// Description for the inserted section marker
        #[arg(long = "desc")]
        desc: Option<String>,
        /// Extend the range to the end of file
        #[arg(long = "to-end")]
        to_end: bool,
        #[command(flatten)]
        global: GlobalArgs,
    },
    /// Remove a named section (markers and/or body) from files.
    Remove {
        /// File or directory paths to process
        paths: Vec<PathBuf>,
        /// Section ID to remove (exactly one)
        #[arg(short = 'S', long = "section")]
        section: String,
        /// Recursively walk directories
        #[arg(short = 'R', long = "recursive")]
        recursive: bool,
        /// What to strip: markers | commented | all
        #[arg(long = "remove-mode", default_value = "commented")]
        remove_mode: RemoveMode,
        /// Exit non-zero if -S <ID> matched no sections.
        #[arg(long = "require-match")]
        require_match: bool,
        #[command(flatten)]
        global: GlobalArgs,
    },
}

/// Canonical kebab-case name for a `ValueEnum` value (e.g. `Lines` -> "lines").
fn enum_name<E: clap::ValueEnum>(v: &E) -> OsString {
    v.to_possible_value()
        .expect("ValueEnum variant is not skipped")
        .get_name()
        .into()
}

impl Commands {
    /// Translate this subcommand into the equivalent legacy argv, with `bin` as
    /// argv[0]. The result is fed back through `build_command()` so clap owns
    /// defaults/validation/aliasing — making subcommand/flat-flag drift
    /// structurally impossible.
    pub fn to_legacy_argv(&self, bin: OsString) -> Vec<OsString> {
        let mut out: Vec<OsString> = vec![bin];
        match self {
            Commands::Toggle {
                paths,
                sections,
                lines,
                recursive,
                force,
                to_end,
                pair,
                atomic,
                no_backup,
                global,
            } => {
                for s in sections {
                    out.push("-S".into());
                    out.push(s.into());
                }
                for l in lines {
                    out.push("-l".into());
                    out.push(l.into());
                }
                if *recursive {
                    out.push("--recursive".into());
                }
                if let Some(f) = force {
                    out.push("--force".into());
                    out.push(f.into());
                }
                if *to_end {
                    out.push("--to-end".into());
                }
                if *pair {
                    out.push("--pair".into());
                }
                if *atomic {
                    out.push("--atomic".into());
                }
                if *no_backup {
                    out.push("--no-backup".into());
                }
                global.push_argv(&mut out);
                push_paths(&mut out, paths);
            }
            Commands::Scan {
                paths,
                sections,
                recursive,
                pair,
                global,
            } => {
                out.push("--scan".into());
                for s in sections {
                    out.push("-S".into());
                    out.push(s.into());
                }
                if *recursive {
                    out.push("--recursive".into());
                }
                if *pair {
                    out.push("--pair".into());
                }
                global.push_argv(&mut out);
                push_paths(&mut out, paths);
            }
            Commands::Check {
                paths,
                sections,
                pair,
                global,
            } => {
                out.push("--scan".into());
                out.push("--check".into());
                for s in sections {
                    out.push("-S".into());
                    out.push(s.into());
                }
                if *pair {
                    out.push("--pair".into());
                }
                global.push_argv(&mut out);
                push_paths(&mut out, paths);
            }
            Commands::List {
                paths,
                recursive,
                fields,
                global,
            } => {
                out.push("--list-sections".into());
                if *recursive {
                    out.push("--recursive".into());
                }
                if *fields != ListFields::Lines {
                    out.push("--fields".into());
                    out.push(enum_name(fields));
                }
                global.push_argv(&mut out);
                push_paths(&mut out, paths);
            }
            Commands::Insert {
                path,
                section,
                line,
                desc,
                to_end,
                global,
            } => {
                out.push("--insert".into());
                out.push("-S".into());
                out.push(section.into());
                out.push("-l".into());
                out.push(line.into());
                if let Some(d) = desc {
                    out.push("--desc".into());
                    out.push(d.into());
                }
                if *to_end {
                    out.push("--to-end".into());
                }
                global.push_argv(&mut out);
                out.push("--".into());
                out.push(path.into());
            }
            Commands::Remove {
                paths,
                section,
                recursive,
                remove_mode,
                require_match,
                global,
            } => {
                out.push("--remove".into());
                out.push("-S".into());
                out.push(section.into());
                if *recursive {
                    out.push("--recursive".into());
                }
                if *remove_mode != RemoveMode::Commented {
                    out.push("--remove-mode".into());
                    out.push(enum_name(remove_mode));
                }
                if *require_match {
                    out.push("--require-match".into());
                }
                global.push_argv(&mut out);
                push_paths(&mut out, paths);
            }
        }
        out
    }
}

/// Push positional paths after a `--` guard so a path starting with `-` (or a
/// path literally named like a flag) is never misread as an option.
fn push_paths(out: &mut Vec<OsString>, paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    out.push("--".into());
    for p in paths {
        out.push(p.into());
    }
}
