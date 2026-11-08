//! `parse`: every statement of a Databricks SQL input, or the error its parser reports.

use sqlquack_syntax::{Expression, ExpressionWalk, Parsed};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::MAX_TREE_DEPTH;
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// Every statement of `sql`; a statement holding a construct the shared tree lacks is a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let mut parser: Parser<'_> = Parser::new(sql, scan(sql));
    let mut statements: Vec<Expression> = Vec::new();
    loop {
        while parser.eat_char(';') {}
        if parser.peek().is_none() {
            if statements.is_empty() {
                return Err(parser.error_with(
                    "Syntax error, unexpected empty statement",
                    sql.len(),
                    true,
                ));
            }
            return Ok(Parsed {
                statements,
                gaps: parser.gaps,
            });
        }
        let first: usize = parser.at;
        parser.opaque = false;
        let parsed: Result<Expression, ParseError> = parser.statement();
        if let Some(too_deep) = parser.too_deep.take() {
            return Err(too_deep);
        }
        let statement: Expression = parsed?;
        if !(parser.peek().is_none() || parser.is_char(';')) {
            return Err(parser.error());
        }
        let start: usize = parser.tokens[first].start;
        if statement.tree_depth() > MAX_TREE_DEPTH {
            return Err(parser.error_with("the statement nests too deeply", start, false));
        }
        let end: usize = parser.tokens[parser.at - 1].end;
        statements.push(match parser.opaque {
            true => command(&sql[start..end]),
            false => statement,
        });
    }
}
