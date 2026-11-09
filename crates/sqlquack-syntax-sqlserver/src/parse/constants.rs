//! Words the grammar reads specially.

/// Aggregate functions: `*` and `DISTINCT` are allowed in their argument lists.
pub(crate) const AGGREGATES: [&str; 17] = [
    "APPROX_COUNT_DISTINCT",
    "AVG",
    "CHECKSUM_AGG",
    "COUNT",
    "COUNT_BIG",
    "GROUPING",
    "GROUPING_ID",
    "MAX",
    "MIN",
    "STDEV",
    "STDEVP",
    "STRING_AGG",
    "SUM",
    "VAR",
    "VARP",
    "APPROX_PERCENTILE_CONT",
    "APPROX_PERCENTILE_DISC",
];

/// Functions whose first argument is a date part written as a bare word, such as `day`.
pub(crate) const DATEPART_FUNCTIONS: [&str; 7] = [
    "DATEADD",
    "DATEDIFF",
    "DATEDIFF_BIG",
    "DATENAME",
    "DATEPART",
    "DATETRUNC",
    "DATE_BUCKET",
];

/// The date parts those functions take, lowercase; the engine refuses any other with 155.
pub(crate) const DATE_PARTS: [&str; 40] = [
    "year",
    "yy",
    "yyyy",
    "quarter",
    "qq",
    "q",
    "month",
    "mm",
    "m",
    "dayofyear",
    "dy",
    "y",
    "day",
    "dd",
    "d",
    "week",
    "wk",
    "ww",
    "weekday",
    "dw",
    "w",
    "hour",
    "hh",
    "minute",
    "mi",
    "n",
    "second",
    "ss",
    "s",
    "millisecond",
    "ms",
    "microsecond",
    "mcs",
    "nanosecond",
    "ns",
    "tzoffset",
    "tz",
    "iso_week",
    "isowk",
    "isoww",
];

/// Functions called without parentheses.
pub(crate) const NILADIC_FUNCTIONS: [&str; 5] = [
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "SESSION_USER",
    "SYSTEM_USER",
    "USER",
];

