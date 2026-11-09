//! `is_guard_error`: resource exhaustion must not be treated as a speculative syntax mismatch.

use crate::tokens::errors::Error;

/// Resource exhaustion must not be treated as a speculative syntax mismatch.
pub fn is_guard_error(error: &Error) -> bool {
    matches!(error, Error::Parse { message, .. } if message.starts_with("E_GUARD_"))
}
