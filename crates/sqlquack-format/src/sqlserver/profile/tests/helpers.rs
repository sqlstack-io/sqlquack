//! Helpers of the sqlserver format tests.

use serde_json::Value;

use crate::format::models::FormatDialect;
use crate::format::tests::helpers::refusal;

/// SQL Server's recorded verdicts on the corpus statements SQLQuack refuses.
const SQLSERVER_RECEIPTS: &str = include_str!("fixtures/sqlserver_receipts.jsonl");
/// What every recorded SQL Server run of `statement` reported: its first error number and message, or `None`.
pub(crate) fn recorded_sqlserver_verdicts(statement: &str) -> Vec<Option<(u64, String)>> {
    SQLSERVER_RECEIPTS
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|receipt| receipt["statement"] == statement)
        .map(|receipt| {
            receipt["engine_code"].as_u64().map(|code| {
                (
                    code,
                    receipt["message"].as_str().unwrap_or_default().to_string(),
                )
            })
        })
        .collect()
}

/// The start of the reason the formatter refuses `statement` with as SQL Server; `None` when it is laid out.
pub(crate) fn sqlserver_refusal(statement: &str) -> Option<String> {
    refusal(statement, FormatDialect::SqlServer)
}
