//! `raised`: an expression raising an error with a message, valid wherever a value is.

use sqlquack_core::failures::constants::{RAISE_ID_MARKER, TAG_END, UNIDENTIFIED_RAISE};

use crate::blocks::constants::RAISE_MARKER;
use crate::raises::_helpers::slots::tag_length;
use crate::raises::constants::RAISE_OPENING;

/// Raises `message`, `<tag>#?: <text>` until identified: a failing constant cast, typed NULL.
pub fn raised(message: &str) -> String {
    let message = match tag_length(message) {
        Some(length) if message[length..].starts_with(TAG_END) => format!(
            "{}{RAISE_ID_MARKER}{UNIDENTIFIED_RAISE}{}",
            &message[..length],
            &message[length..]
        ),
        _ => message.to_owned(),
    };
    format!(
        "{RAISE_OPENING}{}{RAISE_MARKER}",
        message.replace('\'', "''")
    )
}
