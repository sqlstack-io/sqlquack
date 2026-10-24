//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use sqlquack_evidence::constants::{
    AUDIT_PENDING_DIR, AUDIT_RESULTS_DIR, CASES_DIR, PENDING_DIR, RESULTS_DIR,
};
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;

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
    /// Write a pending case file for every case without a recorded result.
    Pending(PendingArguments),
    /// Replay every recorded case on DuckDB and compare with the recorded result.
    Verify(VerifyArguments),
    /// Sample recorded cases for re-recording, or compare re-recorded results with stored ones.
    Audit(AuditArguments),
}

/// Where evidence lives.
#[derive(Debug, Args)]
pub struct EvidencePaths {
    /// The policy file.
    #[arg(long, default_value = POLICY_FILE_NAME)]
    pub policy: PathBuf,
    /// The authored case files.
    #[arg(long, default_value = CASES_DIR)]
    pub cases: PathBuf,
    /// The recorded results.
    #[arg(long, default_value = RESULTS_DIR)]
    pub results: PathBuf,
}

/// Arguments of `sqlquack pending`.
#[derive(Debug, Args)]
pub struct PendingArguments {
    /// Where evidence lives.
    #[command(flatten)]
    pub paths: EvidencePaths,
    /// Where pending case files are written.
    #[arg(long, default_value = PENDING_DIR)]
    pub out: PathBuf,
}

/// Arguments of `sqlquack verify`.
#[derive(Debug, Args)]
pub struct VerifyArguments {
    /// Where evidence lives.
    #[command(flatten)]
    pub paths: EvidencePaths,
    /// Print the report as JSON on standard output.
    #[arg(long)]
    pub json: bool,
}

/// Arguments of `sqlquack audit`.
#[derive(Debug, Args)]
pub struct AuditArguments {
    /// Where evidence lives.
    #[command(flatten)]
    pub paths: EvidencePaths,
    /// The share of recorded cases to re-record, such as `10%`.
    #[arg(long, conflicts_with = "compare", required_unless_present = "compare")]
    pub sample: Option<String>,
    /// The sampling seed; the current time when absent, and always printed.
    #[arg(long)]
    pub seed: Option<u64>,
    /// Where the sample's pending case files are written.
    #[arg(long, default_value = AUDIT_PENDING_DIR)]
    pub out: PathBuf,
    /// Compare the re-recorded results with the stored ones.
    #[arg(long)]
    pub compare: bool,
    /// Where the re-recorded results are.
    #[arg(long, default_value = AUDIT_RESULTS_DIR)]
    pub audit_results: PathBuf,
    /// Print the comparison as JSON on standard output.
    #[arg(long)]
    pub json: bool,
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
