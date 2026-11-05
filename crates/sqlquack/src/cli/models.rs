//! Command-line arguments.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use sqlquack_core::diagnostics::models::Diagnostic;

use sqlquack_evidence::constants::{
    AUDIT_PENDING_DIR, AUDIT_RESULTS_DIR, CASES_DIR, PENDING_DIR, RESULTS_DIR,
};
use sqlquack_evidence::coverage::constants::{DOMAINS_DIR, HAZARDS_FILE};
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::models::DivergenceFailure;

use crate::analysis::models::PythonSignature;
use crate::census::constants::DEFAULT_TEXT_LIMIT;
use crate::census::models::CursorType;
use crate::cli::constants::EXIT_STATUS_HELP;

/// Snowflake-to-DuckDB SQL conversion that refuses to guess.
#[derive(Debug, Parser)]
#[command(name = "sqlquack", version, after_help = EXIT_STATUS_HELP)]
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
    /// Convert Snowflake SQL from a file or standard input into DuckDB SQL, or refuse it.
    Convert(ConvertArguments),
    /// Check that Snowflake SQL parses and holds nothing Snowflake rejects, without converting it.
    Check(CheckArguments),
    /// Replay converted SQL over a DuckDB inputs database and compare with recorded results.
    Compare(CompareArguments),
}

/// Arguments of `sqlquack compare`.
#[derive(Debug, Args)]
pub struct CompareArguments {
    /// A JSON list of comparisons; prints each one's outcome as JSON.
    #[arg(long)]
    pub manifest: PathBuf,
    /// The policy file whose comparator and session profile the comparisons use.
    #[arg(long, default_value = "sqlquack.toml")]
    pub policy: PathBuf,
    /// The session profile whose DuckDB time zone each replay runs in.
    #[arg(long, default_value = "utc-v1")]
    pub session_profile: String,
}

/// Arguments of `sqlquack check`.
#[derive(Debug, Args)]
pub struct CheckArguments {
    /// The SQL file; standard input when absent or `-`.
    pub path: Option<PathBuf>,
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
}

/// Arguments of `sqlquack convert`.
#[derive(Debug, Args)]
pub struct ConvertArguments {
    /// The SQL file; standard input when absent or `-`.
    pub path: Option<PathBuf>,
    /// A JSON file mapping relation names to `[column, Snowflake type]` pairs or column objects.
    #[arg(long)]
    pub schema: Option<PathBuf>,
    /// User overrides: a FUNCTION() file, or a directory of them, each with `replaces` and `dialect duckdb`.
    #[arg(long)]
    pub overrides: Option<PathBuf>,
    /// NAME=<sha256>: an override verified for the body with that hash, as `convert --json` reports.
    #[arg(long = "verified-override", value_name = "NAME=SHA256")]
    pub verified_overrides: Vec<String>,
    /// A Snowflake session parameter of the caller's session, `NAME=VALUE`; repeatable.
    #[arg(long = "session", value_name = "NAME=VALUE")]
    pub session: Vec<String>,
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
    /// The policy and what the caller confirms about its sessions.
    #[command(flatten)]
    pub conversion: PolicyArguments,
}

/// Arguments of `sqlquack function`.
#[derive(Debug, Args)]
pub struct FunctionArguments {
    /// The file holding the function's body (a scalar's expression or a table function's query), or `-` for standard input.
    pub body: PathBuf,
    /// The function's name, as the schema declares it.
    #[arg(long)]
    pub name: String,
    /// The schema file declaring the function and the relations its body reads.
    #[arg(long)]
    pub schema: PathBuf,
    /// A Snowflake session parameter of the caller's session, `NAME=VALUE`; repeatable.
    #[arg(long = "session", value_name = "NAME=VALUE")]
    pub session: Vec<String>,
    /// The policy and what the caller confirms about its sessions.
    #[command(flatten)]
    pub conversion: PolicyArguments,
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
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
    /// Fail unless every equivalence and rule is relied on by at least one passed case.
    #[arg(long)]
    pub require_verified: bool,
    /// Fail when DuckDB's error differs in kind from Snowflake's, or no rule used declares it.
    #[arg(long)]
    pub strict_errors: bool,
    /// Re-recorded results, read for the messages of errors stored without one.
    #[arg(long, default_value = AUDIT_RESULTS_DIR)]
    pub audit_results: PathBuf,
    /// A Python interpreter with the duckdb package, replaying cases that call Python functions.
    #[arg(long)]
    pub python: Option<PathBuf>,
    /// Write each case's translation outcome and constructs as JSON, for `syntax-coverage results`.
    #[arg(long)]
    pub translation_statements: Option<PathBuf>,
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
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
}

