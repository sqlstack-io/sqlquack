//! `statement` (SqlBaseParser.g4): the statements this crate builds, and the opaque rest.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::queries::models::With;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::constants::{STATEMENT_STARTS, STATEMENT_STARTS_MORE};

impl Parser<'_> {
    /// One statement.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            if parser.is_char('(') {
                return parser.query();
            }
            if parser.kw("ADD") || (parser.kw("LIST") && !parser.string_at(1)) {
                return parser.manage_resource();
            }
            if parser.kw("DENY") {
                return parser.deny_statement();
            }
            if let Some(statement) = parser.documented_statement() {
                return statement;
            }
            let Some(word) = parser.word_at(0) else {
                return Err(parser.error());
            };
            match word.as_str() {
                "SELECT" | "FROM" | "VALUES" | "TABLE" => parser.query(),
                "WITH" => {
                    let first: usize = parser.at;
                    let with: With = parser.ctes()?;
                    match parser.word_at(0).as_deref() {
                        Some("INSERT") => parser.insert_statement(Some(with)),
                        Some("MERGE") => parser.merge_statement(Some(with)),
                        Some("UPDATE") => parser.update_statement(Some(with)),
                        Some("DELETE") => parser.delete_statement(Some(with)),
                        _ => {
                            let query: Expression = parser.query_rest(Some(with))?;
                            Ok(parser.respanned(first, query))
                        }
                    }
                }
                "INSERT" => parser.insert_statement(None),
                "MERGE" => parser.merge_statement(None),
                "UPDATE" => parser.update_statement(None),
                "DELETE" => parser.delete_statement(None),
                "CREATE" | "REPLACE" => parser.create_statement(),
                "DROP" => parser.drop_statement(),
                "ALTER" => parser.alter_statement(),
                "DESC" | "DESCRIBE" => parser.describe_statement(),
                "BEGIN" => parser.begin_statement(),
                "RESET" => parser.reset_statement(),
                "SHOW" => parser.show_statement(),
                "SET" => parser.set_statement(),
                "USE" => parser.use_statement(),
                "COMMENT" => parser.comment_statement(),
                "TRUNCATE" => parser.truncate_statement(),
                "EXPLAIN" => parser.explain_statement(),
                "GRANT" => parser.grant_statement(),
                "REVOKE" => parser.revoke_statement(),
                "DECLARE" => parser.declare_statement(),
                "CACHE" => parser.cache_statement(),
                "UNCACHE" => parser.uncache_statement(),
                "COPY" => parser.copy_statement(),
                "OPTIMIZE" => parser.optimize_statement(),
                "VACUUM" => parser.vacuum_statement(),
                "RESTORE" => parser.restore_statement(),
                "MSCK" | "REPAIR" => parser.repair_statement(),
                "UNDROP" => parser.undrop_statement(),
                "LOAD" => parser.load_statement(),
                "REFRESH" => parser.refresh_resource(),
                _ => parser.opaque_statement(),
            }
        })
    }

    /// A statement the shared tree has no node for: a gap named by its first word, its text unchecked.
    pub(crate) fn opaque_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let word: String = self.word_at(0).unwrap_or_default().to_ascii_lowercase();
        let construct: &'static str = match STATEMENT_STARTS
            .iter()
            .chain(STATEMENT_STARTS_MORE.iter())
            .find(|start| **start == word)
        {
            Some(construct) => construct,
            None => return Err(self.error()),
        };
        self.at += 1;
        self.skip_statement_rest()?;
        self.gap(construct, start);
        Ok(Expression::Null(
            sqlquack_syntax::expressions::names::models::Null,
        ))
    }
}
