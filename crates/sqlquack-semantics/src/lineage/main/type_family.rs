//! `type_family`: the family of a type as spelled.

use crate::lineage::_helpers::checks::families::{
    composite_family, normalized_spelling, scalar_family, type_arguments,
};
use crate::lineage::models::Family;

/// The family of a type as spelled, `NUMBER(38,0)` an integer and `VARCHAR(10)` a string.
pub(crate) fn type_family(spelling: &str) -> Family {
    let normalized: String = normalized_spelling(spelling);
    if normalized.is_empty() {
        return Family::Unknown;
    }
    if let Some((base, inner)) = type_arguments(&normalized)
        && matches!(base, "nullable" | "lowcardinality")
    {
        return type_family(inner);
    }
    composite_family(&normalized).unwrap_or_else(|| scalar_family(&normalized))
}
