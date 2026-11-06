//! `PreparableStmt` (gram.y): queries, `INSERT`, `UPDATE`, `DELETE` and `MERGE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::{OnConflict, Returning};
use sqlquack_syntax::expressions::queries::models::{From, Join, Where, With};
use sqlquack_syntax::expressions::statements::models::{Merge, Var, When, Whens};

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::{
    boolean, delete, insert, joined_table, ordered, tuple, update,
};
use crate::parse::_helpers::values::expressions::binary_node;
use crate::parse::constants::ROW_CONSTRUCTOR;
use crate::scan::models::TokenKind;

/// A `set_clause_list` as pairs, or as the whole list of `(targets, source)` assignments.
type SetClauses = (Vec<(Identifier, Expression)>, Vec<Expression>);

/// One assignment of `set_clause_list`, as `(targets, source)`.
fn assignment(targets: Vec<Identifier>, source: Expression) -> Expression {
    tuple(vec![
        tuple(targets.into_iter().map(Expression::Identifier).collect()),
        source,
    ])
}

/// The `count` items of a row `(a, b)` or `ROW(a, b)`, which `SET (x, y) = …` splits.
fn row_items(value: &Expression, count: usize) -> Option<Vec<Expression>> {
    match value {
        Expression::Tuple(row) if row.expressions.len() == count => Some(row.expressions.clone()),
        Expression::Function(call) if call.args.len() == count && call.name == ROW_CONSTRUCTOR => {
            Some(call.args.clone())
        }
        _ => None,
    }
}

