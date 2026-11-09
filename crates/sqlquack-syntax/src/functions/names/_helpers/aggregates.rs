//! Functions names: aggregates.

use crate::functions::names::_helpers::lookups::AGGREGATE_FUNCTION_NAME_SET;

/// Returns true if the given uppercased name should use aggregate parsing behavior.
pub(crate) fn is_aggregate_function_name_upper(upper_name: &str) -> bool {
    AGGREGATE_FUNCTION_NAME_SET.contains(upper_name)
}

/// Returns true if the given name (any casing) should use aggregate parsing behavior.
pub(crate) fn is_aggregate_function_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    is_aggregate_function_name_upper(upper.as_str())
}
