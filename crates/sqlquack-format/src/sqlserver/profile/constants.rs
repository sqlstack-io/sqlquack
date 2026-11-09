//! Constants of SQL Server's Transact-SQL's format profile.

/// SQL Server calls the guide keeps on one line however wide.
pub(crate) const INLINE_CALLS: [&str; 2] = ["CAST", "SUBSTRING"];
/// The words before APPLY in a SQL Server apply.
pub(crate) const APPLY_PREFIXES: [&str; 2] = ["CROSS", "OUTER"];
/// Words after BEGIN that make it a statement of its own, such as `BEGIN TRANSACTION`, not a block.
pub(crate) const BEGIN_STATEMENTS: [&str; 5] = [
    "TRAN",
    "TRANSACTION",
    "DISTRIBUTED",
    "DIALOG",
    "CONVERSATION",
];
/// SQL Server statements whose condition is followed by the statement they run.
pub(crate) const BRANCH_STATEMENTS: [&str; 2] = ["IF", "WHILE"];
/// The words around a SQL Server block's or branch's statements, each on its own line.
pub(crate) const BLOCK_WORDS: [&str; 3] = ["BEGIN", "END", "ELSE"];
/// SQL Server keywords after which a name names a table, view, routine, index or other object, kept as written since the database collation may be case-sensitive.
pub(crate) const OBJECT_PRECEDERS: [&str; 22] = [
    "APPLY",
    "DELETE",
    "EXEC",
    "EXECUTE",
    "FROM",
    "FUNCTION",
    "INDEX",
    "INSERT",
    "INTO",
    "JOIN",
    "MERGE",
    "ON",
    "PROC",
    "PROCEDURE",
    "REFERENCES",
    "SEQUENCE",
    "STATISTICS",
    "TABLE",
    "TRIGGER",
    "UPDATE",
    "USING",
    "VIEW",
];
/// SQL Server clauses that hold a comma-separated list, which bound the search for the FROM a comma belongs to.
pub(crate) const LIST_CLAUSES: [&str; 9] = [
    "BY", "FROM", "OUTPUT", "RETURNS", "SELECT", "SET", "VALUES", "WHERE", "HAVING",
];
/// SQL Server keywords the guide separates from a following `(`: `TOP (n)` and a `WITH (hint)` table hint.
pub(crate) const SPACED_BEFORE_PARENTHESIS: [&str; 2] = ["TOP", "WITH"];
pub(crate) const OPAQUE_STATEMENT_FAILURE: &str =
    "formatter does not lay out a statement the SQL Server parser keeps as opaque text";
pub(crate) const BATCH_SEPARATOR_FAILURE: &str = "formatter does not lay out a GO batch separator, which a client reads and SQL Server never sees";
/// The word a SQL Server client reads, alone on its line, as the end of a batch.
pub(crate) const BATCH_SEPARATOR: &str = "GO";
/// Words that after `FOR` start a clause listing a query's result options, as in `FOR XML PATH, TYPE`.
pub(crate) const FOR_CLAUSES: [&str; 2] = ["XML", "JSON"];
