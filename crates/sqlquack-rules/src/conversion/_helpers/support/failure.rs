//! Which failure a text conversion raises.

use sqlquack_duckdb_blocks::blocks::models::TextFailure;

/// NULL for a TRY_ form, else text's own Snowflake error.
pub(crate) fn text_failure(trying: bool) -> TextFailure {
    match trying {
        true => TextFailure::Null,
        false => TextFailure::Text,
    }
}
