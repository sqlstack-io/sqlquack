//! `parse_expressions`: the comma-separated expressions at the start of `tokens`, ignoring what follows them.

use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::parser::models::ParserConfig;
use crate::tokens::models::Dialect;
use crate::tokens::models::Token;
use crate::tokens::types::Result;

/// Leading comma-separated expressions of `tokens`, unguarded: pass only tokens of input already guarded.
pub fn parse_expressions(tokens: Vec<Token>, dialect: Dialect) -> Result<Vec<Expression>> {
    let config = ParserConfig {
        dialect: Some(dialect.dialect_type()),
        faithful_calls: true,
        ..Default::default()
    };
    Parser::with_config(tokens, config).parse_expression_list()
}
