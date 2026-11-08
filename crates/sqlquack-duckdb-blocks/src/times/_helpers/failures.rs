//! What text that is not a time yields.

use sqlquack_core::failures::constants::{
    TIME_FORMAT_MISMATCH, TIME_NOT_RECOGNIZED, VARIANT_NOT_CAST,
};
use sqlquack_core::failures::main::tagged_message::tagged_message;

use crate::blocks::constants::VARIANT_CAST_MESSAGE;
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::times::constants::{TIME_FORMAT_MESSAGE, TIME_TEXT_MESSAGE};

/// NULL when trying, else Snowflake's error for text that is not a time (in a format), or a VARIANT's.
pub(crate) fn time_failure(failure: TextFailure, formatted: bool) -> String {
    match failure {
        TextFailure::Null => "NULL".to_owned(),
        TextFailure::Text if formatted => {
            raised(&tagged_message(TIME_FORMAT_MISMATCH, TIME_FORMAT_MESSAGE))
        }
        TextFailure::Text => raised(&tagged_message(TIME_NOT_RECOGNIZED, TIME_TEXT_MESSAGE)),
        TextFailure::Variant => raised(&tagged_message(VARIANT_NOT_CAST, VARIANT_CAST_MESSAGE)),
    }
}
