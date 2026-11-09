//! Constants of PostgreSQL's format profile.

/// PostgreSQL calls the guide keeps on one line however wide.
pub(crate) const INLINE_CALLS: [&str; 15] = [
    "ARRAY_LENGTH",
    "CAST",
    "COUNT",
    "DATE_PART",
    "EXP",
    "EXTRACT",
    "JSON_AGG",
    "LOGICAL_OR",
    "MODE",
    "PERCENTILE_CONT",
    "SUBSTRING",
    "SUM",
    "TRIM",
    "TRY_CAST",
    "XMLELEMENT",
];
/// PostgreSQL table functions the guide keeps on one line in a FROM clause, however wide.
pub(crate) const INLINE_SOURCES: [&str; 1] = ["UNNEST"];
/// PostgreSQL quantifiers of an array or subquery comparison, such as `= ANY(...)`.
pub(crate) const QUANTIFIERS: [&str; 3] = ["ANY", "SOME", "ALL"];
/// The PostgreSQL quantifier the guide keeps spaced from `(` after a pattern operator.
pub(crate) const SPACED_QUANTIFIER: &str = "ALL";
/// The PostgreSQL clause of SELECT ... INTO, which starts its own line.
pub(crate) const INTO_CLAUSE: &str = "INTO";
/// The PostgreSQL clauses that may follow a recursive CTE's body, each with the word before its last token: `SEARCH ... SET column` and `CYCLE ... USING column`.
pub(crate) const CTE_CLAUSES: [(&str, &str); 2] = [("SEARCH", "SET"), ("CYCLE", "USING")];
/// The bytes PostgreSQL lexes as operator characters beyond the shared ones.
pub(crate) const OPERATOR_BYTES: &str = "@`";
pub(crate) const OPAQUE_STATEMENT_FAILURE: &str =
    "formatter does not lay out a statement the PostgreSQL parser keeps as opaque text";
pub(crate) const SCANNER_MISMATCH_FAILURE: &str =
    "formatter tokens differ from the PostgreSQL scanner's, so the layout cannot be checked";
