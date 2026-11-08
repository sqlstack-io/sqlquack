//! What a run takes and what it finds.

use sqlquack_policy::policy::models::Policy;
use sqlquack_semantics::types::models::Schema;

/// A converted query and what it runs over.
#[derive(Clone, Copy, Debug)]
pub struct RunInputs<'a> {
    /// The Snowflake SQL, which must hold one query.
    pub source: &'a str,
    /// The converted DuckDB SQL.
    pub converted: &'a str,
    /// The macros the converted SQL calls, run first.
    pub macros: &'a [String],
    /// The session profile it converted under.
    pub profile: &'a str,
    /// The policy, or None for the built-in one.
    pub policy: Option<&'a Policy>,
    /// The project's tables, each created empty before the fixtures fill some.
    pub schema: &'a Schema,
    /// `TABLE=ROWS` fixtures: a JSON list of rows, or a path holding a `/`.
    pub fixtures: &'a [String],
    /// The rows the result must hold, as a JSON list.
    pub expect: Option<&'a str>,
}

/// The result of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutcome {
    /// The result as a plain table: a header, then one line per row.
    pub table: String,
    /// How many rows the result holds.
    pub rows: usize,
    /// How the rows differ from the expected ones: None without them, empty when they match.
    pub differences: Option<Vec<String>>,
}
