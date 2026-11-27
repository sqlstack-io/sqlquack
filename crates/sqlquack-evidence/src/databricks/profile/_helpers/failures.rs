//! The kind of a Databricks error: its error class, and for a malformed cast the type cast to.

use sqlquack_diagnostics::failures::models::ErrorKind;

use crate::databricks::profile::constants::{
    CAST_INVALID_INPUT, CAST_TARGET_MARKER, ERROR_CLASS_KINDS, TEXT_TARGET_KINDS,
};
use crate::results::models::RecordedFailure;

/// The kind of the recorded Databricks error, `None` for a class no kind is known for.
pub(crate) fn databricks_kind(failure: &RecordedFailure) -> Option<ErrorKind> {
    let message = failure.message.as_deref()?;
    let class = message.strip_prefix('[')?.split_once(']')?.0;
    let class = class.split_once('.').map_or(class, |(class, _)| class);
    if class == CAST_INVALID_INPUT {
        let target = message.split_once(CAST_TARGET_MARKER)?.1.split_once('"')?.0;
        return TEXT_TARGET_KINDS
            .iter()
            .find(|(start, _)| target.starts_with(start))
            .map(|(_, kind)| *kind);
    }
    ERROR_CLASS_KINDS
        .iter()
        .find(|(known, _)| *known == class)
        .map(|(_, kind)| *kind)
}
