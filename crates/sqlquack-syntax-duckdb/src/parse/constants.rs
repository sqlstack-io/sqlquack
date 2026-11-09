//! The grammar's word sets.

use sqlquack_syntax::expressions::types::models::IntervalUnit;

/// Words that begin a query: a parenthesis holding one is a subquery.
pub(crate) const QUERY_STARTS: [&str; 13] = [
    "SELECT",
    "WITH",
    "VALUES",
    "FROM",
    "TABLE",
    "DESCRIBE",
    "DESC",
    "SUMMARIZE",
    "SHOW",
    "PIVOT",
    "PIVOT_WIDER",
    "UNPIVOT",
    "PIVOT_LONGER",
];

/// The words `Statement`'s alternatives begin with, in lower case, which name a statement's gap.
pub(crate) const STATEMENT_STARTS: [&str; 9] = [
    "alter",
    "connect",
    "create",
    "disconnect",
    "drop",
    "insert",
    "update",
    "register",
    "destroy",
];

/// The names the tree gives the cube, rollup and grouping-set items of `GROUP BY`.
pub(crate) const ROLLUP: &str = "ROLLUP";
pub(crate) const CUBE: &str = "CUBE";
pub(crate) const GROUPING_SETS: &str = "GROUPING SETS";

/// The interval unit words (`YearKeyword` to `MillenniumKeyword`), with their shared-tree unit.
pub(crate) const INTERVAL_UNITS: [(&str, Option<IntervalUnit>); 26] = [
    ("YEAR", Some(IntervalUnit::Year)),
    ("YEARS", Some(IntervalUnit::Year)),
    ("MONTH", Some(IntervalUnit::Month)),
    ("MONTHS", Some(IntervalUnit::Month)),
    ("DAY", Some(IntervalUnit::Day)),
    ("DAYS", Some(IntervalUnit::Day)),
    ("HOUR", Some(IntervalUnit::Hour)),
    ("HOURS", Some(IntervalUnit::Hour)),
    ("MINUTE", Some(IntervalUnit::Minute)),
    ("MINUTES", Some(IntervalUnit::Minute)),
    ("SECOND", Some(IntervalUnit::Second)),
    ("SECONDS", Some(IntervalUnit::Second)),
    ("MILLISECOND", Some(IntervalUnit::Millisecond)),
    ("MILLISECONDS", Some(IntervalUnit::Millisecond)),
    ("MICROSECOND", Some(IntervalUnit::Microsecond)),
    ("MICROSECONDS", Some(IntervalUnit::Microsecond)),
    ("WEEK", Some(IntervalUnit::Week)),
    ("WEEKS", Some(IntervalUnit::Week)),
    ("QUARTER", Some(IntervalUnit::Quarter)),
    ("QUARTERS", Some(IntervalUnit::Quarter)),
    ("DECADE", None),
    ("DECADES", None),
    ("CENTURY", None),
    ("CENTURIES", None),
    ("MILLENNIUM", None),
    ("MILLENNIA", None),
];

/// The characters an operator token is made of (`Tokenizer::CharacterIsOperator`).
pub(crate) const GENERIC_OPERATOR_CHARACTERS: [u8; 14] = *b"+-*/%^<>=~!@&|";

/// Operators with their own precedence (`OperatorMatcher::HasSpecialPrecedence`).
pub(crate) const SPECIAL_PRECEDENCE_OPERATORS: [&str; 18] = [
    "=", "<", ">", "~", "->", "<=", ">=", "!=", "==", "<>", "~~", "~*", "!~", "~~*", "~~~", "!~~",
    "!~*", "!~~*",
];

/// The type name the tree marks as spelt `INTEGER`.
pub(crate) const INTEGER_TYPE: &str = "INTEGER";

/// The type name the tree marks as spelt `REAL`.
pub(crate) const REAL_TYPE: &str = "REAL";

/// The word that gives `TIME` and `TIMESTAMP` a time zone, unlike `WITHOUT`.
pub(crate) const WITH_TIME_ZONE: &str = "WITH";

/// The number of types `MAP(key, value)` takes.
pub(crate) const MAP_TYPE_PARAMETERS: usize = 2;

/// The boolean literal word that is true.
pub(crate) const TRUE_WORD: &str = "TRUE";

/// The cast call that gives `NULL` instead of failing.
pub(crate) const TRY_CAST: &str = "TRY_CAST";

/// The type name that `PRECISION` may follow.
pub(crate) const DOUBLE_TYPE: &str = "DOUBLE";

/// The call whose parentheses may not be empty, unlike `ROW` and `GROUPING`.
pub(crate) const COALESCE: &str = "COALESCE";

/// The number of arguments `IFNULL` takes.
pub(crate) const IFNULL_ARGUMENTS: usize = 2;

/// The join type word that marks a join as written `INNER`.
pub(crate) const INNER_JOIN: &str = "INNER";

