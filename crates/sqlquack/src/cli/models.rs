//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::census::constants::DEFAULT_TEXT_LIMIT;

/// Snowflake-to-DuckDB SQL conversion that refuses to guess.
#[derive(Debug, Parser)]
#[command(name = "sqlquack", version)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// `sqlquack` commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Count the constructs SQL files use and estimate how many files convert as rules are added.
    Census(CensusArguments),
}

/// Arguments of `sqlquack census`.
#[derive(Debug, Args)]
pub struct CensusArguments {
    /// SQL files, or directories searched for `.sql` files: one query or compiled model per file.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
    /// The dialect the SQL is written in.
    #[arg(long, default_value = "snowflake")]
    pub dialect: String,
    /// Print the report as JSON on standard output.
    #[arg(long)]
    pub json: bool,
    /// Constructs listed in the text report.
    #[arg(long, default_value_t = DEFAULT_TEXT_LIMIT)]
    pub limit: usize,
}
