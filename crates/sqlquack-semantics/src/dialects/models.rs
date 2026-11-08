//! The dialect profiles.

use crate::dialects::_helpers::snowflake_types::same_snowflake_type;
use crate::dialects::constants::{
    CLICKHOUSE_TIMESTAMP_TYPE, SNOWFLAKE_TIMESTAMP_TYPE, SQLSERVER_TIMESTAMP_TYPE,
};
use crate::dialects::main::clickhouse_type::clickhouse_type;
use crate::dialects::types::DialectProfile;

/// Snowflake: unquoted identifiers resolve to upper case, and resolved names compare exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snowflake;

impl DialectProfile for Snowflake {
    fn name(&self) -> &'static str {
        "Snowflake"
    }

    fn resolved_identifier(&self, name: &str, quoted: bool) -> String {
        match quoted {
            true => name.to_owned(),
            false => name.to_ascii_uppercase(),
        }
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        same_snowflake_type(left, right)
    }

    fn timestamp_type(&self) -> &'static str {
        SNOWFLAKE_TIMESTAMP_TYPE
    }
}

/// Snowflake with QUOTED_IDENTIFIERS_IGNORE_CASE: quoted identifiers resolve to upper case too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnowflakeQuotedIgnoringCase;

impl DialectProfile for SnowflakeQuotedIgnoringCase {
    fn name(&self) -> &'static str {
        "Snowflake"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_ascii_uppercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        same_snowflake_type(left, right)
    }

    fn timestamp_type(&self) -> &'static str {
        SNOWFLAKE_TIMESTAMP_TYPE
    }
}

/// DuckDB: identifiers keep their case as written, and names compare without regard to ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DuckDb;

impl DialectProfile for DuckDb {
    fn name(&self) -> &'static str {
        "DuckDB"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// PostgreSQL: unquoted identifiers resolve to lower case, and resolved names compare exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Postgres;

impl DialectProfile for Postgres {
    fn name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn resolved_identifier(&self, name: &str, quoted: bool) -> String {
        match quoted {
            true => name.to_owned(),
            false => name.to_ascii_lowercase(),
        }
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }
}

/// SQLite: identifiers keep their case as written, and names compare without regard to ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sqlite;

impl DialectProfile for Sqlite {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// ClickHouse: names keep their case and compare exactly; types compare as ClickHouse names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClickHouse;

impl DialectProfile for ClickHouse {
    fn name(&self) -> &'static str {
        "ClickHouse"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn timestamp_type(&self) -> &'static str {
        CLICKHOUSE_TIMESTAMP_TYPE
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        match (clickhouse_type(left), clickhouse_type(right)) {
            (Some(left), Some(right)) => left == right,
            _ => left == right,
        }
    }
}

/// BigQuery: columns, aliases, CTEs and relations match whatever their ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BigQuery;

impl DialectProfile for BigQuery {
    fn name(&self) -> &'static str {
        "BigQuery"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_ascii_lowercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// SQL Server under its default collation: names resolve and compare without regard to case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqlServer;

impl DialectProfile for SqlServer {
    fn name(&self) -> &'static str {
        "SQL Server"
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_lowercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.to_lowercase() == right.to_lowercase()
    }

    fn timestamp_type(&self) -> &'static str {
        SQLSERVER_TIMESTAMP_TYPE
    }
}

/// The dialect SQL is written in, and the one it is resolved for, whose names may clash.
#[derive(Clone, Copy, Debug)]
pub struct Profiles {
    /// The dialect the SQL is written in, which decides what a name means.
    pub source: &'static dyn DialectProfile,
    /// The dialect the names must also stay apart in.
    pub target: &'static dyn DialectProfile,
}
