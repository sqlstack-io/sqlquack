//! The root API of polyglot the ported tests call, over the copied modules.

pub(crate) use crate::dialects::{Dialect, DialectType};
pub(crate) use crate::error::{Error, Result};
pub(crate) use crate::expressions::{DataType, Expression};
pub(crate) use crate::guard::{ComplexityGuardOptions, ParseOptions};
pub(crate) use crate::parser::Parser;
pub(crate) use crate::tokens::{Token, TokenType, Tokenizer};
pub(crate) use crate::traversal::ExpressionWalk;

pub(crate) fn parse(sql: &str, dialect: DialectType) -> Result<Vec<Expression>> {
    Dialect::get(dialect).parse(sql)
}

pub(crate) fn parse_with_options(
    sql: &str,
    dialect: DialectType,
    options: &ParseOptions,
) -> Result<Vec<Expression>> {
    Dialect::get(dialect).parse_with_options(sql, options)
}

pub(crate) fn parse_one(sql: &str, dialect: DialectType) -> Result<Expression> {
    parse_one_with_options(sql, dialect, &ParseOptions::default())
}

pub(crate) fn parse_one_with_options(
    sql: &str,
    dialect: DialectType,
    options: &ParseOptions,
) -> Result<Expression> {
    let mut expressions = parse_with_options(sql, dialect, options)?;
    if expressions.len() != 1 {
        return Err(Error::parse(
            format!("Expected 1 statement, found {}", expressions.len()),
            0,
            0,
            0,
            0,
        ));
    }
    Ok(expressions.remove(0))
}

pub(crate) fn parse_data_type(sql: &str, dialect: DialectType) -> Result<DataType> {
    Dialect::get(dialect).parse_data_type(sql)
}

pub(crate) fn parse_data_type_with_options(
    sql: &str,
    dialect: DialectType,
    options: &ParseOptions,
) -> Result<DataType> {
    Dialect::get(dialect).parse_data_type_with_options(sql, options)
}