/// Words after a DML target that start its next clause, so they are never its bare alias.
pub(crate) const TARGET_ALIAS_STOPS: [&str; 4] = ["SET", "USING", "WHERE", "RETURNING"];

/// The transformer's refusal of a qualified column on the left of `UPDATE ... SET`.
pub(crate) const QUALIFIED_SET: &str = "Qualified column names in UPDATE .. SET not supported";

/// `OrAction` (insert.gram): the actions `INSERT OR` takes.
pub(crate) const INSERT_OR_ACTIONS: [&str; 2] = ["REPLACE", "IGNORE"];

/// Words that start a column constraint, so a column before one has no type.
pub(crate) const COLUMN_CONSTRAINT_STARTS: [&str; 12] = [
    "GENERATED",
    "AS",
    "NULL",
    "NOT",
    "UNIQUE",
    "PRIMARY",
    "DEFAULT",
    "CHECK",
    "REFERENCES",
    "USING",
    "COLLATE",
    "CONSTRAINT",
];

/// Words that start a table constraint in a column list.
pub(crate) const TABLE_CONSTRAINT_STARTS: [&str; 5] =
    ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK", "FOREIGN"];

/// The column compressions `EnumUtil` knows, in lower case; `auto` has its own refusal.
pub(crate) const COMPRESSIONS: [&str; 15] = [
    "uncompressed",
    "constant",
    "rle",
    "dictionary",
    "pfor",
    "bitpacking",
    "fsst",
    "chimp",
    "patas",
    "zstd",
    "alp",
    "alprd",
    "roaring",
    "empty",
    "dict_fsst",
];

/// The compression that means no choice, which a column may not name.
pub(crate) const AUTO_COMPRESSION: &str = "auto";

/// `DEFAULT` read as a value.
pub(crate) const DEFAULT_WORD: &str = "DEFAULT";

/// The type names `VARCHAR` resolves from, compared without case.
pub(crate) const VARCHAR_NAMES: [&str; 7] = [
    "VARCHAR", "TEXT", "STRING", "CHAR", "BPCHAR", "NVARCHAR", "NCHAR",
];

/// The only object kind that may be PERSISTENT.
pub(crate) const SECRET_KIND: &str = "SECRET";

/// The transformer's refusals of a `CREATE` or `DROP` the grammar accepts.
pub(crate) const PERSISTENT_ONLY_SECRET: &str = "PERSISTENT is only supported for CREATE SECRET";
pub(crate) const REPLACE_AND_IF_NOT_EXISTS: &str =
    "Cannot specify both OR REPLACE and IF NOT EXISTS within single create statement";
pub(crate) const CTAS_NEEDS_SELECT: &str = "CREATE TABLE AS requires a SELECT clause";
pub(crate) const NO_COLUMNS: &str = "Table must have at least one column!";
pub(crate) const ON_COMMIT_DELETE: &str = "Only ON COMMIT PRESERVE ROWS is supported";
pub(crate) const STORED_GENERATED: &str = "Can not create a STORED generated column!";
pub(crate) const SUBQUERY_IN_CHECK: &str = "subqueries prohibited in CHECK constraints";
pub(crate) const GENERATED_ALWAYS: &str =
    "for a generated column, GENERATED ALWAYS must be specified";
pub(crate) const GENERATED_QUALIFIED: &str =
    "Qualified (tbl.name) column references are not allowed inside of generated column expressions";
pub(crate) const DEFAULT_TWICE: &str = "Cannot define a default value twice";
pub(crate) const COLLATE_TWICE: &str = "multiple COLLATE clauses not allowed";
pub(crate) const GENERATED_COLLATE: &str = "Collations are not supported on generated columns";
pub(crate) const GENERATED_DEFAULT: &str = "Not allowed to set default on a generated column";
pub(crate) const COMPRESSION_AUTO: &str = "Unrecognized option for column compression, expected none, uncompressed, rle, dictionary, pfor, bitpacking, fsst, chimp, patas, zstd, alp, alprd or roaring";
pub(crate) const FK_COLUMN_COUNT: &str =
    "The number of referencing and referenced columns for foreign keys must be the same";
pub(crate) const TEMPORARY_INDEX: &str = "Temporary indexes are not supported";
pub(crate) const VARCHAR_COLLATION: &str = "Only VARCHAR columns can have collations!";
pub(crate) const COLLATION_TYPE: &str = "Specify the VARCHAR type to set a collation";
pub(crate) const ALTER_ONE: &str = "Only one ALTER command per statement is supported";
pub(crate) const ALTER_SCHEMA: &str = "Altering schemas is not yet supported";
pub(crate) const ALTER_SEQUENCE_RENAME: &str = "Renaming sequences is not yet supported";
pub(crate) const ALTER_SEQUENCE_OWNED_TWICE: &str = "Owned by value should be passed at most once";
pub(crate) const ALTER_SEQUENCE_OPTION: &str = "ALTER SEQUENCE option not yet supported";
pub(crate) const ALTER_DROP_CONSTRAINT: &str = "No support for that ALTER TABLE option yet!";
pub(crate) const ALTER_TYPE_USING: &str =
    "Omitting the type is only possible in combination with USING";
