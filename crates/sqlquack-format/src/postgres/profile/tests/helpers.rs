//! Helpers of the postgres format tests.

use serde_json::Value;

/// PostgreSQL 18.6's recorded verdicts, two runs of each statement.
const POSTGRES_RECEIPTS: &str = include_str!("fixtures/pg18.6_receipts.jsonl");
/// The SQLSTATE PostgreSQL reports for a syntax error.
const SYNTAX_ERROR: &str = "42601";

/// The syntax error a PostgreSQL receipt records, as its message and one-based position; `None` when it parsed.
fn recorded_postgres_rejection(receipt: &Value) -> Option<(String, usize)> {
    (receipt["engine_code"] == SYNTAX_ERROR).then(|| {
        (
            receipt["message"].as_str().unwrap_or_default().to_string(),
            receipt["position"]
                .as_str()
                .and_then(|position| position.parse::<usize>().ok())
                .unwrap_or_default(),
        )
    })
}

/// What every recorded PostgreSQL 18.6 run of `statement` reported: its syntax error, or `None`.
pub(crate) fn recorded_postgres_rejections(statement: &str) -> Vec<Option<(String, usize)>> {
    POSTGRES_RECEIPTS
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|receipt| receipt["statement"] == statement)
        .map(|receipt| recorded_postgres_rejection(&receipt))
        .collect()
}

/// The syntax error SQLQuack's PostgreSQL parser reports for `statement`, with a one-based position.
pub(crate) fn native_postgres_rejection(statement: &str) -> Option<(String, usize)> {
    sqlquack_parse::postgres::parse_strict(statement)
        .err()
        .and_then(|failure| failure.parse_error().cloned())
        .map(|error| (error.message, error.position + 1))
}