/// One-part names SQL Server 2022 reads as built-in function calls, probed under PARSEONLY and sorted.
pub(crate) const BUILT_IN_FUNCTIONS: [&str; 260] = [
    "ABS",
    "ACOS",
    "APPLOCK_MODE",
    "APPLOCK_TEST",
    "APPROX_COUNT_DISTINCT",
    "APPROX_PERCENTILE_CONT",
    "APPROX_PERCENTILE_DISC",
    "APP_NAME",
    "ASCII",
    "ASIN",
    "ASSEMBLYPROPERTY",
    "ASYMKEYPROPERTY",
    "ASYMKEY_ID",
    "ATAN",
    "ATN2",
    "AVG",
    "BINARY_CHECKSUM",
    "BIT_COUNT",
    "CAST",
    "CEILING",
    "CERTENCODED",
    "CERTPRIVATEKEY",
    "CERTPROPERTY",
    "CERT_ID",
    "CHAR",
    "CHARINDEX",
    "CHECKSUM",
    "CHECKSUM_AGG",
    "CHOOSE",
    "COLLATIONPROPERTY",
    "COLUMNPROPERTY",
    "COLUMNS_UPDATED",
    "COL_LENGTH",
    "COL_NAME",
    "COMPRESS",
    "CONCAT",
    "CONCAT_WS",
    "CONNECTIONPROPERTY",
    "CONTEXT_INFO",
    "COS",
    "COT",
    "COUNT",
    "COUNT_BIG",
    "CRYPT_GEN_RANDOM",
    "CUME_DIST",
    "CURRENT_REQUEST_ID",
    "CURRENT_TIMEZONE",
    "CURRENT_TIMEZONE_ID",
    "CURRENT_TRANSACTION_ID",
    "CURSOR_STATUS",
    "DATABASEPROPERTYEX",
    "DATABASE_PRINCIPAL_ID",
    "DATALENGTH",
    "DATEADD",
    "DATEDIFF",
    "DATEDIFF_BIG",
    "DATEFROMPARTS",
    "DATENAME",
    "DATEPART",
    "DATETIME2FROMPARTS",
    "DATETIMEFROMPARTS",
    "DATETIMEOFFSETFROMPARTS",
    "DATETRUNC",
    "DATE_BUCKET",
    "DAY",
    "DB_ID",
    "DB_NAME",
    "DECOMPRESS",
    "DECRYPTBYASYMKEY",
    "DECRYPTBYCERT",
    "DECRYPTBYKEY",
    "DECRYPTBYKEYAUTOASYMKEY",
    "DECRYPTBYKEYAUTOCERT",
    "DECRYPTBYPASSPHRASE",
    "DEGREES",
    "DENSE_RANK",
    "DIFFERENCE",
    "EDGE_ID_FROM_PARTS",
    "ENCRYPTBYASYMKEY",
    "ENCRYPTBYCERT",
    "ENCRYPTBYKEY",
    "ENCRYPTBYPASSPHRASE",
    "EOMONTH",
    "ERROR_LINE",
    "ERROR_MESSAGE",
    "ERROR_NUMBER",
    "ERROR_PROCEDURE",
    "ERROR_SEVERITY",
    "ERROR_STATE",
    "EVENTDATA",
    "EXP",
    "FILEGROUPPROPERTY",
    "FILEGROUP_ID",
    "FILEGROUP_NAME",
    "FILENAME",
    "FILEPATH",
    "FILEPROPERTY",
    "FILE_ID",
    "FILE_IDEX",
    "FILE_NAME",
    "FIRST_VALUE",
    "FLOOR",
    "FORMAT",
    "FORMATMESSAGE",
    "FULLTEXTCATALOGPROPERTY",
    "FULLTEXTSERVICEPROPERTY",
    "GETANSINULL",
    "GETDATE",
    "GETUTCDATE",
    "GET_BIT",
    "GET_FILESTREAM_TRANSACTION_CONTEXT",
    "GET_TRANSMISSION_STATUS",
    "GRAPH_ID_FROM_EDGE_ID",
    "GRAPH_ID_FROM_NODE_ID",
    "GREATEST",
    "GROUPING",
    "GROUPING_ID",
    "HASHBYTES",
    "HAS_DBACCESS",
    "HAS_PERMS_BY_NAME",
    "HOST_ID",
    "HOST_NAME",
    "IDENT_CURRENT",
    "IDENT_INCR",
    "IDENT_SEED",
    "INDEXKEY_PROPERTY",
    "INDEXPROPERTY",
    "INDEX_COL",
    "ISDATE",
    "ISJSON",
    "ISNULL",
    "ISNUMERIC",
    "IS_MEMBER",
    "IS_OBJECTSIGNED",
    "IS_ROLEMEMBER",
    "IS_SRVROLEMEMBER",
    "JSON_ARRAY",
    "JSON_MODIFY",
    "JSON_OBJECT",
    "JSON_PATH_EXISTS",
    "JSON_QUERY",
    "JSON_VALUE",
    "KEY_GUID",
    "KEY_ID",
    "KEY_NAME",
    "LAG",
    "LAST_VALUE",
    "LEAD",
    "LEAST",
    "LEFT_SHIFT",
    "LEN",
    "LOG",
    "LOG10",
    "LOGINPROPERTY",
    "LOWER",
    "LTRIM",
    "MAX",
    "MIN",
    "MIN_ACTIVE_ROWVERSION",
    "MONTH",
    "NCHAR",
    "NEWID",
    "NEWSEQUENTIALID",
    "NODE_ID_FROM_PARTS",
    "NTILE",
    "OBJECTPROPERTY",
    "OBJECTPROPERTYEX",
    "OBJECT_DEFINITION",
    "OBJECT_ID",
    "OBJECT_ID_FROM_EDGE_ID",
    "OBJECT_ID_FROM_NODE_ID",
    "OBJECT_NAME",
    "OBJECT_SCHEMA_NAME",
    "ORIGINAL_DB_NAME",
    "ORIGINAL_LOGIN",
    "PARSE",
    "PARSENAME",
    "PATINDEX",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "PERCENT_RANK",
    "PERMISSIONS",
    "PI",
    "PLATFORM",
    "POWER",
    "PUBLISHINGSERVERNAME",
    "PWDCOMPARE",
    "PWDENCRYPT",
    "QUOTENAME",
    "RADIANS",
    "RAND",
    "RANK",
    "REPLACE",
    "REPLICATE",
    "REVERSE",
    "RIGHT_SHIFT",
    "ROUND",
    "ROWCOUNT_BIG",
    "ROW_NUMBER",
    "RTRIM",
    "SCHEMA_ID",
    "SCHEMA_NAME",
    "SCOPE_IDENTITY",
    "SERVERPROPERTY",
    "SESSIONPROPERTY",
    "SESSION_CONTEXT",
    "SET_BIT",
    "SIGN",
    "SIGNBYASYMKEY",
    "SIGNBYCERT",
    "SIN",
    "SMALLDATETIMEFROMPARTS",
    "SOUNDEX",
    "SPACE",
    "SQL_VARIANT_PROPERTY",
    "SQRT",
    "SQUARE",
    "STATS_DATE",
    "STDEV",
    "STDEVP",
    "STR",
    "STRING_AGG",
    "STRING_ESCAPE",
    "STUFF",
    "SUBSTRING",
    "SUM",
    "SUSER_ID",
    "SUSER_NAME",
    "SUSER_SID",
    "SUSER_SNAME",
    "SWITCHOFFSET",
    "SYMKEYPROPERTY",
    "SYSDATETIME",
    "SYSDATETIMEOFFSET",
    "SYSUTCDATETIME",
    "TAN",
    "TERTIARY_WEIGHTS",
    "TEXTPTR",
    "TEXTVALID",
    "TIMEFROMPARTS",
    "TODATETIMEOFFSET",
    "TRANSLATE",
    "TRIGGER_NESTLEVEL",
    "TRIM",
    "TRY_CAST",
    "TRY_PARSE",
    "TYPEPROPERTY",
    "TYPE_ID",
    "TYPE_NAME",
    "UNICODE",
    "UPPER",
    "USER_ID",
    "USER_NAME",
    "VAR",
    "VARP",
    "VERIFYSIGNEDBYASYMKEY",
    "VERIFYSIGNEDBYCERT",
    "XACT_STATE",
    "XML_SCHEMA_NAMESPACE",
    "YEAR",
];

