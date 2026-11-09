//! `helper_call`: a call of a helper macro.

use crate::macros::constants::HELPER_CALL_PREFIX;

/// `sqlquack.<name>(<arguments>)`.
pub fn helper_call(name: &str, arguments: &[&str]) -> String {
    format!("{HELPER_CALL_PREFIX}{name}({})", arguments.join(", "))
}
