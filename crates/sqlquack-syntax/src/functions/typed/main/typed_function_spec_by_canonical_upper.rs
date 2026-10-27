//! `typed_function_spec_by_canonical_upper`: returns typed function spec by canonical uppercased name.

use crate::functions::typed::_helpers::lookups::TYPED_FUNCTION_SPEC_BY_CANONICAL;
use crate::functions::typed::models::TypedFunctionSpec;

/// Returns typed function spec by canonical uppercased name.
pub(crate) fn typed_function_spec_by_canonical_upper(
    canonical_upper_name: &str,
) -> Option<&'static TypedFunctionSpec> {
    TYPED_FUNCTION_SPEC_BY_CANONICAL
        .get(canonical_upper_name)
        .copied()
}
