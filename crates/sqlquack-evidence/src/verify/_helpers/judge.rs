//! The verdict for one case.

use std::collections::BTreeMap;
use std::path::Path;

use sqlquack_compare::comparison::main::compare_recordings::compare_recordings;
use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_core::failures::constants::RAISE_ID_MARKER;
use sqlquack_core::failures::main::raise_id::raise_id;
use sqlquack_core::parsing::models::SourceLocation;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_policy::policy::models::Policy;

use crate::cases::models::{CaseRole, LoadedCase};
use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::ReplaySettings;
use crate::results::main::load_result::load_result;
use crate::results::models::{RecordedOutcome, SessionDefaults, StoredResult};
use crate::verify::_helpers::compile_errors::rejected_at_compile_time;
use crate::verify::_helpers::compile_errors::undeclared_compile_error;
use crate::verify::_helpers::error_check::error_check;
use crate::verify::_helpers::layout_check::{layout_failure, reads_clock};
use crate::verify::_helpers::microsecond_evidence::differs_below_microseconds;
use crate::verify::_helpers::row_checks::{RowCheck, row_check};
use crate::verify::_helpers::session::{SessionCheck, session_check};
use crate::verify::models::{Conversion, ErrorCheck, Verdict};
use crate::verify::types::{CaseConverter, FailureModes};

/// A case's verdict, what its conversion relied on, and how the two errors compare.
type Judged = (Verdict, Option<Provenance>, Option<ErrorCheck>);

/// A case's judgement and whether its result's session is unproven.
pub(crate) type JudgedCase = (Judged, bool);

/// A DuckDB replay: its rows, or DuckDB's error.
type Replayed = Result<sqlquack_compare::values::models::ResultSet, String>;

pub(crate) fn judge(
    case: &LoadedCase,
    (results, defaults): (&Path, Option<&SessionDefaults>),
    policy: &Policy,
    checks: (&dyn CaseConverter, &FailureModes),
) -> JudgedCase {
    let stored = match load_result(results, &case.hash) {
        Ok(Some(stored)) => stored,
        Ok(None) => return ((Verdict::Pending, None, None), false),
        Err(error) => {
            let failed = Verdict::Failed {
                reason: error.to_string(),
            };
            return ((failed, None, None), false);
        }
    };
    match session_check(case, &stored.metadata, defaults) {
        SessionCheck::Mismatch(reason) => ((Verdict::Failed { reason }, None, None), false),
        SessionCheck::Matches => (judge_stored(case, (stored, results), policy, checks), false),
        SessionCheck::Unproven => (judge_stored(case, (stored, results), policy, checks), true),
    }
}

