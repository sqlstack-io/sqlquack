//! How DuckDB's error compares with the recorded Snowflake error.

use sqlquack_diagnostics::failures::constants::RAISE_ID_MARKER;
use sqlquack_diagnostics::failures::main::engine_failure::engine_failure;
use sqlquack_diagnostics::failures::models::{ErrorKind, FailureMode};
use sqlquack_diagnostics::provenance::models::Provenance;
use sqlquack_diagnostics::snowflake::failures::main::snowflake_kind::snowflake_kind;
use sqlquack_policy::policy::models::Policy;

use crate::results::models::RecordedFailure;
use crate::verify::models::ErrorCheck;
use crate::verify::types::FailureModes;

/// Compares DuckDB's `error` with the `recorded` Snowflake error, given what the conversion used.
pub(crate) fn error_check(
    (recorded, error): (&RecordedFailure, &str),
    provenance: &Provenance,
    (policy, failure_modes): (&Policy, &FailureModes),
) -> ErrorCheck {
    let (Some(errno), Some(sqlstate)) = (recorded.errno, recorded.sqlstate.as_deref()) else {
        return unknown(recorded);
    };
    let Some(kind) = snowflake_kind(errno, sqlstate) else {
        return unknown(recorded);
    };
    if let Some(divergence) = policy.divergences.keys().find(|divergence| {
        error.contains(&format!("{divergence}:"))
            || error.contains(&format!("{divergence}{RAISE_ID_MARKER}"))
    }) {
        return ErrorCheck::DivergenceRaise {
            divergence: divergence.clone(),
        };
    }
    let duckdb = engine_failure(error);
    if let Some(tagged) = duckdb.and_then(|failure| failure.snowflake)
        && tagged.errno != errno
    {
        return ErrorCheck::ErrnoMismatch {
            recorded: errno,
            tagged: tagged.errno,
        };
    }
    if duckdb.map(|failure| failure.kind) != Some(kind) {
        return ErrorCheck::KindMismatch {
            snowflake: kind,
            duckdb: duckdb.map(|failure| failure.kind),
            error: error.to_owned(),
        };
    }
    let declared = provenance
        .rules
        .iter()
        .filter_map(|rule| failure_modes.get(rule))
        .flatten()
        .any(|mode| declares(mode, (kind, errno, sqlstate)));
    match declared {
        true => ErrorCheck::Agrees { kind },
        false => ErrorCheck::Undeclared {
            errno,
            sqlstate: sqlstate.to_owned(),
        },
    }
}

fn unknown(recorded: &RecordedFailure) -> ErrorCheck {
    ErrorCheck::UnknownSnowflakeError {
        errno: recorded.errno,
        sqlstate: recorded.sqlstate.clone(),
    }
}

fn declares(mode: &FailureMode, (kind, errno, sqlstate): (ErrorKind, i64, &str)) -> bool {
    mode.kind == kind
        && mode
            .snowflake
            .iter()
            .any(|code| code.errno == errno && code.sqlstate == sqlstate)
}
