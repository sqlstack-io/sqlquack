//! `parse`: every statement of a Transact-SQL batch, or the error its parser reports.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::statements::models::Command;

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::errors::ParseError;
use crate::parse::models::{Parsed, StatementRange};
use crate::scan::main::scan::scan;
use crate::scan::models::{Token, TokenKind};

/// Every statement of the batch `sql`, a construct the tree lacks ending it as one `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let tokens: Vec<Token> = scan(sql);
    if let Some(fault) = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Fault(_)))
    {
        return Err(ParseError {
            message: fault.value.clone(),
            position: fault.start,
            syntax: true,
        });
    }
    let mut parser: Parser<'_> = Parser::new(sql, tokens);
    match parser.batch() {
        Ok(statements) => Ok(Parsed {
            statements,
            gaps: Vec::new(),
            ranges: parser.ranges,
        }),
        Err(_) if parser.gapped() => {
            let start: usize = parser.gaps.first().map_or(0, |gap| gap.start);
            Ok(Parsed {
                statements: vec![Expression::Command(Box::new(Command {
                    this: sql[start..].to_owned(),
                }))],
                gaps: parser.gaps,
                ranges: vec![StatementRange {
                    start,
                    end: sql.len(),
                    depth: 0,
                }],
            })
        }
        Err(error) => Err(error),
    }
}
