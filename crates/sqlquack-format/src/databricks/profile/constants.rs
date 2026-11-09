//! Constants of Databricks SQL's format profile.

/// Databricks calls the guide keeps on one line however wide: PostgreSQL's, and OVERLAY.
pub(crate) const INLINE_CALLS: [&str; 16] = [
    "ARRAY_LENGTH",
    "CAST",
    "COUNT",
    "DATE_PART",
    "EXP",
    "EXTRACT",
    "JSON_AGG",
    "LOGICAL_OR",
    "MODE",
    "OVERLAY",
    "PERCENTILE_CONT",
    "SUBSTRING",
    "SUM",
    "TRIM",
    "TRY_CAST",
    "XMLELEMENT",
];
/// Databricks clauses written as a word and `BY`, which list items as GROUP BY does.
pub(crate) const BY_CLAUSES: [&str; 3] = ["CLUSTER", "DISTRIBUTE", "SORT"];
/// Keywords the guide separates from a following `(` in Databricks, as in `UNPIVOT INCLUDE NULLS (...)`.
pub(crate) const SPACED_BEFORE_PARENTHESIS: [&str; 1] = ["NULLS"];
/// Databricks' try-cast operator, laid out as `::` is.
pub(crate) const TRY_CAST_OPERATOR: &str = "?::";
/// The character that, last on a Databricks `--` comment's line, continues the comment onto the next.
pub(crate) const COMMENT_CONTINUATION: char = '\\';
pub(crate) const OPAQUE_STATEMENT_FAILURE: &str =
    "formatter does not lay out a statement the Databricks parser keeps as opaque text";
pub(crate) const HINT_FAILURE: &str = "formatter does not lay out Databricks hints";
pub(crate) const LINE_CONTINUATION_FAILURE: &str =
    "formatter does not lay out a `--` comment that a backslash continues onto the next line";