fn judge_stored(
    case: &LoadedCase,
    (stored, results): (StoredResult, &Path),
    policy: &Policy,
    (converter, failure_modes): (&dyn CaseConverter, &FailureModes),
) -> Judged {
    if case.role == CaseRole::Typing {
        return match stored.metadata.outcome {
            RecordedOutcome::Error => (
                Verdict::Failed {
                    reason: "a typing case recorded a warehouse error, so it holds no types"
                        .to_owned(),
                },
                None,
                None,
            ),
            RecordedOutcome::Rows | RecordedOutcome::Types => (Verdict::Typed, None, None),
        };
    }
    if let Some(reason) = undeclared_compile_error(case, stored.metadata.error.as_ref()) {
        return (Verdict::Failed { reason }, None, None);
    }
    if stored.metadata.outcome == RecordedOutcome::Types {
        return (
            Verdict::Failed {
                reason: "the recording keeps no values, which only a typing case may".to_owned(),
            },
            None,
            None,
        );
    }
    if let CaseRole::EquivalenceSource { canonical_hash, .. } = &case.role {
        return (
            judge_pair(case, &stored, results, canonical_hash),
            None,
            None,
        );
    }
    let (duckdb_sql, setup, provenance, raises, python) = match converter.convert(case) {
        Conversion::Converted {
            sql,
            setup,
            provenance,
            raises,
            python,
        } => (sql, setup, provenance, raises, python),
        Conversion::Refused {
            reasons,
            constructs,
        } => {
            return (
                Verdict::Refused {
                    reasons,
                    constructs,
                    snowflake_rejected: rejected_at_compile_time(
                        case,
                        stored.metadata.error.as_ref(),
                    ),
                },
                None,
                None,
            );
        }
    };
    let settings = ReplaySettings {
        time_zone: policy
            .session_profiles
            .get(&case.session_profile)
            .map_or_else(String::new, |profile| profile.duckdb_time_zone.clone()),
        null_order: policy.comparator.null_order.duckdb_setting().to_owned(),
    };
    let replayed = replay_on_duckdb((&setup, &duckdb_sql), python.as_deref(), &settings);
    let layout = layout_failure(
        &converter.laid_out(&duckdb_sql),
        (&setup, &duckdb_sql),
        (&replayed, reads_clock(provenance.divergences.keys())),
        (python.as_deref(), &settings),
    );
    let duckdb_error = replayed.as_ref().err().cloned();
    let verdict = match &case.expected_divergence {
        Some(divergence) => divergence_verdict(
            case,
            (&stored, replayed),
            policy,
            (divergence, &provenance, &raises),
        ),
        None => replayed_verdict(case, (&stored, replayed), policy, &raises),
    };
    let verdict = match (verdict, &stored.rows) {
        (
            Verdict::Diverged {
                divergence,
                error,
                matched,
                ..
            },
            Some(recorded),
        ) => match row_check(case, recorded, (converter, policy, &settings), &divergence) {
            RowCheck::Wrong(reason) => Verdict::Failed { reason },
            RowCheck::Unreadable(error) => Verdict::Failed {
                reason: format!("the recorded rows could not be read one by one: {error}"),
            },
            checked => Verdict::Diverged {
                divergence,
                error,
                matched,
                rows_proven: matches!(checked, RowCheck::Proven),
            },
        },
        (verdict, _) => verdict,
    };
    let verdict = match layout {
        Some(reason) => Verdict::Failed { reason },
        None => verdict,
    };
    let error_check = match (&verdict, &stored.metadata.error, duckdb_error) {
        (Verdict::Passed, Some(recorded), Some(error)) => Some(error_check(
            (recorded, &error),
            &provenance,
            (policy, failure_modes),
        )),
        _ => None,
    };
    (verdict, Some(provenance), error_check)
}

/// Passes only a declared divergence where Snowflake returned rows and DuckDB raised.
fn divergence_verdict(
    case: &LoadedCase,
    (stored, replayed): (&StoredResult, Replayed),
    policy: &Policy,
    (divergence, provenance, raises): (&str, &Provenance, &BTreeMap<String, Vec<SourceLocation>>),
) -> Verdict {
    if !provenance.divergences.contains_key(divergence) {
        return Verdict::Failed {
            reason: format!(
                "the case expects divergence `{divergence}`, but the conversion does not rely on it"
            ),
        };
    }
    match (&stored.rows, replayed) {
        (Some(_), Err(error)) => match divergence_error(&error, divergence, policy) {
            Some(matched) => Verdict::Diverged {
                divergence: divergence.to_owned(),
                error,
                matched,
                rows_proven: false,
            },
            None => Verdict::Failed {
                reason: format!(
                    "the case expects divergence `{divergence}`, but DuckDB raised another error: {}",
                    error.lines().next().unwrap_or_default()
                ),
            },
        },
        (None, _) => Verdict::Failed {
            reason: format!(
                "the case expects divergence `{divergence}`, but Snowflake raised an error"
            ),
        },
        (Some(_), replayed) => match replayed_verdict(case, (stored, replayed), policy, raises) {
            Verdict::Passed => Verdict::Failed {
                reason: format!(
                    "the case expects divergence `{divergence}`, but DuckDB matched Snowflake"
                ),
            },
            other => other,
        },
    }
}

/// How `error` is `divergence`'s failure: its guard's tag, or a declared native error pattern.
pub(crate) fn divergence_error(error: &str, divergence: &str, policy: &Policy) -> Option<String> {
    if error.contains(&format!("'{divergence}: "))
        || error.contains(&format!("'{divergence}{RAISE_ID_MARKER}"))
    {
        return Some(format!("the {divergence} guard's raise"));
    }
    let first_line = error.lines().next().unwrap_or_default();
    policy
        .divergences
        .get(divergence)?
        .native_errors
        .iter()
        .find(|pattern| matches_pattern(first_line, pattern))
        .map(|pattern| format!("DuckDB error matching `{pattern}`"))
}

