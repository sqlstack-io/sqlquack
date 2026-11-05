//! Paths and names of the result-matching measure's files.

/// The statements file format `sqlquack verify` writes.
pub const STATEMENTS_FORMAT: &str = "sqlquack-translation-statements/1";
/// The baseline's format name.
pub const BASELINE_FORMAT: &str = "sqlquack-translation-results/1";
/// How statement constructs other than function calls link to forms, in the translation directory.
pub const LINKS_FILE: &str = "form-links.toml";
/// Forms DuckDB has no meaningful equivalent for, with their reasons.
pub const NOT_TRANSLATABLE_FILE: &str = "not-translatable.toml";
/// The auditor's sign-offs of the links and the list.
pub const RESULTS_REVIEW_FILE: &str = "reviews/auditor-results.toml";
/// The committed outcomes the gate compares with.
pub const BASELINE_FILE: &str = "results-baseline.json";
/// The sign-off subject of `form-links.toml`.
pub const FORM_LINKS_SUBJECT: &str = "form-links";
/// The sign-off subject of the function-name index.
pub const FUNCTION_LINKS_SUBJECT: &str = "function-links";
/// The sign-off subject prefix of a not-translatable group.
pub const NOT_TRANSLATABLE_SUBJECT: &str = "not-translatable:";
/// The construct key prefix of a function call.
pub const FUNCTION_PREFIX: &str = "fn:";
/// The catalogue family of functions, whose call forms link by name.
pub const FUNCTIONS_FAMILY: &str = "functions";
/// Statement references listed per worklist entry.
pub const WORK_EXAMPLES: usize = 3;
/// Worklist entries printed as text; the JSON report holds them all.
pub const WORK_PRINTED: usize = 25;
/// Problems of one kind printed before the rest are counted.
pub const PROBLEMS_PRINTED: usize = 40;
/// Families whose forms a query-result receipt can measure, in the capture queue's order.
pub const QUERY_FAMILIES: [&str; 3] = ["functions", "expressions", "query"];
/// The construct of a window function call, which a function's OVER form needs.
pub const WINDOW_FUNCTION: &str = "node:window_function";
/// The construct of a named argument, which a function's `=>` form needs.
pub const NAMED_ARGUMENT: &str = "node:named_argument";
/// The refused construct of a refusal that names no construct: the whole statement.
pub const STATEMENT_REFUSAL: &str = "statement";
