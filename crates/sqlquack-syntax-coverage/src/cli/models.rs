//! Command-line arguments.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::formats::constants::COVERAGE_ROOT;

#[derive(Debug, Parser)]
#[command(
    name = "syntax-coverage",
    about = "Checks documented Snowflake syntax coverage."
)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load every coverage file and report format problems.
    Validate {
        /// The coverage root.
        #[arg(long, default_value = COVERAGE_ROOT)]
        root: PathBuf,
    },
}
