// Command-line interface for the Toggle CLI

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// File or directory paths to process
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Line range in format <start_line>:<end_line> or <start_line>:+<count>
    #[arg(short = 'l', long = "line")]
    pub line: Option<String>,

    /// Section ID to toggle
    #[arg(short = 'S', long = "section", action = clap::ArgAction::Append)]
    pub sections: Vec<String>,

    /// Force toggle state (on/off)
    #[arg(short = 'f', long = "force")]
    pub force: Option<String>,

    /// Comment mode (auto/single/multi)
    #[arg(short = 'm', long = "mode", default_value = "auto")]
    pub mode: String,
}
