//! Dialects and source positions of parsed SQL.

use serde::Serialize;
use sqlquack_syntax::Dialect;
/// A dialect SQLQuack converts from. Every entry point takes it as a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDialect {
    /// Snowflake SQL.
    Snowflake,
}

impl SourceDialect {
    /// The parser and printer dialect for this source.
    pub fn syntax(self) -> Dialect {
        match self {
            Self::Snowflake => Dialect::Snowflake,
        }
    }

    /// The lower-case name used on the command line and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Snowflake => "snowflake",
        }
    }

    /// The source dialect with this command-line name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "snowflake" => Some(Self::Snowflake),
            _ => None,
        }
    }
}

/// A position in source SQL, in Unicode characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct SourceLocation {
    /// Character offset of the first character.
    pub start: usize,
    /// Character offset one past the last character.
    pub end: usize,
    /// One-based line.
    pub line: usize,
    /// One-based column.
    pub column: usize,
}