impl Parser<'_> {
    /// `[WITH ...] SELECT | INSERT | UPDATE | DELETE | MERGE`.
    pub(crate) fn preparable_statement(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let first: usize = parser.at;
            let with: Option<With> = match parser.kw("with") {
                true => Some(parser.with_clause()?),
                false => None,
            };
            match parser.keyword_at(0) {
                Some("insert") => parser.insert_statement(with),
                Some("update") => parser.update_statement(with),
                Some("delete") => parser.delete_statement(with),
                Some("merge") => parser.merge_statement(with),
                _ => {
                    let body: Expression = parser.set_expression(0)?;
                    let query: Expression = parser.select_options(body, with)?;
                    Ok(parser.respanned(first, query))
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

    /// `RETURNING [WITH (...)] target_list`, when present, as its aliases and its targets.
    fn returning(&mut self) -> Parsing<(Vec<Expression>, Vec<Expression>)> {
        if !self.eat_kw("returning") {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut aliases: Vec<Expression> = Vec::new();
        if self.kw("with") && self.char_at(1, '(') {
            self.at += 1;
            aliases = self.parenthesized(|parser| {
                let which: &str = match parser.keyword_at(0) {
                    Some("old") => "OLD",
                    Some("new") => "NEW",
                    _ => return Err(parser.error()),
                };
                parser.at += 1;
                parser.expect_kw("as")?;
                let name: Identifier = parser.name(is_col_id)?;
                Ok(tuple(vec![var(which), Expression::Identifier(name)]))
            })?;
        }
        Ok((aliases, self.comma_list(|parser| parser.target())?))
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

    /// `set_clause_list` as `col = expr` pairs, with `(cols) = row` split into one pair each.
    fn set_clauses(&mut self) -> Parsing<Vec<(Identifier, Expression)>> {
        let start: usize = self.offset();
        let (clauses, assignments) = self.set_clause_list()?;
        if !assignments.is_empty() {
            self.gap("multiple_column_set", start);
        }
        Ok(clauses)
    }

    /// `set_clause_list` as pairs, and as `(targets, source)` items when a `(cols) = source` is no row.
    fn set_clause_list(&mut self) -> Parsing<SetClauses> {
        let mut clauses: Vec<(Identifier, Expression)> = Vec::new();
        let mut assignments: Vec<Expression> = Vec::new();
        let mut unsplit: bool = false;
        loop {
            let multiple: bool = self.is_char('(');
            let targets: Vec<Identifier> = match multiple {
                true => self.parenthesized(|parser| parser.set_target())?,
                false => vec![self.set_target()?],
            };
            self.expect_char('=')?;
            let value: Expression = self.a_expr()?;
            let pairs: Vec<(Identifier, Expression)> = match multiple {
                false => vec![(targets[0].clone(), value.clone())],
                true => match row_items(&value, targets.len()) {
                    Some(items) => targets.iter().cloned().zip(items).collect(),
                    None => {
                        unsplit = true;
                        Vec::new()
                    }
                },
            };
            match pairs.is_empty() {
                true => assignments.push(assignment(targets, value)),
                false => {
                    for (target, item) in &pairs {
                        assignments.push(assignment(vec![target.clone()], item.clone()));
                    }
                    clauses.extend(pairs);
                }
            }
            if !self.eat_char(',') {
                return Ok(match unsplit {
                    true => (Vec::new(), assignments),
                    false => (clauses, Vec::new()),
                });
            }
        }
    }

    /// `WHERE a_expr` or `WHERE CURRENT OF cursor`, when present.
    fn where_or_current(&mut self) -> Parsing<(Option<Where>, Option<Identifier>)> {
        if !self.eat_kw("where") {
            return Ok((None, None));
        }
        if self.kw("current") && self.kw_at(1, "of") {
            self.at += 2;
            return Ok((None, Some(self.name(is_col_id)?)));
        }
        Ok((
            Some(Where {
                this: self.a_expr()?,
            }),
            None,
        ))
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
        let mut target = insert(crate::parse::_helpers::tree::nodes::table(name), Vec::new());
        if self.eat_kw("as") {
            target.alias = Some(self.name(is_col_id)?);
            target.alias_explicit_as = true;
        }
        if self.is_char('(') && !self.starts_query_at(1) {
            target.columns = self.parenthesized(|parser| parser.set_target())?;
        }
        target.overriding = self.overriding()?;
        if target.overriding.is_none() && self.kw("default") && self.kw_at(1, "values") {
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
        (target.returning_aliases, target.returning) = self.returning()?;
        target.with = with;
        Ok(Expression::Insert(Box::new(target)))
    }

    /// The `SelectStmt` of an `INSERT`.
    fn preparable_query(&mut self) -> Parsing<Expression> {
        self.query()
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
            false => self.c_expr(crate::parse::_helpers::values::expressions::Flavor::B)?,
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
        (node.set, node.assignments) = self.set_clause_list()?;
        if self.eat_kw("from") {
            let (items, joins) = self.parse_from_list()?;
            node.from_clause = Some(From { expressions: items });
            node.from_joins = joins;
        }
        (node.where_clause, node.current_of) = self.where_or_current()?;
        (node.returning_aliases, node.returning) = self.returning()?;
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
        (node.where_clause, node.current_of) = self.where_or_current()?;
        (node.returning_aliases, node.returning) = self.returning()?;
        node.with = with;
        Ok(Expression::Delete(Box::new(node)))
    }

    /// `MERGE INTO target [alias] USING table_ref ON condition merge_when_clause... [RETURNING]`.
    fn merge_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("merge")?;
        self.expect_kw("into")?;
        let (target, _) = self.relation_with_alias(false)?;
        self.expect_kw("using")?;
        let mut source: Expression = self.table_ref()?;
        let joins: Vec<Join> = self.joins()?;
        if !joins.is_empty() {
            source = joined_table(source, joins, None);
        }
        self.expect_kw("on")?;
        let on: Expression = self.a_expr()?;
        let mut whens: Vec<Expression> = vec![self.merge_when()?];
        while self.kw("when") {
            whens.push(self.merge_when()?);
        }
        let (returning_aliases, returning) = self.returning()?;
        Ok(Expression::Merge(Box::new(Merge {
            this: Box::new(Expression::Table(Box::new(target))),
            using: Box::new(source),
            on: Some(Box::new(on)),
            using_cond: None,
            whens: Some(Box::new(Expression::Whens(Box::new(Whens {
                expressions: whens,
            })))),
            with_: with.map(|with| Box::new(Expression::With(Box::new(with)))),
            returning: match returning.is_empty() {
                true => None,
                false => Some(Box::new(Expression::Returning(Box::new(Returning {
                    expressions: returning,
                    into: None,
                })))),
            },
            returning_aliases,
        })))
    }

    /// `merge_when_clause`: `WHEN [NOT] MATCHED [BY SOURCE|TARGET] [AND condition] THEN action`.
    fn merge_when(&mut self) -> Parsing<Expression> {
        self.expect_kw("when")?;
        let not: bool = self.eat_kw("not");
        self.expect_kw("matched")?;
        let mut by_source: bool = false;
        if not && self.eat_kw("by") {
            by_source = match self.keyword_at(0) {
                Some("source") => true,
                Some("target") => false,
                _ => return Err(self.error()),
            };
            self.at += 1;
        }
        let condition: Option<Box<Expression>> = match self.eat_kw("and") {
            true => Some(Box::new(self.a_expr()?)),
            false => None,
        };
        self.expect_kw("then")?;
        let matched: bool = !not || by_source;
        let then: Expression = self.merge_action(matched)?;
        Ok(Expression::When(Box::new(When {
            matched: Some(Box::new(boolean(!not))),
            source: by_source.then(|| Box::new(boolean(true))),
            condition,
            then: Box::new(then),
        })))
    }

    /// The action after `THEN`, as a verb and its parts; `INSERT` only when not matched.
    fn merge_action(&mut self, matched: bool) -> Parsing<Expression> {
        let verb = |text: &str| {
            Expression::Var(Box::new(Var {
                this: text.to_owned(),
            }))
        };
        if self.kw("do") && self.kw_at(1, "nothing") {
            self.at += 2;
            return Ok(verb("DO NOTHING"));
        }
        match (matched, self.keyword_at(0)) {
            (true, Some("update")) => {
                self.at += 1;
                self.expect_kw("set")?;
                let assignments: Vec<Expression> = self
                    .set_clauses()?
                    .into_iter()
                    .map(|(target, value)| binary_node("=", Expression::Identifier(target), value))
                    .collect();
                Ok(tuple(vec![verb("UPDATE"), tuple(assignments)]))
            }
            (true, Some("delete")) => {
                self.at += 1;
                Ok(verb("DELETE"))
            }
            (false, Some("insert")) => {
                self.at += 1;
                self.merge_insert()
                    .map(|parts| tuple(std::iter::once(verb("INSERT")).chain(parts).collect()))
            }
            _ => Err(self.error()),
        }
    }

    /// `OVERRIDING USER VALUE` or `OVERRIDING SYSTEM VALUE`, when present, as "USER" or "SYSTEM".
    fn overriding(&mut self) -> Parsing<Option<String>> {
        if !self.eat_kw("overriding") {
            return Ok(None);
        }
        let kind: &str = match self.keyword_at(0) {
            Some("user") => "USER",
            Some("system") => "SYSTEM",
            _ => return Err(self.error()),
        };
        self.at += 1;
        self.expect_kw("value")?;
        Ok(Some(kind.to_owned()))
    }

    /// `merge_insert` after `INSERT`, as its column and value tuples or `DEFAULT VALUES`.
    fn merge_insert(&mut self) -> Parsing<Vec<Expression>> {
        if self.kw("default") && self.kw_at(1, "values") {
            self.at += 2;
            return Ok(vec![Expression::Var(Box::new(Var {
                this: "DEFAULT VALUES".to_owned(),
            }))]);
        }
        let mut parts: Vec<Expression> = Vec::new();
        if self.is_char('(') {
            let columns: Vec<Identifier> = self.parenthesized(|parser| parser.set_target())?;
            parts.push(tuple(
                columns.into_iter().map(Expression::Identifier).collect(),
            ));
        }
        if let Some(overriding) = self.overriding()? {
            parts.push(tuple(vec![var("OVERRIDING"), var(&overriding)]));
        }
        self.expect_kw("values")?;
        self.expect_char('(')?;
        parts.push(tuple(self.expr_list()?));
        self.expect_char(')')?;
        Ok(parts)
    }
}
