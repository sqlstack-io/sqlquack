//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use sqlquack_evidence::constants::{
    AUDIT_PENDING_DIR, AUDIT_RESULTS_DIR, CASES_DIR, PENDING_DIR, RESULTS_DIR,
};
use sqlquack_evidence::coverage::constants::{DOMAINS_DIR, HAZARDS_FILE};
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
    /// Map building blocks, typing functions and rules to the cases and files relying on them.
    Index(IndexArguments),
    /// Write every case, its conversion and its recorded result as sqllogictest-style text.
    Review(ReviewArguments),
    /// Convert a SQL project function's body into a DuckDB macro body, printed as JSON.
    Function(FunctionArguments),
}

/// Arguments of `sqlquack function`.
#[derive(Debug, Args)]
pub struct FunctionArguments {
    /// The file holding the function's body: a scalar's expression, or a table function's query.
    pub body: PathBuf,
    /// The function's name, as the schema declares it.
    #[arg(long)]
    pub name: String,
    /// The schema file declaring the function and the relations its body reads.
    #[arg(long)]
    pub schema: PathBuf,
    /// The policy file whose known divergences conversion may rely on.
    #[arg(long, default_value = POLICY_FILE_NAME)]
    pub policy: PathBuf,
    /// Refuse every known divergence the policy refuses in strict mode.
    #[arg(long)]
    pub strict: bool,
}

/// Arguments of `sqlquack review`.
#[derive(Debug, Args)]
pub struct ReviewArguments {
    /// Where evidence lives.
    #[command(flatten)]
    pub paths: EvidencePaths,
    /// The directory the review files are written to.
    #[arg(long, default_value = "review")]
    pub out: PathBuf,
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
    /// Fail unless every equivalence and rule is relied on by at least one passed case.
    #[arg(long)]
    pub require_verified: bool,
    /// Fail when DuckDB's error differs in kind from Snowflake's, or no rule used declares it.
    #[arg(long)]
    pub strict_errors: bool,
    /// Re-recorded results, read for the messages of errors stored without one.
    #[arg(long, default_value = AUDIT_RESULTS_DIR)]
    pub audit_results: PathBuf,
    /// Where coverage is checked against.
    #[command(flatten)]
    pub coverage: CoverageArguments,
}

/// The coverage check `sqlquack verify` reports: declared rule domains and the hazard catalogue.
#[derive(Debug, Args)]
pub struct CoverageArguments {
    /// The directory of declared rule domains.
    #[arg(long, default_value = DOMAINS_DIR)]
    pub domains: PathBuf,
    /// The hazard catalogue.
    #[arg(long, default_value = HAZARDS_FILE)]
    pub hazards: PathBuf,
    /// Write the coverage report as JSON to this file.
    #[arg(long)]
    pub coverage_json: Option<PathBuf>,
    /// Count pending cases that convert as passed, to plan a recording.
    #[arg(long, conflicts_with = "require_coverage")]
    pub coverage_projected: bool,
    /// Fail on any coverage gap; off while the generated hazard cases are recorded.
    #[arg(long)]
    pub require_coverage: bool,
}

/// Arguments of `sqlquack audit`.
#[derive(Debug, Args)]
pub struct AuditArguments {
    /// Where evidence lives.
    #[command(flatten)]
    pub paths: EvidencePaths,
    /// The share of recorded cases to re-record, such as `10%`.
    #[arg(
        long,
        conflicts_with_all = ["compare", "errors"],
        required_unless_present_any = ["compare", "errors"]
    )]
    pub sample: Option<String>,
    /// Re-record every case whose stored outcome is a warehouse error, to keep its message.
    #[arg(long, conflicts_with = "compare")]
    pub errors: bool,
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

/// Arguments of `sqlquack index`.
#[derive(Debug, Args)]
pub struct IndexArguments {
    /// Case directories (holding `.toml` case files) and SQL files or directories; default `cases`.
    pub paths: Vec<PathBuf>,
    /// The policy file, which case loading checks cases against.
    #[arg(long, default_value = POLICY_FILE_NAME)]
    pub policy: PathBuf,
    /// Refuse every known divergence the policy refuses in strict mode.
    #[arg(long)]
    pub strict: bool,
    /// Print the index as JSON on standard output.
    #[arg(long)]
    pub json: bool,
    /// A JSON file mapping relation names to `[column, type]` pairs or column objects, for SQL files.
    #[arg(long)]
    pub schema: Option<PathBuf>,
    /// Treat each converted SQL file as the relation its file name references, and convert again.
    #[arg(long)]
    pub chain_models: bool,
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
    /// A JSON file mapping relation names to `[column, Snowflake type]` pairs or column objects.
    #[arg(long)]
    pub schema: Option<PathBuf>,
    /// Treat each converted file as the relation its file name references, and convert again.
    #[arg(long)]
    pub chain_models: bool,
    /// The policy file whose known divergences conversion may rely on; default `sqlquack.toml`.
    #[arg(long)]
    pub policy: Option<PathBuf>,
    /// Refuse every known divergence the policy refuses in strict mode.
    #[arg(long)]
    pub strict: bool,
    /// Print file, model and relation names in the slowest files and the blockers.
    #[arg(long)]
    pub names: bool,
}
