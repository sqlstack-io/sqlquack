use crate::failures::models::{DuckdbFailure, ErrorKind};

/// A DuckDB failure of `kind` that no building block tagged.
pub(crate) fn untagged(kind: ErrorKind) -> Option<DuckdbFailure> {
    Some(DuckdbFailure {
        kind,
        snowflake: None,
    })
}
