//! Paths and names of the result-matching measure's files.

/// The statements file format `sqlquack verify` writes.
pub const STATEMENTS_FORMAT: &str = "sqlquack-translation-statements/1";
/// The baseline's format name.
pub const BASELINE_FORMAT: &str = "sqlquack-translation-results/1";
/// How statement constructs other than function calls link to forms, in the translation directory.
pub const LINKS_FILE: &str = "form-links.toml";
/// Forms DuckDB has no meaningful equivalent for, with their reasons.
pub const NOT_TRANSLATABLE_FILE: &str = "not-translatable.toml";
/// The reviewers' sign-offs of the links and the list.
pub const RESULTS_REVIEW_FILE: &str = "reviews/results.toml";
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
/// The key prefix of a plain function call, beside its `fn:` construct.
pub const CALL: &str = "call:";
/// The key prefix of a function call over a window.
pub const CALL_OVER: &str = "call_over:";
/// The key prefix of a function call with named arguments.
pub const CALL_NAMED: &str = "call_named:";
/// The key prefix of a function called without parentheses.
pub const CALL_BARE: &str = "call_bare:";
/// Every call-shape prefix; these keys qualify a function's construct and link nothing alone.
pub const CALL_PREFIXES: [&str; 4] = [CALL, CALL_OVER, CALL_NAMED, CALL_BARE];
/// The refused construct of a refusal that names no construct: the whole statement.
pub const STATEMENT_REFUSAL: &str = "statement";
/// The directory of per-family obligation files, in the translation directory.
pub const OBLIGATIONS_DIR: &str = "obligations";
/// The prefix of every case an obligation cites, before the statement's reference.
pub const OBLIGATION_CASES_PREFIX: &str = "cases/";
/// The directory under the evidence cases holding translation cases.
pub const TRANSLATION_CASES: &str = "translation/";
/// Path parts a cited case may not have, besides an empty one: they could leave the cases root.
pub const RELATIVE_PATH_PARTS: [&str; 2] = [".", ".."];
/// The boundary kinds a boundary obligation may name.
pub const BOUNDARY_SUBKINDS: [&str; 7] = [
    "overflow",
    "magnitude",
    "rounding_scale",
    "empty_string",
    "negative",
    "timezone_dst",
    "leap_day",
];
/// Withdrawn obligation ids, in the translation directory.
pub const RETIRED_OBLIGATIONS_FILE: &str = "retired-obligations.toml";
/// The extension of each `<family>` obligation file.
pub const OBLIGATION_FILE_EXTENSION: &str = "toml";
