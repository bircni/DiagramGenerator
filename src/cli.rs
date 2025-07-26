use std::path::PathBuf;

use clap::Parser;
use log::LevelFilter;

#[derive(Parser)]
#[command(author, version, about)]
/// Generate a diagram from Rust source code
pub struct Cli {
    /// Path to main.rs or lib.rs or the root of the crate
    #[clap(short, long)]
    pub path: Option<PathBuf>,
    /// Path to output the diagram
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    /// Log Level Filter [Debug, Info, Error, Warn]
    #[clap(short, long, default_value = "Info")]
    pub loglevel: LevelFilter,
    /// Name of the Diagram
    #[clap(short, long, default_value = "Diagram")]
    pub name: String,
    /// Include test functions in the diagram (excluded by default)
    #[clap(short = 't', long, default_value = "false")]
    pub include_tests: bool,
}
