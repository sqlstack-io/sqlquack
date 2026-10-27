use sqlquack_core::failures::main::raise_id::raise_id;

use crate::raises::main::identify_raises::identify_raises;
use crate::raises::main::raised::raised;

/// `a / operand`, raising Snowflake's division by zero error when the operand is zero.
pub(crate) fn guarded(operand: &str) -> String {
    format!(
        "(CASE WHEN {operand} = 0 THEN {} ELSE a / {operand} END)",
        raised("sqlquack:100051: Division by zero")
    )
}

/// Whether identifying `left` and `right` gives their last raises the same ID.
pub(crate) fn same_ids(left: &str, right: &str) -> bool {
    identify_raises(left).1.last() == identify_raises(right).1.last()
}

/// Which of the IDs `sql` was given DuckDB's error text for its first raise reads back.
pub(crate) fn read_back(sql: &str) -> Option<usize> {
    let (identified, ids) = identify_raises(sql);
    let literal = identified.split('\'').nth(1).unwrap_or_default();
    let error = format!("Conversion Error: Could not convert string '{literal}' to INT32");
    let id = raise_id(&error)?;
    ids.iter().position(|given| given == id)
}
