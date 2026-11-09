//! Functions typed: lookups.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::functions::typed::constants::TYPED_DISPATCH_GROUP_SPECS;
use crate::functions::typed::constants::TYPED_FUNCTION_SPECS;
use crate::functions::typed::main::typed_function_spec_by_canonical_upper::typed_function_spec_by_canonical_upper;
use crate::functions::typed::models::TypedDispatchGroup;
use crate::functions::typed::models::TypedFunctionSpec;

/// Canonical typed function spec map.
pub(crate) static TYPED_FUNCTION_SPEC_BY_CANONICAL: LazyLock<
    HashMap<&'static str, &'static TypedFunctionSpec>,
> = LazyLock::new(|| {
    TYPED_FUNCTION_SPECS
        .iter()
        .map(|spec| (spec.canonical_name, spec))
        .collect()
});

/// Upper-case typed-function aliases, normalized before `parse_typed_function` dispatch.
pub(crate) static TYPED_FUNCTION_ALIAS_TO_CANONICAL: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        TYPED_FUNCTION_SPECS
            .iter()
            .flat_map(|spec| {
                spec.aliases
                    .iter()
                    .map(|&alias| (alias, spec.canonical_name))
            })
            .collect()
    });

pub(crate) static TYPED_DISPATCH_GROUP_BY_NAME: LazyLock<
    HashMap<&'static str, TypedDispatchGroup>,
> = LazyLock::new(|| {
    TYPED_DISPATCH_GROUP_SPECS
        .iter()
        .map(|spec| (spec.name, spec.group))
        .collect()
});

/// Returns typed function spec by canonical or alias uppercased name.
pub(crate) fn typed_function_spec_by_name_upper(
    upper_name: &str,
) -> Option<&'static TypedFunctionSpec> {
    let canonical_name = TYPED_FUNCTION_ALIAS_TO_CANONICAL
        .get(upper_name)
        .copied()
        .unwrap_or(upper_name);
    typed_function_spec_by_canonical_upper(canonical_name)
}
