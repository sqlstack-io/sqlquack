//! `raised`: an expression raising an error with a message, valid wherever a value is.

use crate::blocks::constants::RAISE_MARKER;

/// Raises `message` when evaluated: a failing constant cast, typed NULL, never volatile.
pub fn raised(message: &str) -> String {
    format!("CAST(CAST('{}{RAISE_MARKER}", message.replace('\'', "''"))
}
