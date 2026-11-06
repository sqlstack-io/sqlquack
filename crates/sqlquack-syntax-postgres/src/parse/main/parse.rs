//! `parse`: every statement of a PostgreSQL 18 input, or the error PostgreSQL's parser reports.

use sqlquack_syntax::Expression;

use crate::parse::_helpers::cursor::{Parser, Parsing};
use crate::parse::_helpers::nodes::command;
use crate::parse::constants::{STATEMENT_STARTS, STATEMENT_STARTS_MORE};
use crate::parse::models::{ParseError, Parsed};
use crate::scan::main::scan::scan;
use crate::scan::models::TokenKind;

impl Parser<'_> {
    /// One statement: a query or DML statement, or any other statement as an opaque gap.
    fn statement(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return self.preparable_statement();
        }
        match self.keyword_at(0) {
            Some(
                "select" | "values" | "table" | "with" | "insert" | "update" | "delete" | "merge",
            ) => self.preparable_statement(),
            Some(word)
                if STATEMENT_STARTS.contains(&word) || STATEMENT_STARTS_MORE.contains(&word) =>
            {
                let start: usize = self.offset();
                self.skip_statement_rest()?;
                if self.is_char(')') {
                    return Err(self.error());
                }
                self.gap(word, start);
                Ok(command(""))
            }
            _ => Err(self.error()),
        }
    }
}

/// Every statement of `sql`; a statement holding a construct the shared tree lacks is a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    let mut parser: Parser<'_> = Parser::new(sql, scan(sql));
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
        debug_assert!(!matches!(
            parser.tokens.get(parser.at).map(|token| token.kind),
            Some(TokenKind::Fault(_))
        ));
    }
}
