//! `ParserQuery`'s statements SQLBuild runs; every other statement is a named gap.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{AlterTable, AlterTableAction};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::queries::models::{SelectInto, Where};
use sqlquack_syntax::expressions::statements::models::{Describe, Kill, Truncate};
use sqlquack_syntax::expressions::tables::models::OnCluster;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{call, command, delete, insert, table, var};
use crate::parse::errors::ParseError;
use crate::scan::models::TokenKind;

impl Parser<'_> {
    /// One statement of a multi-statement input and its `;`, as `splitMultipartQuery` reads it.
    pub(crate) fn top_level_statement(&mut self) -> Parsing<Expression> {
        let Some(first) = self.peek().cloned() else {
            return Err(self.empty_query());
        };
        if first.kind == TokenKind::Semicolon {
            return Err(self.empty_query());
        }
        if first.kind.is_error() {
            return Err(self.error());
        }
        self.opaque = false;
        let parsed: Parsing<Expression> = self.statement();
        if let Some(error) = self.too_deep.take() {
            return Err(error);
        }
        let statement: Expression = parsed?;
        if !self.at_end() {
            return Err(self.error());
        }
        let end: usize = self
            .at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(first.end, |token| token.end);
        self.eat(TokenKind::Semicolon);
        Ok(match self.opaque {
            true => command(&self.sql[first.start..end]),
            false => statement,
        })
    }

    /// The refusal of a statement with no tokens: a leading `;`, `;;` or an input of blanks.
    pub(crate) fn empty_query(&self) -> ParseError {
        self.error_with("Empty query", true)
    }

    /// One statement, as `ParserQuery` tries its parsers.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        let word: String = self.word_at(0).unwrap_or_default();
        match word.as_str() {
            "SELECT" | "WITH" | "FROM" => self.query_with_output(),
            _ if self.is(TokenKind::OpeningRoundBracket) => self.query_with_output(),
            "INSERT" => self.insert_statement(),
            "DELETE" => self.delete_statement(),
            "TRUNCATE" => self.truncate_statement(),
            "OPTIMIZE" => self.optimize_statement(),
            "KILL" if self.kw_at(1, "QUERY") => self.kill_statement(),
            "DESCRIBE" | "DESC" => self.describe_statement(),
            "RENAME" if self.kw_at(1, "TABLE") => self.rename_statement(),
            "EXCHANGE" if self.kw_at(1, "TABLES") => self.exchange_statement(),
            "CREATE" => self.create_statement(),
            "DROP" => self.drop_statement(),
            "ALTER" if self.kw_at(1, "TABLE") => self.alter_statement(),
            _ => self.unsupported_statement(),
        }
    }

    /// A statement this parser does not read: its tokens up to the `;`, as a gap.
    pub(crate) fn unsupported_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.at_end() {
            return Err(self.error());
        }
        self.skip_statement_rest()?;
        self.gap("unsupported_statement", start);
        Ok(var(""))
    }

    /// `ParserQueryWithOutput`: `INTO OUTFILE`, then `FORMAT` and `SETTINGS` in either order.
    fn query_with_output(&mut self) -> Parsing<Expression> {
        let mut query: Expression = self.select_with_union()?;
        if self.kws(&["INTO", "OUTFILE"]) {
            let start: usize = self.offset();
            self.at += 2;
            let into: SelectInto = self.outfile_target()?;
            match &mut query {
                Expression::Select(select) => select.into = Some(into),
                _ => self.gap("into_outfile", start),
            }
        }
        loop {
            let start: usize = self.offset();
            if self.eat_kw("FORMAT") {
                let format: Expression = self.format_name()?;
                match &mut query {
                    Expression::Select(select) if select.format.is_none() => {
                        select.format = Some(format)
                    }
                    _ => self.gap("query_output_format", start),
                }
                continue;
            }
            if self.eat_kw("SETTINGS") {
                let settings: Vec<Expression> = self.settings_list()?;
                match &mut query {
                    Expression::Select(select) => select
                        .settings
                        .get_or_insert_with(Vec::new)
                        .extend(settings),
                    _ => self.gap("query_output_settings", start),
                }
                continue;
            }
            return Ok(query);
        }
    }

    /// An `INTO OUTFILE` target: the file, its modifiers as words and its compression as a call.
    fn outfile_target(&mut self) -> Parsing<SelectInto> {
        let this: Expression = self.string_expression()?;
        let mut expressions: Vec<Expression> = Vec::new();
        for words in [&["APPEND"][..], &["TRUNCATE"], &["AND", "STDOUT"]] {
            if self.eat_kws(words) {
                expressions.push(var(&words.join(" ")));
            }
        }
        if self.eat_kw("COMPRESSION") {
            let mut args: Vec<Expression> = vec![self.string_expression()?];
            if self.eat_kw("LEVEL") {
                let signed: bool = self.is(TokenKind::Minus) || self.is(TokenKind::Plus);
                if !self.kind_at(usize::from(signed), TokenKind::Number) {
                    return Err(self.error());
                }
                args.push(self.literal_value()?);
            }
            expressions.push(call("COMPRESSION", args));
        }
        Ok(SelectInto {
            this,
            temporary: false,
            unlogged: false,
            bulk_collect: false,
            expressions,
        })
    }

    /// `TRUNCATE [TABLE] [IF EXISTS] name [ON CLUSTER c] [SYNC]`; other TRUNCATE forms are gaps.
    fn truncate_statement(&mut self) -> Parsing<Expression> {
        let at: usize = self.at;
        self.expect_kw("TRUNCATE")?;
        let other: &[&str] = &[
            "ALL",
            "DATABASE",
            "TABLES",
            "TEMPORARY",
            "VIEW",
            "DICTIONARY",
        ];
        if self.kw_in(0, other) {
            self.at = at;
            return self.unsupported_statement();
        }
        self.eat_kw("TABLE");
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        if self.kws(&["IF", "EMPTY"]) {
            self.at = at;
            return self.unsupported_statement();
        }
        let mut node: Truncate = Truncate::new(String::new());
        node.table = self.table_name()?;
        node.if_exists = if_exists;
        if self.is(TokenKind::Comma) {
            return Err(self.error_with("Only Support DROP multiple tables currently", true));
        }
        node.on_cluster = self.on_cluster()?;
        let start: usize = self.offset();
        if self.eat_kws(&["NO", "DELAY"]) {
            self.gap("truncate_no_delay", start);
        } else {
            self.eat_kw("SYNC");
        }
        self.output_clauses()?;
        Ok(Expression::Truncate(Box::new(node)))
    }

    /// `ParserOptimizeQuery`, read whole as a `Command`; `DEDUPLICATE BY` is a gap.
    fn optimize_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["OPTIMIZE", "TABLE"])?;
        self.table_name()?;
        self.on_cluster()?;
        if self.eat_kw("PARTITION") {
            self.partition()?;
        }
        if !self.eat_kw("FINAL") {
            self.eat_kw("FORCE");
        }
        let deduplicate: bool = self.eat_kw("DEDUPLICATE");
        self.eat_kw("CLEANUP");
        let start: usize = self.offset();
        if deduplicate && self.eat_kw("BY") {
            self.skip_statement_rest()?;
            self.gap("optimize_deduplicate_by", start);
        }
        self.output_clauses()?;
        self.opaque = true;
        Ok(var(""))
    }

    /// `INTO OUTFILE`, `FORMAT` or `SETTINGS` after a statement that is not a query, as a gap.
    pub(crate) fn output_clauses(&mut self) -> Parsing<()> {
        let start: usize = self.offset();
        if self.kws(&["INTO", "OUTFILE"]) || self.kw_in(0, &["FORMAT", "SETTINGS"]) {
            self.skip_statement_rest()?;
            self.gap("statement_output", start);
        }
        Ok(())
    }

    /// `[db.]name` as a table reference, leaving any third part for the caller to refuse.
    pub(crate) fn table_name(&mut self) -> Parsing<TableRef> {
        let mut parts: Vec<Identifier> = vec![self.ident()?];
        if self.eat(TokenKind::Dot) {
            parts.push(self.ident()?);
        }
        Ok(table(parts))
    }

    /// `ON CLUSTER name`, if next.
    pub(crate) fn on_cluster(&mut self) -> Parsing<Option<OnCluster>> {
        if !self.kws(&["ON", "CLUSTER"]) {
            return Ok(None);
        }
        self.at += 2;
        let name: Expression = match self.is(TokenKind::StringLiteral) {
            true => self.string_expression()?,
            false => Expression::Identifier(self.ident()?),
        };
        Ok(Some(OnCluster {
            this: Box::new(name),
        }))
    }

    /// `ParserInsertQuery`: a table, columns and settings, then `VALUES`, `FORMAT` or a query.
    fn insert_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("INSERT")?;
        self.expect_kw("INTO")?;
        self.eat_kw("TABLE");
        if self.kw("FUNCTION") {
            self.skip_statement_rest()?;
            self.gap("insert_into_function", start);
            return Ok(var(""));
        }
        let mut node = insert(self.table_name()?);
        if self.is(TokenKind::OpeningRoundBracket) {
            self.at += 1;
            node.columns = self.comma_list(|parser| parser.ident())?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        }
        if self.eat_kw("SETTINGS") {
            node.settings = self.settings_list()?;
        }
        if self.eat_kw("FORMAT") {
            let format: String = self.advance()?.value;
            self.at = self.tokens.len();
            node.query = Some(command(&format!("FORMAT {format}")));
        } else if self.eat_kw("VALUES") {
            node.values = self.comma_list(|parser| {
                parser.expect_kind(TokenKind::OpeningRoundBracket)?;
                let row: Vec<Expression> = parser.layer_elements(TokenKind::ClosingRoundBracket)?;
                Ok(row)
            })?;
        } else {
            node.query = Some(self.query_with_output()?);
        }
        Ok(Expression::Insert(Box::new(node)))
    }

    /// `ParserDeleteQuery`: `DELETE FROM name [ON CLUSTER c] [IN PARTITION p] WHERE e [SETTINGS ...]`.
    fn delete_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["DELETE", "FROM"])?;
        let mut node = delete(self.table_name()?);
        node.on_cluster = self.on_cluster()?;
        if self.kws(&["IN", "PARTITION"]) {
            let start: usize = self.offset();
            self.at += 2;
            self.partition()?;
            self.gap("delete_in_partition", start);
        }
        self.expect_kw("WHERE")?;
        node.where_clause = Some(Where {
            this: self.expression()?,
        });
        if self.eat_kw("SETTINGS") {
            node.settings = self.settings_list()?;
        }
        Ok(Expression::Delete(Box::new(node)))
    }

    /// `ParserKillQueryQuery`: `KILL QUERY [ON CLUSTER c] WHERE e [SYNC|ASYNC|TEST]`.
    fn kill_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["KILL", "QUERY"])?;
        let start: usize = self.offset();
        if self.on_cluster()?.is_some() {
            self.gap("kill_on_cluster", start);
        }
        self.expect_kw("WHERE")?;
        let this: Expression = self.expression()?;
        if !self.eat_kw("ASYNC") && !self.eat_kw("SYNC") {
            self.eat_kw("TEST");
        }
        Ok(Expression::Kill(Box::new(Kill {
            this,
            kind: Some("QUERY".to_owned()),
        })))
    }

    /// `ParserDescribeTableQuery`: `DESCRIBE|DESC [TABLE] name`.
    fn describe_statement(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let kind: Option<String> = self.eat_kw("TABLE").then(|| "TABLE".to_owned());
        let target: Expression = match self.subquery_at(0) {
            true => self.table_expression(false)?,
            false => Expression::Table(Box::new(self.table_name()?)),
        };
        let mut node: Describe = Describe::new(target);
        node.kind = kind;
        Ok(Expression::Describe(Box::new(node)))
    }

    /// `RENAME TABLE a TO b [, ...] [ON CLUSTER c]`, as `ALTER TABLE a RENAME TO b`.
    fn rename_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["RENAME", "TABLE"])?;
        let start: usize = self.offset();
        let pairs: Vec<(TableRef, TableRef)> = self.comma_list(|parser| {
            let from: TableRef = parser.table_name()?;
            parser.expect_kw("TO")?;
            Ok((from, parser.table_name()?))
        })?;
        let on_cluster: Option<OnCluster> = self.on_cluster()?;
        if pairs.len() > 1 {
            self.gap("rename_many_tables", start);
        }
        let mut pairs = pairs.into_iter();
        let Some((from, to)) = pairs.next() else {
            return Err(self.error());
        };
        Ok(alter(
            from,
            vec![AlterTableAction::RenameTable(to)],
            on_cluster,
        ))
    }

    /// `EXCHANGE TABLES a AND b [ON CLUSTER c]`, as `ALTER TABLE a SWAP WITH b`.
    fn exchange_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["EXCHANGE", "TABLES"])?;
        let left: TableRef = self.table_name()?;
        self.expect_kw("AND")?;
        let right: TableRef = self.table_name()?;
        let on_cluster: Option<OnCluster> = self.on_cluster()?;
        Ok(alter(
            left,
            vec![AlterTableAction::SwapWith(right)],
            on_cluster,
        ))
    }

    /// `ParserPartition`: `ID 'id'`, `ALL`, or an expression.
    pub(crate) fn partition(&mut self) -> Parsing<Expression> {
        if self.kw("ID") && self.kind_at(1, TokenKind::StringLiteral) {
            self.at += 1;
            let id: Expression = self.string_expression()?;
            return Ok(crate::parse::_helpers::tree::nodes::call("ID", vec![id]));
        }
        if self.eat_kw("ALL") {
            return Ok(var("ALL"));
        }
        self.expression()
    }

    /// The source text from byte `start` to the last token consumed.
    pub(crate) fn text_from(&self, start: usize) -> String {
        let end: usize = self
            .at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(start, |token| token.end);
        self.sql[start..end.max(start)].to_owned()
    }
}

/// `ALTER TABLE name` with `actions`.
pub(crate) fn alter(
    name: TableRef,
    actions: Vec<AlterTableAction>,
    on_cluster: Option<OnCluster>,
) -> Expression {
    let mut node: AlterTable = AlterTable::new(String::new());
    node.name = name;
    node.actions = actions;
    node.on_cluster = on_cluster;
    Expression::AlterTable(Box::new(node))
}
