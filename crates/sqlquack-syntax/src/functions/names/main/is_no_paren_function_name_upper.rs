//! `is_no_paren_function_name_upper`: returns true if the given uppercased name can be parsed as a no-paren function.

use crate::functions::names::_helpers::lookups::NO_PAREN_FUNCTION_NAME_SET;

/// Returns true if the given uppercased name can be parsed as a no-paren function.
pub fn is_no_paren_function_name_upper(upper_name: &str) -> bool {
    NO_PAREN_FUNCTION_NAME_SET.contains(upper_name)
}
