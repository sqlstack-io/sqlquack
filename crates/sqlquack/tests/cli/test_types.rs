pub(crate) struct CommandTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) mention: &'static str,
    pub(crate) expected_outcome: CommandOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) wrote_pending_files: bool,
    pub(crate) stderr_mentions: bool,
}

pub(crate) struct DivergenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) case_marker: &'static str,
    pub(crate) policy_default: &'static str,
    pub(crate) expected_outcome: DivergenceOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DivergenceOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) verdict: String,
}

pub(crate) struct ReviewTestCase {
    pub(crate) description: &'static str,
    pub(crate) file: &'static str,
    pub(crate) expected_lines: &'static [&'static str],
}

/// One `sqlquack review` of a copy of the repository: its exit code and export directory.
pub(crate) struct ReviewedRepository {
    pub(crate) exit_code: Option<i32>,
    pub(crate) directory: tempfile::TempDir,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ReviewOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) missing_lines: Vec<String>,
}

pub(crate) struct ErrorCheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) recorded_error: Option<(i64, &'static str)>,
    pub(crate) audited_message: Option<&'static str>,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: ErrorCheckOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErrorCheckOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) verdict: String,
    pub(crate) error_check: String,
    pub(crate) snowflake_message: Option<String>,
}

#[derive(Debug)]
pub(crate) struct FunctionMacroTestCase {
    pub(crate) description: &'static str,
    pub(crate) returns: &'static str,
    pub(crate) body: &'static str,
    pub(crate) call: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: FunctionMacroOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FunctionMacroOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) values: Vec<String>,
}

pub(crate) struct CoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) key_families: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: CoverageOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CoverageOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) shape_gaps: Vec<String>,
    pub(crate) hazard_gaps: Vec<String>,
}

pub(crate) struct PendingFlagsTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_flags: Vec<(String, Option<bool>, Option<bool>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConvertOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) printed_sql: bool,
    pub(crate) json_converted: Option<bool>,
    pub(crate) stderr_mentions: bool,
}

pub(crate) struct ConvertTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) mention: &'static str,
    pub(crate) expected_outcome: ConvertOutcome,
}

pub(crate) struct ConvertedRunTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct ColumnNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_names: Vec<String>,
}

pub(crate) struct PolicyFreeTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) policy_file: Option<&'static str>,
    pub(crate) expected_exit_code: Option<i32>,
}

pub(crate) struct ValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_value: &'static str,
}

pub(crate) struct RunErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_mention: &'static str,
}

pub(crate) struct PythonRunTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) handlers: &'static [(&'static str, &'static str)],
    pub(crate) expected_refused_without_runtime: Option<bool>,
    pub(crate) expected_functions: serde_json::Value,
    pub(crate) expected_rows: &'static [&'static [&'static str]],
}

pub(crate) struct CountedCallsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) handlers: &'static [(&'static str, &'static str)],
    pub(crate) expected_distinct_values: usize,
}

pub(crate) struct PythonRefusalTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_mention: &'static str,
}

pub(crate) struct CrossingTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) handler: (&'static str, &'static str),
    pub(crate) expected_mention: &'static str,
}

pub(crate) struct OverrideTestCase {
    pub(crate) description: &'static str,
    pub(crate) strict: bool,
    /// `NAME=<sha256>` to pass as verified; empty means the body's own hash.
    pub(crate) verified_sha256: Option<&'static str>,
    pub(crate) expected_value: Option<&'static str>,
    pub(crate) expected_refusal: Option<&'static str>,
}

pub(crate) struct OverrideFailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) body: &'static str,
    pub(crate) expected_mention: &'static str,
}

pub(crate) struct DedicatedOverrideTestCase {
    pub(crate) description: &'static str,
    pub(crate) replaces: &'static str,
    pub(crate) body: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_value: Option<&'static str>,
    pub(crate) expected_failure: Option<&'static str>,
}