pub(crate) const ADD_NO_TYPE: &str = "Column definition requires a type or generated expression";
pub(crate) const ADD_GENERATED: &str =
    "Adding generated columns after table creation is not supported yet";
pub(crate) const ADD_PRIMARY_KEY: &str =
    "Adding columns with PRIMARY KEY constraints is not supported yet";
pub(crate) const ADD_CHECK: &str = "Adding columns with CHECK constraints is not supported yet";
pub(crate) const ADD_FOREIGN_KEY: &str =
    "Adding columns with FOREIGN KEY constraints is not supported yet";
pub(crate) const ADD_NESTED_NOT_NULL: &str =
    "Adding NOT NULL constraints to nested fields is not supported";
pub(crate) const ADD_NESTED_UNIQUE: &str =
    "Adding UNIQUE constraints to nested fields is not supported";
pub(crate) const ADD_NESTED_COMPRESSION: &str =
    "Adding compression to nested fields is not supported";
pub(crate) const OIDS: &str = "OIDS for index are not yet implemented.";
pub(crate) const MODIFIERS_ONLY_IN_ORDER_BY: &str =
    "ORDER BY modifiers are only supported in the ORDER BY option";
pub(crate) const PARTITION_PATH_VALUE: &str = "PARTITION_PATH expects an expression";
pub(crate) const PARTITION_PATH_ONE: &str = "PARTITION_PATH expects a single expression";
pub(crate) const VIEW_OPTION: &str =
    "Only DEFER_BINDING is currently supported as option for CREATE VIEW";
pub(crate) const VIEW_OPTION_CONSTANT: &str = "Defer binding option must be a constant value";
pub(crate) const VIEW_OPTION_BOOLEAN: &str = "Defer binding option must be a boolean";
pub(crate) const RECURSIVE_LIMIT: &str = "LIMIT or OFFSET in a recursive query is not allowed";
pub(crate) const RECURSIVE_ORDER: &str = "ORDER BY in a recursive query is not allowed";
pub(crate) const INDEX_NAME: &str = "Please provide an index name, e.g., CREATE INDEX my_name ...";
pub(crate) const INDEX_COLLATION: &str = "Index with collation not supported yet!";
pub(crate) const PARTIAL_INDEX: &str = "Creating partial indexes is not supported currently";
pub(crate) const INDEX_OPTION_CONSTANT: &str = "Create index option must be a constant value";
pub(crate) const TRIGGER_NEW_TWICE: &str =
    "NEW TABLE cannot be specified multiple times in REFERENCING clause";
pub(crate) const TRIGGER_OLD_TWICE: &str =
    "OLD TABLE cannot be specified multiple times in REFERENCING clause";
pub(crate) const TRIGGER_ALIASES: &str = "REFERENCING aliases must be distinct";
pub(crate) const SECRET_TYPE: &str = "Failed to create secret - secret must have a type defined";
pub(crate) const SECRET_NO_VALUE: &str = "No expression in GenericCopyOption";
pub(crate) const SECRET_TYPE_CONSTANT: &str = "Can not combine a non-constant expression for the secret type with a default-named secret. Either provide an explicit secret name or use a constant expression for the secret type.";
pub(crate) const REPOSITORY_REPLACE: &str = "Cannot combine OR REPLACE with IF NOT EXISTS";
pub(crate) const SEQUENCE_CONSTANT: &str = "Expected constant expression.";
pub(crate) const SEQUENCE_MINUS_CHILD: &str =
    "Expected constant expression as child of minus function";
pub(crate) const SEQUENCE_ZERO: &str = "Increment must not be zero";
pub(crate) const SEQUENCE_OWNED: &str = "Unrecognized option \"owned\" for CREATE SEQUENCE";
pub(crate) const DROP_SECRET_BEHAVIOR: &str =
    "CASCADE and RESTRICT are not supported for DROP SECRET";
pub(crate) const DROP_COLLATION: &str = "Cannot drop collation yet";
pub(crate) const DROP_SECRET_STORAGE: &str =
    "Can not combine TEMPORARY with specifying a storage for drop secret";
/// Option names the copy transformer reads specially, compared without case.
pub(crate) const ORDER_BY_OPTION: &str = "ORDER_BY";
pub(crate) const PARTITION_PATH_OPTION: &str = "PARTITION_PATH";
/// The only `CREATE VIEW` option, compared without case.
pub(crate) const DEFER_BINDING: &str = "DEFER_BINDING";
pub(crate) const TEMPORARY_TRIGGER: &str = "Temporary triggers are not supported";
pub(crate) const FK_ACTIONS: &str =
    "FOREIGN KEY constraints cannot use CASCADE, SET NULL or SET DEFAULT";
