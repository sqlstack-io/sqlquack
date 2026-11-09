//! `judge_recording`: a recorded result against a replay, with microsecond-only matches apart.

use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_compare::comparison::models::{RecordedResult, RowOrder};
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::models::ComparatorPolicy;

use crate::verify::_helpers::microsecond_evidence::{
    differs_below_microseconds, truncated_columns,
};
use crate::verify::models::RecordingMatch;

/// `replayed` against `recorded`; microsecond-held nanosecond columns match at microseconds at best.
pub fn judge_recording(
    recorded: &RecordedResult,
    replayed: &ResultSet,
    order: RowOrder,
    comparator: &ComparatorPolicy,
) -> Result<RecordingMatch, String> {
    let exact = compare_with_replay(recorded, replayed, order, comparator);
    if truncated_columns(recorded).is_empty() {
        return Ok(exact.map_or_else(RecordingMatch::Differs, |()| RecordingMatch::Matches));
    }
    let Err(mismatch) = exact else {
        return Ok(RecordingMatch::MatchesAtMicroseconds);
    };
    match differs_below_microseconds(recorded, replayed, order, comparator) {
        Ok(true) => Ok(RecordingMatch::MatchesAtMicroseconds),
        Ok(false) => Ok(RecordingMatch::Differs(mismatch)),
        Err(error) => Err(format!(
            "the replay's sub-microsecond digits could not be dropped: {error}"
        )),
    }
}