/// Reserved words that are also the names of built-in functions, called with parentheses.
pub(crate) const RESERVED_CALLS: [&str; 4] = ["COALESCE", "LEFT", "NULLIF", "RIGHT"];

/// Words that begin a statement the parser does not read; their statement becomes a gap.
pub(crate) const UNREAD_STATEMENTS: [&str; 32] = [
    "BACKUP",
    "BULK",
    "CHECKPOINT",
    "CLOSE",
    "DBCC",
    "DEALLOCATE",
    "DENY",
    "DISABLE",
    "ENABLE",
    "FETCH",
    "GET",
    "GOTO",
    "GRANT",
    "KILL",
    "MOVE",
    "OPEN",
    "READTEXT",
    "RECEIVE",
    "RECONFIGURE",
    "RESTORE",
    "REVERT",
    "REVOKE",
    "SEND",
    "SETUSER",
    "SHUTDOWN",
    "UPDATETEXT",
    "WAITFOR",
    "WRITETEXT",
    "ADD",
    "LOAD",
    "DUMP",
    "OPENROWSET",
];

/// The `SET` switch whose options are a gap.
pub(crate) const STATISTICS_SWITCH: &str = "STATISTICS";

/// The set operator that may take `ALL`.
pub(crate) const UNION_OPERATOR: &str = "UNION";

/// The compound assignment operators, `+=` to `^=`.
pub(crate) const COMPOUND_ASSIGNMENTS: [&str; 8] = ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^="];

/// The spelling of `INT` the tree records as an integer spelling.
pub(crate) const INTEGER_SPELLING: &str = "INTEGER";

/// The type that `NCHAR VARYING` spells as `NVARCHAR`.
pub(crate) const NCHAR_TYPE: &str = "NCHAR";

/// The type whose schema collection is a gap.
pub(crate) const XML_TYPE: &str = "XML";

/// The most parts a qualified name may have.
pub(crate) const MAX_NAME_PARTS: usize = 4;

/// The built-in read as its own conditional form.
pub(crate) const IIF_FUNCTION: &str = "IIF";

/// The fewest arguments `COALESCE` takes.
pub(crate) const MIN_COALESCE_ARGUMENTS: usize = 2;

/// The number of arguments `NULLIF` takes.
pub(crate) const NULLIF_ARGUMENTS: usize = 2;

/// The call whose first argument is its target type.
pub(crate) const TRY_CONVERT_FUNCTION: &str = "TRY_CONVERT";

/// Nesting deeper than this, in parsing or in the finished tree, is the gap `nesting_depth`.
pub(crate) const MAX_DEPTH: usize = 512;
/// The engine's error 10704 for a THROW without arguments outside a CATCH block.
pub(crate) const THROW_OUTSIDE_CATCH: &str =
    "To rethrow an error, a THROW statement must be used inside a CATCH block.";

/// The modes a `FOR XML` clause may name.
pub(crate) const XML_MODES: [&str; 4] = ["RAW", "AUTO", "EXPLICIT", "PATH"];
/// The `FOR XML` modes that take an element name, as in `PATH('row')`.
pub(crate) const NAMED_XML_MODES: [&str; 2] = ["RAW", "PATH"];
/// The modes a `FOR JSON` clause may name.
pub(crate) const JSON_MODES: [&str; 2] = ["AUTO", "PATH"];
/// The `FOR JSON` option that names a root element.
pub(crate) const JSON_ROOT: &str = "ROOT";
/// The `FOR JSON` option that leaves out the array brackets, which a root element cannot go with.
pub(crate) const JSON_WITHOUT_ARRAY_WRAPPER: &str = "WITHOUT_ARRAY_WRAPPER";
/// SQL Server's error 13620, for a `FOR JSON` clause with both ROOT and WITHOUT_ARRAY_WRAPPER.
pub(crate) const JSON_ROOT_WITHOUT_WRAPPER: &str = "ROOT option and WITHOUT_ARRAY_WRAPPER option cannot be used together in FOR JSON. Remove one of these options.";
/// SQL Server's error 198, for `FOR BROWSE` after a set operation.
pub(crate) const BROWSE_IN_SET_OPERATION: &str =
    "Browse mode is invalid for statements containing a UNION, INTERSECT or EXCEPT operator.";
