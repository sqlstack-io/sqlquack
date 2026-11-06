//! `stmt` (gram.y): which statements the parser reads, and the opaque gap for the rest.

use sqlquack_syntax::Expression;

use crate::parse::_helpers::cursor::{Parser, Parsing};
use crate::parse::_helpers::nodes::command;
use crate::parse::constants::{STATEMENT_STARTS, STATEMENT_STARTS_MORE};

impl Parser<'_> {
    /// One statement: a query or DML statement, or any other statement as an opaque gap.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
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
