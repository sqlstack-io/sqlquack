//! `typed_dispatch_group_by_name_upper`: returns specialized typed parser dispatch group for canonical uppercased name.

use crate::functions::typed::_helpers::lookups::TYPED_DISPATCH_GROUP_BY_NAME;
use crate::functions::typed::models::TypedDispatchGroup;

/// Returns specialized typed parser dispatch group for canonical uppercased name.
pub fn typed_dispatch_group_by_name_upper(
    canonical_upper_name: &str,
) -> Option<TypedDispatchGroup> {
    TYPED_DISPATCH_GROUP_BY_NAME
        .get(canonical_upper_name)
        .copied()
}
