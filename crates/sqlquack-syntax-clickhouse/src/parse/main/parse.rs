//! `parse`: every statement of a ClickHouse SQL input, or the error its parser reports.

use sqlquack_syntax::{Expression, ExpressionWalk, Parsed};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::constants::MAX_TREE_DEPTH;
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// Every statement of `sql`; a statement holding a construct the shared tree lacks is a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let mut parser: Parser<'_> = Parser::new(sql, scan(sql));
    let mut statements: Vec<Expression> = Vec::new();
    while parser.peek().is_some() {
        let statement: Expression = parser.top_level_statement()?;
        if statement.tree_depth() > MAX_TREE_DEPTH {
            return Err(parser.error_with("Maximum parse depth exceeded", false));
        }
        statements.push(statement);
    }
    match statements.is_empty() {
        true => Err(parser.empty_query()),
        false => Ok(Parsed {
            statements,
            gaps: parser.gaps,
        }),
    }
}
