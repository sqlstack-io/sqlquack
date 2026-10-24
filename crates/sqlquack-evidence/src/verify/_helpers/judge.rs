//! The verdict for one case.

use std::path::Path;

use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_policy::policy::models::Policy;

use crate::cases::models::LoadedCase;
use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::ReplaySettings;
use crate::results::main::load_result::load_result;
use crate::results::models::StoredResult;
use crate::verify::models::{Conversion, Verdict};
use crate::verify::types::CaseConverter;

pub(crate) fn judge(
    case: &LoadedCase,
    results: &Path,
    policy: &Policy,
    converter: &dyn CaseConverter,
) -> Verdict {
    let stored = match load_result(results, &case.hash) {
        Ok(Some(stored)) => stored,
        Ok(None) => return Verdict::Pending,
        Err(error) => {
            return Verdict::Failed {
                reason: error.to_string(),
            };
        }
    };
    let duckdb_sql = match converter.convert(case) {
        Conversion::Converted(sql) => sql,
        Conversion::Refused(reasons) => return Verdict::Refused { reasons },
    };
    let settings = ReplaySettings {
        time_zone: policy
            .session_profiles
            .get(&case.session_profile)
            .map_or_else(String::new, |profile| profile.duckdb_time_zone.clone()),
        null_order: policy.comparator.null_order.duckdb_setting().to_owned(),
    };
    replayed_verdict(
        case,
        &stored,
        replay_on_duckdb(&duckdb_sql, &settings),
        policy,
    )
}

fn replayed_verdict(
    case: &LoadedCase,
    stored: &StoredResult,
    replayed: Result<sqlquack_compare::values::models::ResultSet, String>,
    policy: &Policy,
) -> Verdict {
    match (&stored.rows, replayed) {
        (None, Err(_)) => Verdict::Passed,
        (None, Ok(_)) => Verdict::Failed {
            reason: format!(
                "Snowflake raised an error ({}), but DuckDB returned rows",
                stored
                    .metadata
                    .error
                    .as_ref()
                    .map_or("unknown", |error| error.message.as_str())
            ),
        },
        (Some(_), Err(error)) => Verdict::Failed {
            reason: format!("Snowflake returned rows, but DuckDB failed: {error}"),
        },
        (Some(recorded), Ok(replayed)) => {
            match compare_with_replay(recorded, &replayed, case.order, &policy.comparator) {
                Ok(()) => Verdict::Passed,
                Err(mismatch) => Verdict::Failed {
                    reason: mismatch.to_string(),
                },
            }
        }
    }
}
