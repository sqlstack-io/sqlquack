//! Run-time checks that raise where DuckDB's JSON could differ from Snowflake's VARIANT.

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::JSON_FIDELITY;

/// An expression raising the json_fidelity error, naming `what`.
pub(crate) fn fidelity_error(what: &str) -> String {
    raised(&format!("{JSON_FIDELITY}: {what}"))
}

/// `body` over `value` evaluated once, as the lambda parameter `name`.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}
