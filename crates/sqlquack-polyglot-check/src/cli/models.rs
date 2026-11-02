//! Command-line arguments.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// `sqlquack-polyglot-check` arguments.
#[derive(Debug, Parser)]
#[command(about = "Check that SQLQuack's parser gives exactly polyglot's results")]
pub struct Arguments {
    /// What to compare.
    #[command(subcommand)]
    pub corpus: CorpusArgument,
    /// Print counts and timings only, never SQL, trees or file names (for private corpora).
    #[arg(long, global = true)]
    pub counts_only: bool,
    /// After comparing, time each parser over the whole corpus this many times and print medians.
    #[arg(long, global = true, default_value_t = 0)]
    pub timing_rounds: usize,
    /// The reviewed list of call spellings SQLQuack keeps where polyglot lowers them.
    #[arg(
        long,
        global = true,
        default_value = "corpora/polyglot-check/expected_differences.toml"
    )]
    pub expected: PathBuf,
    /// Also print each kept spelling with the node polyglot lowers it to and its call count.
    #[arg(long, global = true)]
    pub spellings: bool,
}

/// The corpus to compare.
#[derive(Debug, Subcommand)]
pub enum CorpusArgument {
    /// Vendored corpus files.
    Files {
        /// The corpus JSON files.
        paths: Vec<PathBuf>,
    },
    /// Every `.sql` file below a directory, one entry per file.
    Directory {
        /// The directory.
        path: PathBuf,
        /// The dialect its files are written in.
        #[arg(long, value_enum)]
        dialect: DialectArgument,
    },
    /// SQLQuack's cases: source SQL and converted DuckDB SQL.
    Cases {
        /// The case directory.
        #[arg(long, default_value = "cases")]
        cases: PathBuf,
        /// The policy file.
        #[arg(long, default_value = "sqlquack.toml")]
        policy: PathBuf,
    },
}

/// A dialect on the command line.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DialectArgument {
    /// Snowflake.
    Snowflake,
    /// DuckDB.
    Duckdb,
    /// PostgreSQL.
    Postgres,
    /// SQLite.
    Sqlite,
}