pub(crate) struct ConvertDiagnosticTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_text: &'static str,
    pub(crate) expected_json: (&'static str, &'static str, u64, u64),
}

pub(crate) struct PlacedNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_text: &'static str,
    pub(crate) expected_span: (&'static str, u64, u64),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FunctionStdinOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) converted: Option<bool>,
    pub(crate) first_code: Option<String>,
    pub(crate) stderr_lines: Vec<String>,
}

pub(crate) struct FunctionStdinTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) body: &'static str,
    pub(crate) expected_outcome: FunctionStdinOutcome,
}

pub(crate) struct OutputFormatTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_exit_code: Option<i32>,
    pub(crate) expected_stdout_prefix: &'static str,
}

pub(crate) struct ExplainTestCase {
    pub(crate) description: &'static str,
    pub(crate) code: &'static str,
    pub(crate) expected_exit_code: Option<i32>,
    pub(crate) expected_stdout_mentions: &'static [&'static str],
    pub(crate) expected_stderr_mention: &'static str,
}

pub(crate) struct ColorTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) sql: &'static str,
    pub(crate) environment: &'static [(&'static str, &'static str)],
    pub(crate) expected_exit_code: Option<i32>,
    pub(crate) expected_stdout: &'static str,
    pub(crate) expected_stderr: &'static str,
}

pub(crate) struct CheckCommandTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) sql: &'static str,
    pub(crate) expected_exit_code: Option<i32>,
    pub(crate) expected_stdout_prefix: &'static str,
    pub(crate) expected_stderr_mention: &'static str,
}

pub(crate) struct FailureKindTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_converted: bool,
    pub(crate) expected_failures: &'static [(&'static str, &'static str)],
}

pub(crate) struct CompareTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) ordered: bool,
    pub(crate) arrow_sha256: Option<&'static str>,
    pub(crate) expected_outcome: &'static str,
}

/// A recorded case and what `verify --translation-statements` must write for it.
pub(crate) struct TranslationStatementTestCase {
    pub(crate) description: &'static str,
    pub(crate) id: &'static str,
    pub(crate) expected_outcome: &'static str,
    pub(crate) expected_codes: &'static [&'static str],
    pub(crate) expected_refused: &'static [&'static str],
    /// One construct its statement must list as used.
    pub(crate) expected_use: &'static str,
}

/// A flat chain of `terms` copies of an operand joined by `operator`, and how converting it ends.
pub(crate) struct DeepChainTestCase {
    pub(crate) description: &'static str,
    pub(crate) operator: &'static str,
    pub(crate) operand: &'static str,
    pub(crate) terms: usize,
    pub(crate) expected_exit: Option<i32>,
    /// Text standard error must hold, such as the refusal's code.
    pub(crate) expected_mention: &'static str,
}

/// A statement refused for a divergence under some flags, and the help it must give.
pub(crate) struct DivergenceHelpTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_help: &'static str,
}

/// A malformed schema file and the words its error must use.
pub(crate) struct SchemaErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) schema: &'static str,
    pub(crate) expected_error: &'static str,
}

/// Convert flags and the SQL convert prints under them, from standard output or `--json`'s `sql`.
pub(crate) struct LayoutFlagsTestCase {
    pub(crate) description: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_sql: &'static str,
}

/// A file of a project, by its path under the project folder.
#[derive(Clone, Copy)]
pub(crate) struct ProjectFile {
    pub(crate) path: &'static str,
    pub(crate) text: &'static str,
}

/// A project's files and `[project]` settings, the convert flags, and what the report must hold.
pub(crate) struct ProjectTestCase {
    pub(crate) description: &'static str,
    pub(crate) files: &'static [ProjectFile],
    /// TOML appended to the repository's policy file.
    pub(crate) project: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) sql: &'static str,
    pub(crate) expected_exit: Option<i32>,
    pub(crate) expected_output: &'static str,
}

/// How `sqlquack convert --json` ended in a project folder.
pub(crate) struct ProjectOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}
