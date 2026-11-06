//! `PreparableStmt` (gram.y): queries, `INSERT`, `UPDATE` and `DELETE`; `MERGE` is a gap.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::OnConflict;
use sqlquack_syntax::expressions::queries::models::{From, Where, With};

use crate::parse::_helpers::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::expressions::binary_node;
use crate::parse::_helpers::nodes::{delete, insert, ordered, tuple, update};
use crate::scan::models::TokenKind;

impl Parser<'_> {
    /// `[WITH ...] SELECT | INSERT | UPDATE | DELETE | MERGE`.
    pub(crate) fn preparable_statement(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let with: Option<With> = match parser.kw("with") {
                true => Some(parser.with_clause()?),
                false => None,
            };
            match parser.keyword_at(0) {
                Some("insert") => parser.insert_statement(with),
                Some("update") => parser.update_statement(with),
                Some("delete") => parser.delete_statement(with),
                Some("merge") => {
                    let start: usize = parser.offset();
                    parser.skip_statement_rest()?;
                    parser.gap("merge", start);
                    Ok(Expression::Raw(
                        sqlquack_syntax::expressions::statements::models::Raw {
                            sql: "MERGE".to_owned(),
                        },
                    ))
                }
                _ => {
                    let body: Expression = parser.set_expression(0)?;
                    parser.select_options(body, with)
                }
            }
        })
    }

    /// Skips to the `;` or `)` ending the statement at this depth; a scanner fault stops it.
    pub(crate) fn skip_statement_rest(&mut self) -> Parsing<()> {
        let mut depth: usize = 0;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Fault(_) => return Err(self.error()),
                TokenKind::Char(';') if depth == 0 => return Ok(()),
                TokenKind::Char(')') if depth == 0 => return Ok(()),
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') => depth -= 1,
                _ => {}
            }
            self.at += 1;
        }
        Ok(())
    }

    /// `RETURNING [WITH (...)] target_list`, when present.
    fn returning(&mut self) -> Parsing<Vec<Expression>> {
        if !self.eat_kw("returning") {
            return Ok(Vec::new());
        }
        let start: usize = self.offset();
        if self.kw("with") && self.char_at(1, '(') {
            self.at += 1;
            self.parenthesized(|parser| {
                if !(parser.eat_kw("old") || parser.eat_kw("new")) {
                    return Err(parser.error());
                }
                parser.expect_kw("as")?;
                parser.name(is_col_id)
            })?;
            self.gap("returning_with", start);
        }
        self.comma_list(|parser| parser.target())
    }

    /// `ColId opt_indirection` naming a column to set; indirection is a gap.
    fn set_target(&mut self) -> Parsing<Identifier> {
        let start: usize = self.offset();
        let name: Identifier = self.name(is_col_id)?;
        if self.is_char('.') || self.is_char('[') {
            self.indirection(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ))?;
            self.gap("set_target_indirection", start);
        }
        Ok(name)
    }

    /// `set_clause_list`: `col = expr` and `(cols) = expr`, the latter a gap.
    fn set_clauses(&mut self) -> Parsing<Vec<(Identifier, Expression)>> {
        let mut clauses: Vec<(Identifier, Expression)> = Vec::new();
        loop {
            let start: usize = self.offset();
            if self.is_char('(') {
                let targets: Vec<Identifier> = self.parenthesized(|parser| parser.set_target())?;
                self.expect_char('=')?;
                let value: Expression = self.a_expr()?;
                self.gap("multiple_column_set", start);
                if let Some(first) = targets.into_iter().next() {
                    clauses.push((first, value));
                }
            } else {
                let target: Identifier = self.set_target()?;
                self.expect_char('=')?;
                clauses.push((target, self.a_expr()?));
            }
            if !self.eat_char(',') {
                return Ok(clauses);
            }
        }
    }

    /// `WHERE a_expr` or `WHERE CURRENT OF cursor` (a gap), when present.
    fn where_or_current(&mut self) -> Parsing<Option<Where>> {
        if !self.eat_kw("where") {
            return Ok(None);
        }
        let start: usize = self.offset();
        if self.kw("current") && self.kw_at(1, "of") {
            self.at += 2;
            self.name(is_col_id)?;
            self.gap("where_current_of", start);
            return Ok(None);
        }
        Ok(Some(Where {
            this: self.a_expr()?,
        }))
    }

    /// `relation_expr_opt_alias`; `SET` after an `UPDATE` target is never its alias.
    fn relation_with_alias(&mut self, update_target: bool) -> Parsing<(TableRef, bool)> {
        let mut relation: TableRef = self.relation()?;
        let explicit: bool = self.eat_kw("as");
        let takes: bool = self.is(is_col_id) && !(update_target && self.kw("set"));
        if explicit || takes {
            relation.alias = Some(self.name(is_col_id)?);
            relation.alias_explicit_as = explicit;
        }
        Ok((relation, explicit))
    }

    fn insert_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("insert")?;
        self.expect_kw("into")?;
        let name: Vec<Identifier> = self.table_name()?;
        let mut target = insert(crate::parse::_helpers::nodes::table(name), Vec::new());
        if self.eat_kw("as") {
            target.alias = Some(self.name(is_col_id)?);
            target.alias_explicit_as = true;
        }
        if self.is_char('(') && !self.starts_query_at(1) {
            target.columns = self.parenthesized(|parser| parser.set_target())?;
        }
        let start: usize = self.offset();
        if self.eat_kw("overriding") {
            if !(self.eat_kw("user") || self.eat_kw("system")) {
                return Err(self.error());
            }
            self.expect_kw("value")?;
            self.gap("overriding", start);
        }
        if self.kw("default") && self.kw_at(1, "values") {
            self.at += 2;
            target.default_values = true;
        } else {
            let query: Expression = self.preparable_query()?;
            match query {
                Expression::Values(rows) => {
                    target.values = rows
                        .expressions
                        .into_iter()
                        .map(|row| row.expressions)
                        .collect();
                }
                other => target.query = Some(other),
            }
        }
        if self.kw("on") && self.kw_at(1, "conflict") {
            self.at += 2;
            target.on_conflict = Some(Box::new(self.on_conflict()?));
        }
        target.returning = self.returning()?;
        target.with = with;
        Ok(Expression::Insert(Box::new(target)))
    }

    /// The `SelectStmt` of an `INSERT`.
    fn preparable_query(&mut self) -> Parsing<Expression> {
        let with: Option<With> = match self.kw("with") {
            true => Some(self.with_clause()?),
            false => None,
        };
        let body: Expression = self.set_expression(0)?;
        self.select_options(body, with)
    }

    /// `ON CONFLICT [target] DO NOTHING | DO UPDATE SET ... [WHERE ...]`, after `ON CONFLICT`.
    fn on_conflict(&mut self) -> Parsing<Expression> {
        let mut node: OnConflict = OnConflict {
            duplicate: None,
            expressions: Vec::new(),
            action: None,
            conflict_keys: None,
            index_predicate: None,
            constraint: None,
            where_: None,
        };
        if self.is_char('(') {
            let keys: Vec<Expression> = self.parenthesized(|parser| parser.index_element())?;
            node.conflict_keys = Some(Box::new(tuple(keys)));
            if self.eat_kw("where") {
                node.index_predicate = Some(Box::new(self.a_expr()?));
            }
        } else if self.kw("on") && self.kw_at(1, "constraint") {
            self.at += 2;
            node.constraint = Some(Box::new(Expression::Identifier(self.name(is_col_id)?)));
        }
        self.expect_kw("do")?;
        if self.eat_kw("nothing") {
            node.action = Some(Box::new(Expression::Identifier(Identifier::new("NOTHING"))));
        } else {
            self.expect_kw("update")?;
            self.expect_kw("set")?;
            let assignments: Vec<Expression> = self
                .set_clauses()?
                .into_iter()
                .map(|(name, value)| binary_node("=", Expression::Identifier(name), value))
                .collect();
            node.action = Some(Box::new(tuple(assignments)));
            if self.eat_kw("where") {
                node.where_ = Some(Box::new(self.a_expr()?));
            }
        }
        Ok(Expression::OnConflict(Box::new(node)))
    }

    /// `index_elem`: a column, a call, or `( a_expr )`, with collation, operator class and order.
    fn index_element(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let this: Expression = match self.is_char('(') {
            true => {
                self.at += 1;
                let inner: Expression = self.a_expr()?;
                self.expect_char(')')?;
                inner
            }
            false => self.c_expr(crate::parse::_helpers::expressions::Flavor::B)?,
        };
        if self.eat_kw("collate") {
            self.qualified_name()?;
            self.gap("index_element_collate", start);
        }
        if self.is(is_col_id) && !self.kw("nulls") {
            self.qualified_name()?;
            self.gap("index_element_opclass", start);
        }
        let desc: bool = self.eat_kw("desc");
        let explicit_asc: bool = !desc && self.eat_kw("asc");
        let nulls_first: Option<bool> = self.nulls_order();
        Ok(Expression::Ordered(Box::new(ordered(
            this,
            desc,
            explicit_asc,
            nulls_first,
        ))))
    }

    fn update_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("update")?;
        let (relation, _) = self.relation_with_alias(true)?;
        let mut node = update(relation);
        self.expect_kw("set")?;
        node.set = self.set_clauses()?;
        if self.eat_kw("from") {
            let (items, joins) = self.parse_from_list()?;
            node.from_clause = Some(From { expressions: items });
            node.from_joins = joins;
        }
        node.where_clause = self.where_or_current()?;
        node.returning = self.returning()?;
        node.with = with;
        Ok(Expression::Update(Box::new(node)))
    }

    fn delete_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("delete")?;
        self.expect_kw("from")?;
        let (mut relation, explicit) = self.relation_with_alias(false)?;
        let alias: Option<Identifier> = relation.alias.take();
        relation.alias_explicit_as = false;
        let mut node = delete(relation);
        node.alias = alias;
        node.alias_explicit_as = explicit;
        let start: usize = self.offset();
        if self.eat_kw("using") {
            let (items, joins) = self.parse_from_list()?;
            let mut opaque: bool = !joins.is_empty();
            for item in items {
                match item {
                    Expression::Table(relation) => node.using.push(*relation),
                    _ => opaque = true,
                }
            }
            if opaque {
                self.gap("delete_using_non_table", start);
            }
        }
        node.where_clause = self.where_or_current()?;
        node.returning = self.returning()?;
        node.with = with;
        Ok(Expression::Delete(Box::new(node)))
    }
}
