//! Each row of a case that diverged as expected, replayed alone so a raise cannot hide a wrong row.

use arrow::compute::concat_batches;
use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_compare::comparison::models::RecordedResult;
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::models::Policy;

use crate::cases::models::LoadedCase;
use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::ReplaySettings;
use crate::verify::_helpers::judge::divergence_error;
use crate::verify::_helpers::microsecond_evidence::differs_below_microseconds;
use crate::verify::models::Conversion;
use crate::verify::types::CaseConverter;

/// What replaying a diverged case's rows alone shows.
pub(crate) enum RowCheck {
    /// Every row matched its recorded row or raised the divergence alone.
    Proven,
    /// The rows could not all be replayed alone; nothing is shown either way.
    Unproven,
    /// A row alone returned a value other than its recorded one.
    Wrong(String),
    /// The recorded batches do not join into one, so no row can be read.
    Unreadable(String),
}

/// Replays each of `case`'s rows alone against its row of `recorded`.
pub(crate) fn row_check(
    case: &LoadedCase,
    recorded: &RecordedResult,
    (converter, policy, settings): (&dyn CaseConverter, &Policy, &ReplaySettings),
    divergence: &str,
) -> RowCheck {
    let rows = match concat_batches(&recorded.rows.schema, &recorded.rows.batches) {
        Ok(rows) => rows,
        Err(error) => return RowCheck::Unreadable(error.to_string()),
    };
    if case.row_sources.is_empty() || rows.num_rows() != case.row_sources.len() {
        return RowCheck::Unproven;
    }
    let mut proven = true;
    for (index, source_sql) in case.row_sources.iter().enumerate() {
        let alone = LoadedCase {
            source_sql: source_sql.clone(),
            ..case.clone()
        };
        let Conversion::Converted { sql, setup, .. } = converter.convert(&alone) else {
            proven = false;
            continue;
        };
        match replay_on_duckdb((&setup, &sql), settings) {
            Err(error) => proven &= divergence_error(&error, divergence, policy).is_some(),
            Ok(replayed) => {
                let row = RecordedResult {
                    columns: recorded.columns.clone(),
                    rows: ResultSet {
                        schema: rows.schema(),
                        batches: vec![rows.slice(index, 1)],
                    },
                };
                if let Err(mismatch) =
                    compare_with_replay(&row, &replayed, case.order, &policy.comparator)
                {
                    match differs_below_microseconds(
                        &row,
                        &replayed,
                        case.order,
                        &policy.comparator,
                    ) {
                        Ok(true) => {
                            proven = false;
                            continue;
                        }
                        Ok(false) => {}
                        Err(error) => return RowCheck::Unreadable(error.to_string()),
                    }
                    return RowCheck::Wrong(format!(
                        "row {index} alone, hidden by the case's raise: {mismatch}"
                    ));
                }
            }
        }
    }
    match proven {
        true => RowCheck::Proven,
        false => RowCheck::Unproven,
    }
}
