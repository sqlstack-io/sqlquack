//! Helpers of the bigquery format tests.

use serde_json::Value;

/// BigQuery's recorded dry-run verdicts, two runs of each statement.
const BIGQUERY_RECEIPTS: &str = include_str!("fixtures/bigquery_receipts.jsonl");
/// The reason BigQuery gives an invalid query.
const INVALID_QUERY: &str = "invalidQuery";
/// The syntax error a BigQuery receipt records, as its message and `line:column`; `None` when it was accepted.
fn recorded_bigquery_rejection(receipt: &Value) -> Option<(String, String)> {
    (receipt["engine_code"] == INVALID_QUERY).then(|| {
        (
            receipt["message"].as_str().unwrap_or_default().to_string(),
            receipt["position"].as_str().unwrap_or_default().to_string(),
        )
    })
}

/// What every recorded BigQuery run of `statement` reported: its syntax error, or `None`.
pub(crate) fn recorded_bigquery_rejections(statement: &str) -> Vec<Option<(String, String)>> {
    BIGQUERY_RECEIPTS
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|receipt| receipt["statement"] == statement)
        .map(|receipt| recorded_bigquery_rejection(&receipt))
        .collect()
}

/// Whether SQLQuack's BigQuery parser refuses `statement` with a syntax error.
pub(crate) fn native_bigquery_refusal(statement: &str) -> bool {
    sqlquack_parse::bigquery::parse_strict(statement)
        .err()
        .and_then(|failure| failure.parse_error().cloned())
        .is_some_and(|error| error.syntax)
}
