//! `cmd` (parse.y): a query, or another statement kept whole as a gap.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::With;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::command;
use crate::parse::constants::{STATEMENT_KEYWORDS, WITH_STATEMENTS};
use crate::scan::models::TokenKind;

/// The gap's name for a statement keyword: the keyword in lower case.
fn gap_name(keyword: &str) -> &'static str {
    match keyword {
        "EXPLAIN" => "explain",
        "BEGIN" => "begin",
        "COMMIT" => "commit",
        "END" => "end",
        "ROLLBACK" => "rollback",
        "SAVEPOINT" => "savepoint",
        "RELEASE" => "release",
        "CREATE" => "create",
        "DROP" => "drop",
        "INSERT" => "insert",
        "REPLACE" => "replace",
        "UPDATE" => "update",
        "DELETE" => "delete",
        "PRAGMA" => "pragma",
        "VACUUM" => "vacuum",
        "ATTACH" => "attach",
        "DETACH" => "detach",
        "REINDEX" => "reindex",
        "ANALYZE" => "analyze",
        _ => "alter",
    }
}

impl Parser<'_> {
    /// One statement: a query as a tree, any other statement as a gap.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        match self.kind_at(0) {
            Some(TokenKind::Keyword("SELECT" | "VALUES")) => self.select(),
            Some(TokenKind::Keyword("WITH")) => {
                let with: With = self.with_clause()?;
                match self.kind_at(0) {
                    Some(TokenKind::Keyword(keyword)) if WITH_STATEMENTS.contains(&keyword) => {
                        self.statement_gap(keyword, start)
                    }
                    _ => self.select_with(Some(with)),
                }
            }
            Some(TokenKind::Keyword(keyword)) if STATEMENT_KEYWORDS.contains(&keyword) => {
                self.statement_gap(keyword, start)
            }
            _ => Err(self.error()),
        }
    }

    /// A statement other than a query: a gap over its tokens, a trigger's body included.
    fn statement_gap(&mut self, keyword: &str, start: usize) -> Parsing<Expression> {
        let trigger: bool = matches!(keyword, "CREATE")
            && (self.kw_at(1, "TRIGGER") || (self.kw_at(1, "TEMP") && self.kw_at(2, "TRIGGER")));
        self.skip_statement(trigger)?;
        self.gap(gap_name(keyword), start);
        Ok(command(""))
    }
}
