//! `inline_timestamp`: `text_to_timestamp` written out, for its helper macros' bodies and VARIANT reads.

use crate::blocks::models::TextFailure;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::readings::{text_to_date, timestamp_read};
use crate::temporal::_helpers::text::{bound, whitespace_trimmed};
use crate::temporal::constants::TIMESTAMP_TRIMMED;

/// `text_to_timestamp` written out; a VARIANT read keeps the spelling released VARIANT macros pin.
pub fn inline_timestamp(value: &str, (on_failure, date_only): (TextFailure, bool)) -> String {
    once(value, "timestamp_text", |sql| {
        let compact = !matches!(on_failure, TextFailure::Variant);
        let trimmed = match compact {
            true => TIMESTAMP_TRIMMED.to_owned(),
            false => whitespace_trimmed(sql),
        };
        let read = match date_only {
            true => text_to_date(sql, (&trimmed, on_failure)),
            false => timestamp_read(sql, (&trimmed, on_failure, compact)),
        };
        match compact {
            true => bound(&whitespace_trimmed(sql), TIMESTAMP_TRIMMED, &read),
            false => read,
        }
    })
}
