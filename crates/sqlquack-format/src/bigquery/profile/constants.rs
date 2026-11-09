//! Constants of BigQuery's GoogleSQL's format profile.

/// BigQuery calls the guide keeps on one line however wide.
pub(crate) const INLINE_CALLS: [&str; 1] = ["JSON_OBJECT"];
/// Words before a BigQuery set operator that start its line, such as `LEFT UNION ALL`.
pub(crate) const SET_OPERATOR_PREFIXES: [&str; 4] = ["INNER", "LEFT", "FULL", "OUTER"];
pub(crate) const OPAQUE_STATEMENT_FAILURE: &str =
    "formatter does not lay out a statement the BigQuery parser keeps as opaque text";