pub(crate) const RECURSIVE_VIEW_COLUMNS: &str = "CREATE RECURSIVE VIEW requires a column list";
pub(crate) const DROP_MATERIALIZED: &str = "Cannot drop MATERIALIZED VIEW yet";
pub(crate) const DROP_ONE: &str = "Can only drop one object at a time";
/// The statement kinds `PivotEntryCheck` names, which refuse a PIVOT whose values come from the data.
pub(crate) const VIEW_KIND: &str = "view";
pub(crate) const MACRO_KIND: &str = "macro";
pub(crate) const EXPLAIN_SQL_KIND: &str = "EXPLAIN (SQL) statement";
/// The drop kind of `DEALLOCATE`, which DuckDB reads as dropping a prepared statement.
pub(crate) const PREPARED_STATEMENT_KIND: &str = "PREPARED STATEMENT";
/// The first words of the statements `PREPARE` takes: queries, `INSERT`, `UPDATE`, `DELETE` and `COPY`.
pub(crate) const PREPARABLE_STARTS: [&str; 18] = [
    "SELECT",
    "FROM",
    "VALUES",
    "TABLE",
    "PIVOT",
    "PIVOT_WIDER",
    "UNPIVOT",
    "PIVOT_LONGER",
    "DESCRIBE",
    "DESC",
    "SUMMARIZE",
    "SHOW",
    "WITH",
    "INSERT",
    "UPDATE",
    "DELETE",
    "TRUNCATE",
    "COPY",
];
/// The type DuckDB gives a statement by its first word, for those `PREPARE` refuses.
pub(crate) const STATEMENT_TYPES: [(&str, &str); 32] = [
    ("MERGE", "MERGE_INTO_STATEMENT"),
    ("CREATE", "CREATE_STATEMENT"),
    ("DROP", "DROP_STATEMENT"),
    ("DEALLOCATE", "DROP_STATEMENT"),
    ("ALTER", "ALTER_STATEMENT"),
    ("COMMENT", "ALTER_STATEMENT"),
    ("ATTACH", "ATTACH_STATEMENT"),
    ("DETACH", "DETACH_STATEMENT"),
    ("PRAGMA", "PRAGMA_STATEMENT"),
    ("IMPORT", "PRAGMA_STATEMENT"),
    ("VACUUM", "VACUUM_STATEMENT"),
    ("ANALYZE", "VACUUM_STATEMENT"),
    ("ANALYSE", "VACUUM_STATEMENT"),
    ("EXPORT", "EXPORT_STATEMENT"),
    ("EXECUTE", "EXECUTE_STATEMENT"),
    ("PREPARE", "PREPARE_STATEMENT"),
    ("BEGIN", "TRANSACTION_STATEMENT"),
    ("START", "TRANSACTION_STATEMENT"),
    ("COMMIT", "TRANSACTION_STATEMENT"),
    ("END", "TRANSACTION_STATEMENT"),
    ("ROLLBACK", "TRANSACTION_STATEMENT"),
    ("ABORT", "TRANSACTION_STATEMENT"),
    ("USE", "SET_STATEMENT"),
    ("SET", "SET_STATEMENT"),
    ("RESET", "SET_STATEMENT"),
    ("CHECKPOINT", "CALL_STATEMENT"),
    ("CALL", "CALL_STATEMENT"),
    ("INSTALL", "LOAD_STATEMENT"),
    ("LOAD", "LOAD_STATEMENT"),
    ("EXPLAIN", "EXPLAIN_STATEMENT"),
    ("CONNECT", "CONNECT_STATEMENT"),
    ("DISCONNECT", "DISCONNECT_STATEMENT"),
];
/// The `ExplainableStatements` alternatives `EXPLAIN` takes besides a query, by first word.
pub(crate) const EXPLAINABLE_STARTS: [&str; 26] = [
    "ALTER",
    "ANALYZE",
    "ANALYSE",
    "CALL",
    "CHECKPOINT",
    "COPY",
    "CREATE",
    "DEALLOCATE",
    "DELETE",
    "DROP",
    "EXECUTE",
    "INSERT",
    "LOAD",
    "MERGE",
    "PRAGMA",
    "PREPARE",
    "BEGIN",
    "START",
    "COMMIT",
    "END",
    "ROLLBACK",
    "ABORT",
    "UPDATE",
    "VACUUM",
    "SET",
    "RESET",
];

/// The deepest tree a statement may build, counted after it parses, so that serialising it cannot exhaust a stack.
pub(crate) const MAX_TREE_DEPTH: usize = 8_000;
