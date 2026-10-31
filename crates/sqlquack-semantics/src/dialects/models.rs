//! The dialect profiles.

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

/// The dialect SQL is written in, and the one it is resolved for, whose names may clash.
#[derive(Clone, Copy, Debug)]
pub struct Profiles {
    /// The dialect the SQL is written in, which decides what a name means.
    pub source: &'static dyn DialectProfile,
    /// The dialect the names must also stay apart in.
    pub target: &'static dyn DialectProfile,
}