/// Whether `text` matches `pattern` from its start, `*` standing for any run of characters.
fn matches_pattern(text: &str, pattern: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(first) = parts.next() else {
        return true;
    };
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    for part in parts {
        match rest.find(part) {
            Some(position) => rest = &rest[position + part.len()..],
            None => return false,
        }
    }
    pattern.ends_with('*') || rest.is_empty()
}

fn judge_pair(
    case: &LoadedCase,
    stored: &StoredResult,
    results: &Path,
    canonical_hash: &str,
) -> Verdict {
    let canonical = match load_result(results, canonical_hash) {
        Ok(Some(canonical)) => canonical,
        Ok(None) => return Verdict::Pending,
        Err(error) => {
            return Verdict::Failed {
                reason: error.to_string(),
            };
        }
    };
    match (&stored.rows, &canonical.rows) {
        (Some(source), Some(canonical)) => {
            match compare_recordings(source, canonical, case.order) {
                Ok(()) => Verdict::Passed,
                Err(mismatch) => Verdict::Failed {
                    reason: format!("the two spellings recorded different results: {mismatch}"),
                },
            }
        }
        (None, None) if error_key(stored) == error_key(&canonical) => Verdict::Passed,
        _ => Verdict::Failed {
            reason: "one spelling raised an error and the other did not, or the errors differ"
                .to_owned(),
        },
    }
}

fn error_key(result: &StoredResult) -> Option<(Option<i64>, Option<String>)> {
    let error = result.metadata.error.as_ref()?;
    Some((error.errno, error.sqlstate.clone()))
}

fn replayed_verdict(
    case: &LoadedCase,
    (stored, replayed): (&StoredResult, Replayed),
    policy: &Policy,
    raises: &BTreeMap<String, Vec<SourceLocation>>,
) -> Verdict {
    match (&stored.rows, replayed) {
        (None, Err(_)) => Verdict::Passed,
        (None, Ok(_)) => Verdict::Failed {
            reason: format!(
                "Snowflake raised error {}, but DuckDB returned rows",
                stored.metadata.error.as_ref().map_or_else(
                    || "unknown".to_owned(),
                    |error| format!(
                        "{} (SQLSTATE {})",
                        error.errno.unwrap_or_default(),
                        error.sqlstate.as_deref().unwrap_or("unknown")
                    )
                )
            ),
        },
        (Some(_), Err(error)) => Verdict::Failed {
            reason: format!(
                "Snowflake returned rows, but DuckDB failed{}: {error}",
                raised_at(&error, raises)
            ),
        },
        (Some(recorded), Ok(replayed)) => {
            match compare_with_replay(recorded, &replayed, case.order, &policy.comparator) {
                Ok(()) => Verdict::Passed,
                Err(mismatch) => microsecond_verdict(case, recorded, &replayed, policy)
                    .unwrap_or_else(|| Verdict::Failed {
                        reason: mismatch.to_string(),
                    }),
            }
        }
    }
}

/// Unproven when the replay differs only in sub-microsecond digits the recording truncated.
fn microsecond_verdict(
    case: &LoadedCase,
    recorded: &sqlquack_compare::comparison::models::RecordedResult,
    replayed: &sqlquack_compare::values::models::ResultSet,
    policy: &Policy,
) -> Option<Verdict> {
    match differs_below_microseconds(recorded, replayed, case.order, &policy.comparator) {
        Ok(true) => Some(Verdict::Unproven {
            reason: "the connector stored the declared nanosecond columns in microseconds, which \
                     it does for a whole column when any row lies outside the nanosecond range, \
                     and DuckDB's values differ only below a microsecond"
                .to_owned(),
        }),
        Ok(false) => None,
        Err(error) => Some(Verdict::Failed {
            reason: format!("the replay's sub-microsecond digits could not be dropped: {error}"),
        }),
    }
}

/// ` at <line>:<column>, …` for the source expressions whose raise `error` is, if it names one.
fn raised_at(error: &str, raises: &BTreeMap<String, Vec<SourceLocation>>) -> String {
    let Some(locations) = raise_id(error).and_then(|id| raises.get(id)) else {
        return String::new();
    };
    let places: Vec<String> = locations
        .iter()
        .map(|location| format!("{}:{}", location.line, location.column))
        .collect();
    format!(" at {} of the source", places.join(", "))
}
