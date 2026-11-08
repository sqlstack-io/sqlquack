//! The implemented dialect profiles.

use crate::dialects::models::{
    BigQuery, ClickHouse, Databricks, DuckDb, Postgres, Profiles, Snowflake,
    SnowflakeQuotedIgnoringCase, SqlServer, Sqlite,
};

/// The Snowflake profile.
pub const SNOWFLAKE: Snowflake = Snowflake;

/// The Snowflake profile under QUOTED_IDENTIFIERS_IGNORE_CASE.
pub const SNOWFLAKE_QUOTED_IGNORING_CASE: SnowflakeQuotedIgnoringCase = SnowflakeQuotedIgnoringCase;

/// The DuckDB profile.
pub const DUCKDB: DuckDb = DuckDb;

/// The PostgreSQL profile.
pub const POSTGRES: Postgres = Postgres;

/// The ClickHouse profile.
pub const CLICKHOUSE: ClickHouse = ClickHouse;

/// The BigQuery profile.
pub const BIGQUERY: BigQuery = BigQuery;

/// The SQLite profile.
pub const SQLITE: Sqlite = Sqlite;

/// The SQL Server profile, under the server's default case-insensitive collation.
pub const SQL_SERVER: SqlServer = SqlServer;

/// The Databricks SQL profile.
pub const DATABRICKS: Databricks = Databricks;

/// Snowflake SQL resolved so its names stay apart in DuckDB, as conversion needs.
pub const SNOWFLAKE_TO_DUCKDB: Profiles = Profiles {
    source: &SNOWFLAKE,
    target: &DUCKDB,
};
/// The length of Snowflake text declared without one.
pub(crate) const SNOWFLAKE_LONGEST_VARCHAR: u32 = 16_777_216;
/// What ClickHouse makes of TIMESTAMP, an alias of `DateTime`.
pub(crate) const CLICKHOUSE_TIMESTAMP_TYPE: &str = "DateTime";
/// What Snowflake's TIMESTAMP is under the default TIMESTAMP_TYPE_MAPPING.
pub(crate) const SNOWFLAKE_TIMESTAMP_TYPE: &str = "TIMESTAMP_NTZ";
/// Databricks' fixed-point type, whose precision and scale default when omitted.
pub(crate) const DATABRICKS_DECIMAL: &str = "DECIMAL";
/// The precision and scale of a Databricks DECIMAL declared without them.
pub(crate) const DATABRICKS_DEFAULT_DECIMAL: (u32, u32) = (10, 0);

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

/// What SQL Server stores a timestamp written without its kind as; its TIMESTAMP is rowversion.
pub(crate) const SQLSERVER_TIMESTAMP_TYPE: &str = "DATETIME2";
