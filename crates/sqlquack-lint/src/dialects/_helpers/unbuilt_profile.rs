//! The profile of a dialect the crate was built without.

use crate::dialects::models::LintProfile;
use crate::lint::constants::UNBUILT_DIALECT_FAILURE;

/// A profile that reads nothing, for a dialect built without its feature.
pub(crate) fn unbuilt_profile() -> LintProfile {
    LintProfile {
        tokens: |_| Err(UNBUILT_DIALECT_FAILURE.to_owned()),
        statements: |_, _| Err(UNBUILT_DIALECT_FAILURE.to_owned()),
        predicates: |_, _| Err(UNBUILT_DIALECT_FAILURE.to_owned()),
        own_predicate_parser: false,
        foreign_aggregates: &[],
        plain_call_aggregates: false,
        named_aggregates: &[],
        single_argument_aggregates: &[],
        json_parse_functions: &[],
        folds_upper: false,
        quoted_names_ignore_case: false,
        identifier_quote: '"',
        union_distinct: false,
    }
}
