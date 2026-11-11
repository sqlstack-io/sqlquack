//! `parse_expressions`: the comma-separated expressions at the start of `tokens`, ignoring what follows them.

use crate::parser::models::Parser;
use crate::parser::models::ParserConfig;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax::tokens::models::Token;
use sqlquack_syntax::tokens::types::Result;

use crate::parser::_helpers::support::dialects::dialect_type;

/// Leading comma-separated expressions of `tokens`, unguarded: pass only tokens of input already guarded.
pub fn parse_expressions(tokens: Vec<Token>, dialect: Dialect) -> Result<Vec<Expression>> {
    let config = ParserConfig {
        dialect: Some(dialect_type(dialect)?),
        ..Default::default()
    };
    Parser::with_config(tokens, config).parse_expression_list()
}
