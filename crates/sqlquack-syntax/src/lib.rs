//! SQLQuack's tokenizer, parser and syntax tree for Snowflake and DuckDB.

mod ast_children;
mod dialects;
pub mod error;
pub mod expressions;
mod function_registry;
mod guard;
mod lossless;
mod modes;
mod parser;
pub mod tokens;
mod traversal;

#[cfg(test)]
mod ported;

pub(crate) use dialects::DialectType;
pub use error::{Error, Result};
pub use expressions::{DataType, Expression};
pub use guard::{ComplexityGuardOptions, ParseOptions};
pub use lossless::{LosslessToken, TokenKind, TokenizeFailure};
pub use modes::{parse_strict, parse_tolerant, StrictFailure, TolerantParse, TolerantStatement};
pub use traversal::ExpressionWalk;

/// A dialect SQLQuack parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// Snowflake SQL.
    Snowflake,
    /// DuckDB SQL.
    DuckDB,
}

impl Dialect {
    pub(crate) fn dialect_type(self) -> DialectType {
        match self {
            Dialect::Snowflake => DialectType::Snowflake,
            Dialect::DuckDB => DialectType::DuckDB,
        }
    }
}

/// How the parser treats function calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallMode {
    /// Every Snowflake call keeps its name and arguments as written; SQLQuack's mode.
    Faithful,
    /// As [`CallMode::Faithful`], recording each faithful call's source span (for the exact check).
    FaithfulWithSpans,
    /// polyglot's lowering of calls into cross-dialect nodes (for the exact check only).
    Lowered,
}

/// Parses SQL, possibly several statements, keeping calls as written, under complexity limits.
pub fn parse_statements(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>> {
    parse_statements_with(sql, dialect, options, CallMode::Faithful)
}

/// Parses SQL as [`parse_statements`] does, treating calls as `mode` says.
pub fn parse_statements_with(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    mode: CallMode,
) -> Result<Vec<Expression>> {
    let flags = match mode {
        CallMode::Faithful => (true, false),
        CallMode::FaithfulWithSpans => (true, true),
        CallMode::Lowered => (false, false),
    };
    dialects::Dialect::get(dialect.dialect_type()).parse_with_mode(sql, options, flags)
}

/// Splits SQL into tokens covering every byte: significant tokens, whitespace and comments.
pub fn tokenize(
    sql: &str,
    dialect: Dialect,
) -> std::result::Result<Vec<LosslessToken>, TokenizeFailure> {
    lossless::tokenize_lossless(sql, dialect)
}

#[cfg(test)]
pub(crate) use ported::compat::{parse, parse_one, Parser};
