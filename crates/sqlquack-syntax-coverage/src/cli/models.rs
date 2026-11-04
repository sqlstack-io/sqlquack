//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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

/// Where the files are, and how to print.
#[derive(Debug, Args)]
pub struct Common {
    /// The coverage root.
    #[arg(long, default_value = COVERAGE_ROOT)]
    pub root: PathBuf,
    /// Print JSON on stdout alone.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load every coverage file and report format problems.
    Validate {
        #[command(flatten)]
        common: Common,
    },
    /// Counts for the whole catalogue and per family; fails on any unresolved reference.
    Summary {
        #[command(flatten)]
        common: Common,
    },
    /// Every form of one family with its obligation completion and reasons.
    Family {
        family: String,
        #[command(flatten)]
        common: Common,
    },
    /// Every unsatisfied required obligation, with its reason.
    Missing {
        #[arg(long)]
        family: Option<String>,
        #[command(flatten)]
        common: Common,
    },
    /// Counts per documentation page, a secondary view of the same forms.
    Pages {
        #[arg(long)]
        family: Option<String>,
        #[command(flatten)]
        common: Common,
    },
}
