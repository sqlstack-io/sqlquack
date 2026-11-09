//! `parse`: every statement of a DuckDB SQL input, or the error its parser reports.

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::MAX_TREE_DEPTH;
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;
use crate::scan::models::{Token, TokenKind};
use sqlquack_syntax::Parsed;

/// Every statement of `sql`, a construct the tree lacks kept as a `Command`; faults come first.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let tokens: Vec<Token> = scan(sql);
    if let Some(fault) = tokens
        .last()
        .filter(|token| token.kind == TokenKind::Fault(true))
    {
        return Err(ParseError {
            message: fault.value.clone(),
            position: fault.start,
            syntax: true,
        });
    }
    let mut parser: Parser<'_> = Parser::new(sql, tokens);
    let mut statements: Vec<Expression> = Vec::new();
    loop {
        while parser.eat_char(';') {}
        if parser.peek().is_none() {
            return Ok(Parsed {
                statements,
                gaps: parser.gaps,
            });
        }
        let first: usize = parser.at;
        parser.opaque = false;
        parser.refusal = None;
        let parsed: Result<Expression, ParseError> = parser.statement();
        if let Some(too_deep) = parser.too_deep.take() {
            return Err(too_deep);
        }
        let statement: Expression = parsed?;
        if !(parser.peek().is_none() || parser.is_char(';')) {
            return Err(parser.error());
        }
        if let Some(refusal) = parser.refusal.take() {
            return Err(refusal);
        }
        if mixes_parameters(&parser.tokens[first..parser.at]) {
            return Err(parser.error_with(
                "Mixing named and positional parameters is not supported yet",
                parser.tokens[first].start,
                false,
            ));
        }
        let start: usize = parser.tokens[first].start;
        let end: usize = parser.tokens[parser.at - 1].end;
        if statement.tree_depth() > MAX_TREE_DEPTH {
            return Err(parser.error_with("the statement nests too deeply", start, false));
        }
        statements.push(match parser.opaque {
            true => command(&sql[start..end]),
            false => statement,
        });
    }
}

/// Whether a statement's tokens mix a named parameter with a positional one, as DuckDB refuses.
fn mixes_parameters(tokens: &[Token]) -> bool {
    let mut named: bool = false;
    let mut positional: bool = false;
    for (index, token) in tokens.iter().enumerate() {
        let next_is_number = || {
            tokens
                .get(index + 1)
                .is_some_and(|next| next.kind == TokenKind::Number)
        };
        match token.kind {
            TokenKind::Char('?') => positional = true,
            TokenKind::Char('$') if next_is_number() => positional = true,
            TokenKind::Char('$') => named = true,
            _ => {}
        }
    }
    named && positional
}
