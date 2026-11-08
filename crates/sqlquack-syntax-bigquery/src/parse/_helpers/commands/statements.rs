//! `sql_statement` and `script` (googlesql.tm): the statements this parser builds, and the rest.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Null;
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
            if parser.is_op("|>") {
                let start: usize = parser.offset();
                while parser.is_op("|>") {
                    parser.at += 1;
                    parser.deeper(|parser| parser.pipe(Expression::Null(Null)))?;
                }
                parser.gap("subpipeline_statement", start);
                return Ok(Expression::Null(Null));
            }
            if parser.is(crate::parse::_helpers::tree::cursor::is_identifier)
                && parser.char_at(1, ':')
            {
                let start: usize = parser.offset();
                parser.at += 2;
                parser.skip_block()?;
                parser.gap("label", start);
                return Ok(Expression::Null(
                    sqlquack_syntax::expressions::names::models::Null,
                ));
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
                "CREATE" => parser.create_statement(),
                "GRAPH" => {
                    let start: usize = parser.offset();
                    parser.skip_statement_rest()?;
                    parser.gap("graph", start);
                    Ok(Expression::Null(
                        sqlquack_syntax::expressions::names::models::Null,
                    ))
                }
                "DROP" => parser.drop_statement(),
                "ALTER" => parser.alter_statement(),
                "UNDROP" => parser.undrop_statement(),
                "TRUNCATE" => parser.truncate_statement(),
                "GRANT" => parser.grant_statement(),
                "REVOKE" => parser.revoke_statement(),
                _ => parser.opaque_statement(),
            }
        })
    }

    /// A statement outside this slice: a gap named by its first word, its text unchecked.
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
        let block: bool = match construct {
            "begin" => !(self.kw_at(1, "TRANSACTION") || self.ends_at(1)),
            "if" | "loop" | "while" | "repeat" | "for" | "case" => true,
            _ => false,
        };
        match block {
            true => self.skip_block()?,
            false => {
                self.at += 1;
                self.skip_statement_rest()?;
            }
        }
        self.gap(construct, start);
        Ok(Expression::Null(
            sqlquack_syntax::expressions::names::models::Null,
        ))
    }
}
