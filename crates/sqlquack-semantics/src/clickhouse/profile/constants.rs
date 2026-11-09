//! The values ClickHouse's profile and lineage rules read.

/// What ClickHouse makes of TIMESTAMP, an alias of `DateTime`.
pub(crate) const CLICKHOUSE_TIMESTAMP_TYPE: &str = "DateTime";
/// ClickHouse 26.1's type families matched with case, as `system.data_type_families` lists them.
pub(crate) const CLICKHOUSE_EXACT_FAMILIES: [&str; 20] = [
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "Int128",
    "Int256",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "UInt128",
    "UInt256",
    "Float32",
    "Float64",
    "String",
    "UUID",
    "Nothing",
    "FixedString",
    "IPv4",
    "IPv6",
];
/// ClickHouse 26.1's type families matched without regard to case.
pub(crate) const CLICKHOUSE_CASELESS_FAMILIES: [&str; 10] = [
    "Decimal",
    "Decimal32",
    "Decimal64",
    "Decimal128",
    "Decimal256",
    "Date",
    "Date32",
    "DateTime",
    "DateTime64",
    "Bool",
];
/// ClickHouse 26.1's type aliases, upper case, each with its family; all match without case.
pub(crate) const CLICKHOUSE_ALIASES: [(&str, &str); 69] = [
    ("BIGINT", "Int64"),
    ("BIGINT SIGNED", "Int64"),
    ("BIGINT UNSIGNED", "UInt64"),
    ("BINARY", "FixedString"),
    ("BINARY LARGE OBJECT", "String"),
    ("BINARY VARYING", "String"),
    ("BIT", "UInt64"),
    ("BLOB", "String"),
    ("BOOL", "Bool"),
    ("BOOLEAN", "Bool"),
    ("BYTE", "Int8"),
    ("BYTEA", "String"),
    ("CHAR", "String"),
    ("CHAR LARGE OBJECT", "String"),
    ("CHAR VARYING", "String"),
    ("CHARACTER", "String"),
    ("CHARACTER LARGE OBJECT", "String"),
    ("CHARACTER VARYING", "String"),
    ("CLOB", "String"),
    ("DEC", "Decimal"),
    ("DOUBLE", "Float64"),
    ("DOUBLE PRECISION", "Float64"),
    ("FIXED", "Decimal"),
    ("FLOAT", "Float32"),
    ("INT", "Int32"),
    ("INT SIGNED", "Int32"),
    ("INT UNSIGNED", "UInt32"),
    ("INT1", "Int8"),
    ("INT1 SIGNED", "Int8"),
    ("INT1 UNSIGNED", "UInt8"),
    ("INTEGER", "Int32"),
    ("INTEGER SIGNED", "Int32"),
    ("INTEGER UNSIGNED", "UInt32"),
    ("LONGBLOB", "String"),
    ("LONGTEXT", "String"),
    ("MEDIUMBLOB", "String"),
    ("MEDIUMINT", "Int32"),
    ("MEDIUMINT SIGNED", "Int32"),
    ("MEDIUMINT UNSIGNED", "UInt32"),
    ("MEDIUMTEXT", "String"),
    ("NATIONAL CHAR", "String"),
    ("NATIONAL CHAR VARYING", "String"),
    ("NATIONAL CHARACTER", "String"),
    ("NATIONAL CHARACTER LARGE OBJECT", "String"),
    ("NATIONAL CHARACTER VARYING", "String"),
    ("NCHAR", "String"),
    ("NCHAR LARGE OBJECT", "String"),
    ("NCHAR VARYING", "String"),
    ("NUMERIC", "Decimal"),
    ("NVARCHAR", "String"),
    ("REAL", "Float32"),
    ("SET", "UInt64"),
    ("SIGNED", "Int64"),
    ("SINGLE", "Float32"),
    ("SMALLINT", "Int16"),
    ("SMALLINT SIGNED", "Int16"),
    ("SMALLINT UNSIGNED", "UInt16"),
    ("TEXT", "String"),
    ("TIMESTAMP", "DateTime"),
    ("TINYBLOB", "String"),
    ("TINYINT", "Int8"),
    ("TINYINT SIGNED", "Int8"),
    ("TINYINT UNSIGNED", "UInt8"),
    ("TINYTEXT", "String"),
    ("UNSIGNED", "UInt64"),
    ("VARBINARY", "String"),
    ("VARCHAR", "String"),
    ("VARCHAR2", "String"),
    ("YEAR", "UInt16"),
];
/// The types that wrap one other type, matched with case.
pub(crate) const CLICKHOUSE_WRAPPERS: [&str; 3] = ["Nullable", "LowCardinality", "Array"];
/// The integer and float families, whose numeric parameters ClickHouse accepts and ignores.
pub(crate) const CLICKHOUSE_NUMBER_FAMILIES: [&str; 3] = ["Int", "UInt", "Float"];
/// What ClickHouse makes of `Decimal` written without a precision.
pub(crate) const CLICKHOUSE_DECIMAL_DEFAULT: &str = "Decimal(10, 0)";
/// The fixed-precision decimal families, each with the precision it stands for.
pub(crate) const CLICKHOUSE_DECIMAL_WIDTHS: [(&str, u32); 4] = [
    ("Decimal32", 9),
    ("Decimal64", 18),
    ("Decimal128", 38),
    ("Decimal256", 76),
];
/// What ClickHouse's comparisons, logical operators and predicates return.
pub(crate) const CLICKHOUSE_BOOLEAN_RESULT: &str = "UInt8";
/// What ClickHouse's counts and ranking functions return.
pub(crate) const CLICKHOUSE_COUNT_TYPE: &str = "UInt64";
/// What ClickHouse's `now()` and one-argument `toDateTime` return.
pub(crate) const CLICKHOUSE_DATETIME_TYPE: &str = "DateTime";
/// How ClickHouse spells every Decimal type's name.
pub(crate) const CLICKHOUSE_DECIMAL_PREFIX: &str = "Decimal(";
/// Built-ins whose ClickHouse 26.1 result type does not depend on their arguments' types.
pub(crate) const CLICKHOUSE_FIXED_RESULTS: [(&str, &str); 32] = [
    ("toDate", "Date"),
    ("today", "Date"),
    ("toString", "String"),
    ("toInt8", "Int8"),
    ("toInt16", "Int16"),
    ("toInt32", "Int32"),
    ("toInt64", "Int64"),
    ("toUInt8", "UInt8"),
    ("toUInt16", "UInt16"),
    ("toUInt32", "UInt32"),
    ("toUInt64", "UInt64"),
    ("toFloat32", "Float32"),
    ("toFloat64", "Float64"),
    ("toYYYYMM", "UInt32"),
    ("toYYYYMMDD", "UInt32"),
    ("toYear", "UInt16"),
    ("toMonth", "UInt8"),
    ("toDayOfMonth", "UInt8"),
    ("toHour", "UInt8"),
    ("length", "UInt64"),
    ("lengthUTF8", "UInt64"),
    ("concat", "String"),
    ("substring", "String"),
    ("trim", "String"),
    ("replaceAll", "String"),
    ("formatDateTime", "String"),
    ("toStartOfMonth", "Date"),
    ("toStartOfQuarter", "Date"),
    ("toStartOfYear", "Date"),
    ("toStartOfWeek", "Date"),
    ("toMonday", "Date"),
    ("dateDiff", "Int64"),
];
/// How ClickHouse spells every FixedString type's name.
pub(crate) const CLICKHOUSE_FIXED_STRING_PREFIX: &str = "FixedString(";
/// The prefix of a hexadecimal number literal.
pub(crate) const CLICKHOUSE_HEX_PREFIX: &str = "0x";
/// The wrappers that do not change a value's type for arithmetic and functions.
pub(crate) const CLICKHOUSE_NULLABLE_WRAPPERS: [&str; 2] = ["Nullable", "LowCardinality"];
/// ClickHouse's text type.
pub(crate) const CLICKHOUSE_STRING_TYPE: &str = "String";
