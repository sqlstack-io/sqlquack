//! The layout check: the laid-out SQL returns exactly what the single-line SQL returns.

use sqlquack_compare::values::models::ResultSet;

use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::{PythonReplay, ReplaySettings};

/// The divergence whose values read the clock, so two replays may differ by the time between them.
const CLOCK_DIVERGENCE: &str = "current_time";

/// Why the laid-out SQL returns another result or error than the single-line `sql`, if it does.
pub(crate) fn layout_failure(
    laid_out: &Result<String, String>,
    (setup, sql): (&[String], &str),
    (single_line, reads_clock): (&Result<ResultSet, String>, bool),
    (python, settings): (Option<&PythonReplay>, &ReplaySettings),
) -> Option<String> {
    let laid_out = match laid_out {
        Ok(laid_out) if laid_out == sql => return None,
        Ok(laid_out) => laid_out,
        Err(reason) => return Some(format!("the layout changed more than whitespace: {reason}")),
    };
    let replayed = replay_on_duckdb((setup, laid_out), python, settings);
    let same = match (single_line, &replayed) {
        (Ok(single_line), Ok(replayed)) => same_result(single_line, replayed, reads_clock),
        (Err(single_line), Err(replayed)) => first_line(single_line) == first_line(replayed),
        _ => false,
    };
    (!same).then(|| {
        format!(
            "the laid-out SQL returned another result than the single-line SQL: {}",
            match &replayed {
                Ok(_) => "different rows or columns".to_owned(),
                Err(error) => first_line(error).to_owned(),
            }
        )
    })
}

/// Whether the provenance's divergences include one whose values read the clock.
pub(crate) fn reads_clock<'name>(mut divergences: impl Iterator<Item = &'name String>) -> bool {
    divergences.any(|divergence| divergence == CLOCK_DIVERGENCE)
}

/// Whether two results have the same schema and, unless they read the clock, the same rows.
fn same_result(left: &ResultSet, right: &ResultSet, reads_clock: bool) -> bool {
    left.schema == right.schema && (reads_clock || left.batches == right.batches)
}

/// An error's first line; later lines quote the SQL with its line numbers.
fn first_line(error: &str) -> &str {
    error.lines().next().unwrap_or_default()
}
