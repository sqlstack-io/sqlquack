//! `parse`: every statement of a GoogleSQL input, or the error its parser reports.

use sqlquack_syntax::Expression;
use sqlquack_syntax::Parsed;

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
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
        let statement: Expression = parser.statement()?;
        if !(parser.peek().is_none() || parser.is_char(';')) {
            return Err(parser.error());
        }
        let start: usize = parser.tokens[first].start;
        let end: usize = parser.tokens[parser.at - 1].end;
        statements.push(match parser.opaque {
            true => command(&sql[start..end]),
            false => statement,
        });
    }
}