/// Arguments of `sqlquack index`.
#[derive(Debug, Args)]
pub struct IndexArguments {
    /// Case directories (holding `.toml` case files) and SQL files or directories; default `cases`.
    pub paths: Vec<PathBuf>,
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
    /// A JSON file mapping relation names to `[column, type]` pairs or column objects, for SQL files.
    #[arg(long)]
    pub schema: Option<PathBuf>,
    /// Treat each converted SQL file as the relation its file name references, and convert again.
    #[arg(long)]
    pub chain_models: bool,
    /// The policy and what the caller confirms about its sessions.
    #[command(flatten)]
    pub conversion: PolicyArguments,
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
    /// How to print the report on standard output.
    #[command(flatten)]
    pub output: OutputArguments,
    /// Constructs listed in the text report.
    #[arg(long, default_value_t = DEFAULT_TEXT_LIMIT)]
    pub limit: usize,
    /// A JSON file mapping relation names to `[column, Snowflake type]` pairs or column objects.
    #[arg(long)]
    pub schema: Option<PathBuf>,
    /// Treat each converted file as the relation its file name references, and convert again.
    #[arg(long)]
    pub chain_models: bool,
    /// User overrides: a FUNCTION() file, or a directory of them, each with `replaces` and `dialect duckdb`.
    #[arg(long)]
    pub overrides: Option<PathBuf>,
    /// NAME=<sha256>: an override verified for the body with that hash, as `convert --json` reports.
    #[arg(long = "verified-override", value_name = "NAME=SHA256")]
    pub verified_overrides: Vec<String>,
    /// Print file, model and relation names in the slowest files and the blockers.
    #[arg(long)]
    pub names: bool,
    /// The project's cursor type: cursor intrinsics become a bound as SQLBuild renders one.
    #[arg(long, value_enum)]
    pub cursor_type: Option<CursorType>,
    /// Construct keys or `rule.<id>`s the what-if counts treat as supported, comma-separated.
    #[arg(long, value_delimiter = ',')]
    pub assume_supported: Vec<String>,
    /// The policy and what the caller confirms about its sessions.
    #[command(flatten)]
    pub conversion: PolicyArguments,
}

/// The policy conversion relies on, and what the caller confirms about its sessions.
#[derive(Debug, Args)]
pub struct PolicyArguments {
    /// The policy file whose known divergences conversion may rely on.
    #[arg(long, default_value = POLICY_FILE_NAME)]
    pub policy: PathBuf,
    /// Use the built-in default instead of a policy file: every known divergence is refused.
    #[arg(long, conflicts_with = "policy")]
    pub builtin_policy: bool,
    /// Refuse every known divergence the policy refuses in strict mode.
    #[arg(long)]
    pub strict: bool,
    /// Confirm both sessions run in UTC, so strict mode relies on `session_timezone_utc`.
    #[arg(long)]
    pub utc_session: bool,
    /// Confirm the TIMESTAMP_TZ offsets the schema declares, so strict mode relies on them.
    #[arg(long)]
    pub confirm_declared_offsets: bool,
    /// Confirm a Python runtime registers the project's Python functions with SQLQuack's runtime.
    #[arg(long)]
    pub python_runtime: bool,
    /// Refuse every known divergence that can return a different value rather than raise.
    #[arg(long)]
    pub loud_divergences_only: bool,
}

/// How a report is printed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Text for people.
    #[default]
    Text,
    /// JSON for programs.
    Json,
}

/// The report format: `--format json`, or its short form `--json`.
#[derive(Debug, Args)]
pub struct OutputArguments {
    /// Print the report as text or as JSON.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    /// Print the report as JSON; the same as `--format json`.
    #[arg(long, conflicts_with = "format")]
    pub json: bool,
}

impl OutputArguments {
    /// Whether the report is printed as JSON.
    pub fn json(&self) -> bool {
        self.json || self.format == OutputFormat::Json
    }
}

/// What `convert --json` prints.
#[derive(Clone, Debug, Serialize)]
pub struct ConvertReport {
    /// Whether every statement converted.
    pub converted: bool,
    /// The session profile conversion ran under.
    pub profile: String,
    /// The DuckDB SQL, one statement after another; empty when refused.
    pub sql: String,
    /// Each refusal.
    pub refusals: Vec<ConvertRefusal>,
    /// Known divergences relied on, with the constructs relying on each.
    pub divergences: BTreeMap<String, Vec<String>>,
    /// Of those, the named assumptions.
    pub assumptions: Vec<String>,
    /// Each divergence relied on and its failure: `loud` raises, `value` can return another value.
    pub divergence_failures: BTreeMap<String, DivergenceFailure>,
    /// What the session parameters changed about conversion.
    pub notes: Vec<String>,
    /// The Python functions the SQL calls, which the caller registers before running it.
    pub python_functions: Vec<PythonSignature>,
    /// The macros of the user overrides the SQL calls, which the caller runs before it.
    pub macros: Vec<String>,
    /// Every user override loaded: its name, the function it replaces and its body's hash.
    pub overrides: Vec<OverrideListing>,
}

/// A user override loaded for conversion.
#[derive(Clone, Debug, Serialize)]
pub struct OverrideListing {
    /// The override's name.
    pub name: String,
    /// The function it replaces.
    pub replaces: String,
    /// The hash of its body.
    pub sha256: String,
}

/// One refusal as `convert --json` prints it: the original fields, then its diagnostic's.
#[derive(Clone, Debug, Serialize)]
pub struct ConvertRefusal {
    /// The refusal category, as reports name it.
    pub category: String,
    /// The refused construct, or `statement`.
    pub construct: String,
    /// The one-based line of the refusal, when placed.
    pub line: Option<usize>,
    /// The one-based column of the refusal, when placed.
    pub column: Option<usize>,
    /// Why it is refused.
    pub reason: String,
    /// The refusal's diagnostic, flattened into it.
    #[serde(flatten)]
    pub diagnostic: Option<Diagnostic>,
}

/// What `check --json` prints.
#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    /// Whether the SQL holds no problem.
    pub ok: bool,
    /// Each problem found.
    pub diagnostics: Vec<Diagnostic>,
}

/// One `sqlquack compare` entry: a recording and the DuckDB SQL replayed over an inputs database.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Comparison {
    /// Echoed in the outcome.
    pub(crate) id: String,
    /// The recorded result's sidecar, beside its `.arrow` file.
    pub(crate) recorded: PathBuf,
    /// The DuckDB database the SQL reads, attached read-only as `inputs`.
    pub(crate) database: PathBuf,
    /// The converted SQL file.
    pub(crate) sql: PathBuf,
    /// Whether the rows compare in order.
    pub(crate) ordered: bool,
}
