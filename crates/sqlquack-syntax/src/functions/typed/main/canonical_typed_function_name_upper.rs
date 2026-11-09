//! `canonical_typed_function_name_upper`: returns the canonical uppercased typed-function dispatch key for an uppercased name.

use crate::functions::typed::_helpers::lookups::typed_function_spec_by_name_upper;

/// Returns the canonical uppercased typed-function dispatch key for an uppercased name.
pub fn canonical_typed_function_name_upper(upper_name: &str) -> &str {
    typed_function_spec_by_name_upper(upper_name)
        .map(|spec| spec.canonical_name)
        .unwrap_or(upper_name)
}
