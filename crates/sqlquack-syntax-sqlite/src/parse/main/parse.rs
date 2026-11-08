//! `parse`: every statement of an SQLite input, or the error SQLite's parser reports.

use sqlquack_syntax::{Expression, ExpressionWalk, Parsed};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::{MAX_TREE_DEPTH, TOO_DEEP};
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// Every statement of `sql`, each tree at most `MAX_TREE_DEPTH` deep; a gap becomes a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let mut parser: Parser<'_> = Parser::new(sql, scan(sql));
    let mut statements: Vec<Expression> = Vec::new();
    loop {
        while parser.eat_op("SEMI") {}
        if parser.peek().is_none() {
            return Ok(Parsed {
                statements,
                gaps: parser.gaps,
            });
        }
        let first: usize = parser.at;
        let start: usize = parser.offset();
        parser.opaque = false;
        let statement: Result<Expression, ParseError> =
            parser
                .statement()
                .and_then(|statement| match parser.at_end() {
                    true => Ok(statement),
                    false => Err(parser.error()),
                });
        match (statement, parser.pending.take()) {
            (Ok(_), Some((pending, _))) => return Err(pending),
            (Err(error), Some((pending, next))) => {
                return Err(match parser.at > next {
                    true => pending,
                    false => error,
                });
            }
            (Err(error), None) => return Err(error),
            (Ok(statement), None) if statement.tree_depth() > MAX_TREE_DEPTH => {
                return Err(parser.error_with(TOO_DEEP, start, false));
            }
            (Ok(statement), None) => statements.push(match parser.opaque {
                true => command(parser.text_from(first)),
                false => statement,
            }),
        }
    }
}
