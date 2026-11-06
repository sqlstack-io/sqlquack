//! Evidence layout and formats.

/// Authored case files, relative to the repository root.
pub const CASES_DIR: &str = "cases";

/// Recorded results: append-only, one `<hash>.json` and usually one `<hash>.arrow` per case.
pub const RESULTS_DIR: &str = "evidence/results";
/// The account's session defaults, beside the results directory.
pub const SESSION_DEFAULTS_NAME: &str = "session_defaults.json";
/// The session defaults file in the default layout.
pub const SESSION_DEFAULTS_FILE: &str = "evidence/session_defaults.json";

/// Pending case files for the recorder; derived, never committed.
pub const PENDING_DIR: &str = "evidence/pending";

/// Pending audit cases; derived, never committed.
pub const AUDIT_PENDING_DIR: &str = "evidence/audit/pending";

/// Fresh audit recordings; never committed.
pub const AUDIT_RESULTS_DIR: &str = "evidence/audit/results";

/// The version of the case, pending and result file formats.
pub const EVIDENCE_FORMAT: u32 = 1;

/// The source dialect name recorded in hashes and pending files.
pub const SOURCE_DIALECT: &str = "snowflake";

/// The input column holding the row number; expression results are ordered by it.
pub const ORDINAL_COLUMN: &str = "ordinal";

/// The single result column of expression cases.
pub const VALUE_COLUMN: &str = "value";

/// Appended to an equivalence case's reference for its canonical spelling.
pub const CANONICAL_SUFFIX: &str = "~canonical";

/// The relation that holds the case input rows.
pub const INPUT_RELATION: &str = "input";

/// The extension of authored case files.
pub const CASE_FILE_EXTENSION: &str = "toml";

/// The extension of metadata and pending files.
pub const METADATA_EXTENSION: &str = "json";

/// The length of a case hash in hexadecimal digits.
pub const CASE_HASH_LENGTH: usize = 64;

/// The case tag declaring that Snowflake is expected to raise an error.
pub const ERROR_TAG: &str = "error";

/// Quotes a project function's body in the Snowflake statement creating it.
pub const FUNCTION_BODY_QUOTE: &str = "$$";
/// The Snowflake Python runtime versions a case function may declare.
pub const PYTHON_RUNTIME_VERSIONS: &[&str] = &["3.11", "3.12"];
/// Upper-case query text that makes one result row depend on other input rows.
pub const ROW_COUPLING_WORDS: &[&str] = &[
    " OVER",
    "OVER(",
    "GROUP BY",
    "WITHIN GROUP",
    "DISTINCT",
    "LIMIT",
    "QUALIFY",
    "HAVING",
    "JOIN",
    "UNION",
    "INTERSECT",
    "EXCEPT ",
    "MINUS",
    "(SELECT",
    "( SELECT",
    "FLATTEN",
    "SPLIT_TO_TABLE",
    "AGG(",
    "COUNT(",
    "COUNT_IF(",
    "SUM(",
    "AVG(",
    "MIN(",
    "MAX(",
    "MIN_BY(",
    "MAX_BY(",
    "ANY_VALUE(",
    "MEDIAN(",
    "MODE(",
    "PERCENTILE",
    "STDDEV",
    "VARIANCE",
    "VAR_POP(",
    "VAR_SAMP(",
    "CORR(",
    "COVAR",
    "APPROX",
    "HLL",
    "KURTOSIS(",
    "SKEW(",
];

/// SQLSTATE classes of a Snowflake compile error, which the recorder's pre-check stops on.
pub const COMPILE_ERROR_SQLSTATE_CLASSES: &[&str] = &["0A", "42"];

/// The sidecar outcome of a recording that holds rows.
pub(crate) const ROWS_OUTCOME: &str = "rows";
