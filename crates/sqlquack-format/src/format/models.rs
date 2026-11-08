//! Data models of format.

use std::collections::HashMap;

use sqlquack_syntax::{Dialect, ParseOptions};

use crate::format::constants::DEFAULT_INDENT;

/// The SQL dialects the formatter reads; SQL Server through its own Transact-SQL scanner and parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatDialect {
    Snowflake,
    DuckDB,
    PostgreSQL,
    BigQuery,
    Databricks,
    SqlServer,
}

impl FormatDialect {
    /// The shared tokenizer's dialect, for the dialects it reads.
    pub(crate) fn shared(self) -> Option<Dialect> {
        match self {
            Self::Snowflake => Some(Dialect::Snowflake),
            Self::DuckDB => Some(Dialect::DuckDB),
            Self::PostgreSQL => Some(Dialect::PostgreSQL),
            Self::BigQuery | Self::Databricks | Self::SqlServer => None,
        }
    }
}

/// How SQL is laid out.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// The dialect the SQL is tokenized and parsed in.
    pub dialect: FormatDialect,
    /// Lines longer than this are re-broken; absent, only the parse-tree layout applies.
    pub line_width: Option<usize>,
    /// Spaces per indent step.
    pub indent: usize,
    /// Printed widths of tokens that stand in for longer text, keyed by upper-case token text.
    pub token_widths: HashMap<String, usize>,
    /// Complexity guards for the validating parse.
    pub parse: ParseOptions,
}

impl FormatOptions {
    /// Default layout for `dialect`: no line width, two-space indent.
    pub fn new(dialect: FormatDialect) -> Self {
        Self {
            dialect,
            line_width: None,
            indent: DEFAULT_INDENT,
            token_widths: HashMap::new(),
            parse: ParseOptions::default(),
        }
    }
}

/// Formatted SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    /// The laid-out SQL; the input when nothing changed.
    pub sql: String,
    /// Whether `sql` differs from the input.
    pub changed: bool,
}

/// Why SQL was left as it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatRefusal {
    /// The reason, for display.
    pub reason: String,
}
