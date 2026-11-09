//! `within_budget`: a conversion, or a refusal when converting it outran its time or size budget.

use std::time::Duration;

use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};

use crate::snowflake::conversion::models::{ConversionOptions, Converted};

/// `converted`, or a refusal when converting it outran the time or size budget.
pub fn within_budget(
    converted: Converted,
    (source, elapsed): (&str, Duration),
    options: &ConversionOptions,
) -> Result<Converted, Refusal> {
    let budget = options.budget;
    let emitted = converted.sql.len() as u64;
    let allowed = budget
        .minimum_bytes
        .max((source.len() as u64).saturating_mul(budget.size_ratio));
    let reason = if elapsed > Duration::from_millis(budget.milliseconds) {
        format!(
            "converting the statement took {} ms, beyond the budget of {} ms",
            elapsed.as_millis(),
            budget.milliseconds
        )
    } else if emitted > allowed {
        format!(
            "the DuckDB SQL is {emitted} bytes for {} bytes of source, beyond the budget of \
             {allowed} bytes ({} times the source, at least {})",
            source.len(),
            budget.size_ratio,
            budget.minimum_bytes
        )
    } else {
        return Ok(converted);
    };
    Err(Refusal {
        construct: None,
        location: None,
        category: RefusalCategory::ConversionBudget,
        reason,
    })
}