/// SQL Server's error 176, for `FOR BROWSE` anywhere but after a statement's own query.
pub(crate) const BROWSE_NESTED: &str = "The FOR BROWSE clause is no longer supported in views.";
/// SQL Server's error 171, for `FOR BROWSE` after the query of an INSERT or a SELECT INTO.
pub(crate) const BROWSE_WITH_WRITE: &str =
    "Browse mode cannot be used with INSERT, SELECT INTO, or UPDATE statements.";

/// The table hints that are one word, besides `FORCESEEK` with its index and columns.
pub(crate) const TABLE_HINTS: [&str; 23] = [
    "NOLOCK",
    "READUNCOMMITTED",
    "UPDLOCK",
    "REPEATABLEREAD",
    "SERIALIZABLE",
    "READCOMMITTED",
    "TABLOCK",
    "TABLOCKX",
    "PAGLOCK",
    "ROWLOCK",
    "NOWAIT",
    "READPAST",
    "XLOCK",
    "SNAPSHOT",
    "NOEXPAND",
    "HOLDLOCK",
    "READCOMMITTEDLOCK",
    "FORCESEEK",
    "FORCESCAN",
    "KEEPIDENTITY",
    "KEEPDEFAULTS",
    "IGNORE_CONSTRAINTS",
    "IGNORE_TRIGGERS",
];
/// The isolation levels table hints name, the read-uncommitted one first; a table's hints name at most one (1047).
pub(crate) const ISOLATION_LEVELS: [&[&str]; 6] = [
    &["NOLOCK", "READUNCOMMITTED"],
    &["READCOMMITTED"],
    &["READCOMMITTEDLOCK"],
    &["REPEATABLEREAD"],
    &["SERIALIZABLE", "HOLDLOCK"],
    &["SNAPSHOT"],
];
/// The lock granularities table hints name; a table's hints name at most one, and none with read-uncommitted (1047).
pub(crate) const LOCK_GRANULARITIES: [&str; 4] = ["ROWLOCK", "PAGLOCK", "TABLOCK", "TABLOCKX"];
/// How the table-hint checks name a `FORCESEEK` that names its index and columns.
pub(crate) const PARAMETERIZED_FORCESEEK: &str = "FORCESEEK()";
/// The table hint that caps the cells a spatial index search reads, given once at most (1017).
pub(crate) const SPATIAL_WINDOW_MAX_CELLS: &str = "SPATIAL_WINDOW_MAX_CELLS";
/// SQL Server's error 1047, for one table's locking hints that cannot stand together.
pub(crate) const CONFLICTING_LOCKS: &str = "Conflicting locking hints specified.";
/// The hints SQL Server refuses on the target of an INSERT, UPDATE or DELETE, each with its error (1069, 10724, 10745, 1065).
pub(crate) const DML_TARGET_REFUSED_HINTS: [(&str, &str); 6] = [
    ("INDEX", INDEX_HINT_IN_TARGET),
    (
        "FORCESEEK",
        "The FORCESEEK hint is not allowed for target tables of INSERT, UPDATE, or DELETE statements.",
    ),
    (
        PARAMETERIZED_FORCESEEK,
        "The FORCESEEK hint is not allowed for target tables of INSERT, UPDATE, or DELETE statements.",
    ),
    (
        "FORCESCAN",
        "The FORCESCAN hint is not allowed for target tables of INSERT, UPDATE, or DELETE statements.",
    ),
    ("NOLOCK", DIRTY_READ_IN_TARGET),
    ("READUNCOMMITTED", DIRTY_READ_IN_TARGET),
];
/// The hints SQL Server refuses on the target of a MERGE (1065).
pub(crate) const MERGE_TARGET_REFUSED_HINTS: [(&str, &str); 2] = [
    ("NOLOCK", DIRTY_READ_IN_TARGET),
    ("READUNCOMMITTED", DIRTY_READ_IN_TARGET),
];
/// SQL Server's error 1069, for an index hint on the target of an INSERT, UPDATE or DELETE.
const INDEX_HINT_IN_TARGET: &str = "Index hints are only allowed in a FROM or OPTION clause.";
/// SQL Server's error 1065, for a read-uncommitted hint on the target of a write.
const DIRTY_READ_IN_TARGET: &str = "The NOLOCK and READUNCOMMITTED lock hints are not allowed for target tables of INSERT, UPDATE, DELETE or MERGE statements.";
