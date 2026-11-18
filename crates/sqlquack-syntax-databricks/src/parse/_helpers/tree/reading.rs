//! Reading every statement of an input, with forms not yet reviewed for layout as trees or gaps.

use sqlquack_syntax::{Expression, ExpressionWalk, Gap, Parsed};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::MAX_TREE_DEPTH;
use crate::parse::errors::{ParseError, StrictFailure};
use crate::scan::main::scan::scan;
use crate::scan::main::uncovered::uncovered;

/// How `parse_as` keeps a form of `TREED_FORMS`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Reading {
    /// As its tree.
    Trees,
    /// As a gap, for a reader that lays the statement out.
    Reviewed,
}

/// Every statement of `sql`, read as `reading` says.
pub(crate) fn parse_as(sql: &str, reading: Reading) -> Result<Parsed, ParseError> {
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
        let treed: usize = parser.treed.len();
        parser.statement_treed = treed;
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
            return Err(ParseError {
                guard: true,
                ..parser.error_with("the statement nests too deeply", start, false)
            });
        }
        let end: usize = parser.tokens[parser.at - 1].end;
        if reading == Reading::Reviewed && parser.treed.len() > treed {
            let unreviewed: Vec<Gap> = parser.treed.split_off(treed);
            parser.gaps.extend(unreviewed);
            parser.opaque = true;
        }
        statements.push(match parser.opaque {
            true => command(&sql[start..end]),
            false => statement,
        });
    }
}

/// `parse_as`'s statements, refused on any parse error, syntax or not, or uncovered byte.
pub(crate) fn strict(sql: &str, reading: Reading) -> Result<Parsed, StrictFailure> {
    let parsed: Parsed = parse_as(sql, reading).map_err(StrictFailure::Parse)?;
    match uncovered(sql) {
        Some(position) => Err(StrictFailure::Uncovered { position }),
        None => Ok(parsed),
    }
}
