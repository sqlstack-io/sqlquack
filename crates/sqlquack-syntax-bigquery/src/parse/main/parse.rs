//! `parse`: every statement of a GoogleSQL input, or the error its parser reports.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::Parsed;

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::{MAX_TREE_DEPTH, TOO_DEEP_MESSAGE, TREE_DROP_FRAME};
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// Every statement of `sql`; a statement holding a construct the shared tree lacks is a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let mut parser: Parser<'_> = Parser::new(sql, scan(sql));
    let mut statements: Vec<Expression> = Vec::new();
    loop {
        if !statements.is_empty() {
            parser.eat_char(';');
        }
        if parser.peek().is_none() {
            if statements.is_empty() {
                return Err(parser.error_with(
                    "Syntax error: Unexpected end of statement",
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
        let statement: Expression = within_tree_depth(statement, start)?;
        let end: usize = parser.tokens[parser.at - 1].end;
        statements.push(match parser.opaque {
            true => command(&sql[start..end]),
            false => statement,
        });
    }
}

/// `statement`, or the depth refusal at `start` when its tree nests past the bound.
fn within_tree_depth(statement: Expression, start: usize) -> Result<Expression, ParseError> {
    let depth: usize = statement.tree_depth();
    match depth > MAX_TREE_DEPTH {
        false => Ok(statement),
        true => {
            stacker::grow(depth.saturating_mul(TREE_DROP_FRAME), move || {
                drop(statement)
            });
            Err(ParseError {
                message: TOO_DEEP_MESSAGE.to_owned(),
                position: start,
                syntax: false,
            })
        }
    }
}
