//! `INSERT`, `UPDATE`, `DELETE` and `MERGE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{BooleanLiteral, Identifier, TableRef};
use sqlquack_syntax::expressions::queries::models::{From, Join, Where, With};
use sqlquack_syntax::expressions::statements::models::{Merge, OutputClause, When, Whens};
use sqlquack_syntax::expressions::types::models::Tuple;

use crate::parse::_helpers::statements::queries::set_with;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_name};
use crate::parse::_helpers::tree::nodes::{column_from, delete, insert, table, update, var};

fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}

fn boolean(value: bool) -> Box<Expression> {
    Box::new(Expression::Boolean(BooleanLiteral { value }))
}

impl<'sql> Parser<'sql> {
    /// A DML statement after its `WITH`, which it then carries.
    pub(crate) fn dml_with(&mut self, with: With) -> Parsing<Expression> {
        let mut statement: Expression = match self.word_at(0).as_deref() {
            Some("INSERT") => self.insert_statement()?,
            Some("UPDATE") => self.update_statement()?,
            Some("DELETE") => self.delete_statement()?,
            Some("MERGE") => return Err(self.gap("merge_with", self.offset())),
            _ => {
                let mut query: Expression = self.query_expression()?;
                set_with(&mut query, with);
                self.query_tail()?;
                return Ok(query);
            }
        };
        set_with(&mut statement, with);
        Ok(statement)
    }

    /// The target of a DML statement: a table name, its hints refused as a gap.
    fn dml_target(&mut self) -> Parsing<TableRef> {
        if self.kw("TOP") {
            return Err(self.gap("dml_top", self.offset()));
        }
        if self.kw_in(0, &["OPENQUERY", "OPENROWSET", "OPENDATASOURCE"]) {
            return Err(self.gap("rowset_target", self.offset()));
        }
        if self
            .peek()
            .is_some_and(|token| token.kind == crate::scan::models::TokenKind::Variable)
        {
            return Err(self.gap("table_variable", self.offset()));
        }
        let name: Vec<Identifier> = self.multipart(4)?;
        if self.kw("WITH") && self.char_at(1, '(') {
            return Err(self.gap("table_hint", self.offset()));
        }
        Ok(table(name))
    }

    /// `OUTPUT ...`, when it comes next.
    fn output_clause(&mut self) -> Parsing<Option<OutputClause>> {
        match self.kw("OUTPUT") {
            true => Err(self.gap("output", self.offset())),
            false => Ok(None),
        }
    }

    /// `INSERT [INTO] t [(columns)] { VALUES (...), ... | query | DEFAULT VALUES | EXEC ... }`.
    pub(crate) fn insert_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("INSERT")?;
        if self.kw("BULK") {
            return Err(self.gap("insert_bulk", self.offset()));
        }
        self.eat_kw("INTO");
        let mut node = insert(self.dml_target()?);
        if self.is_char('(') {
            node.columns = self.parenthesized(Parser::name)?;
        }
        node.output = self.output_clause()?;
        if self.eat_kw("VALUES") {
            node.values = self.comma_list(|parser| {
                parser.parenthesized(|parser| match parser.kw("DEFAULT") {
                    true => {
                        parser.at += 1;
                        Ok(var("DEFAULT"))
                    }
                    false => parser.scalar(),
                })
            })?;
        } else if self.kw("DEFAULT") && self.kw_at(1, "VALUES") {
            self.at += 2;
            node.default_values = true;
        } else if self.kw_in(0, &["EXEC", "EXECUTE"]) {
            return Err(self.gap("insert_exec", self.offset()));
        } else {
            node.query = Some(self.query_statement()?);
        }
        Ok(Expression::Insert(Box::new(node)))
    }

    /// `UPDATE t SET col = value, ... [FROM ...] [WHERE ...]`.
    pub(crate) fn update_statement(&mut self) -> Parsing<Expression> {
        if self.kw_at(1, "STATISTICS") {
            return Err(self.gap("update_statistics", self.offset()));
        }
        self.expect_kw("UPDATE")?;
        let mut node = update(self.dml_target()?);
        self.expect_kw("SET")?;
        node.set = self.comma_list(|parser| {
            if !parser.is(is_name) {
                return Err(parser.gap("update_set_target", parser.offset()));
            }
            let parts: Vec<Identifier> = parser.multipart(4)?;
            if !parser.is_char('=') {
                return Err(parser.gap("update_set_operator", parser.offset()));
            }
            parser.at += 1;
            let value: Expression = match parser.kw("DEFAULT") {
                true => {
                    parser.at += 1;
                    var("DEFAULT")
                }
                false => parser.scalar()?,
            };
            Ok((crate::parse::_helpers::tree::nodes::joined(parts), value))
        })?;
        node.output = self.output_clause()?;
        if self.eat_kw("FROM") {
            let (from, joins) = self.read_sources()?;
            node.from_clause = Some(from);
            node.from_joins = joins;
        }
        if self.eat_kw("WHERE") {
            if self.kw("CURRENT") && self.kw_at(1, "OF") {
                return Err(self.gap("current_of", self.offset()));
            }
            node.where_clause = Some(Where {
                this: self.condition()?,
            });
        }
        if self.kw("OPTION") {
            return Err(self.gap("query_option", self.offset()));
        }
        Ok(Expression::Update(Box::new(node)))
    }

    /// `DELETE [FROM] t [FROM ...] [WHERE ...]`.
    pub(crate) fn delete_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("DELETE")?;
        let no_from: bool = !self.eat_kw("FROM");
        let mut node = delete(self.dml_target()?);
        node.no_from = no_from;
        node.output = self.output_clause()?;
        if self.eat_kw("FROM") {
            let (from, joins) = self.read_sources()?;
            node.using = Vec::new();
            node.tables = Vec::new();
            node.joins = joins;
            let mut sources = from.expressions.into_iter();
            if let Some(first) = sources.next() {
                if sources.next().is_some() {
                    return Err(self.gap("delete_from_list", self.offset()));
                }
                match first {
                    Expression::Table(reference) => node.using.push(*reference),
                    _ => return Err(self.gap("delete_from_source", self.offset())),
                }
            }
        }
        if self.eat_kw("WHERE") {
            if self.kw("CURRENT") && self.kw_at(1, "OF") {
                return Err(self.gap("current_of", self.offset()));
            }
            node.where_clause = Some(Where {
                this: self.condition()?,
            });
        }
        if self.kw("OPTION") {
            return Err(self.gap("query_option", self.offset()));
        }
        Ok(Expression::Delete(Box::new(node)))
    }

    /// `MERGE [INTO] target [[AS] alias] USING source ON condition WHEN ... ;`.
    pub(crate) fn merge_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("MERGE")?;
        self.eat_kw("INTO");
        let mut target: TableRef = self.dml_target()?;
        let explicit: bool = self.kw("AS");
        if !self.kw("USING") {
            target.alias = self.table_alias()?;
            target.alias_explicit_as = explicit;
        }
        self.expect_kw("USING")?;
        let (using, joins) = self.single_source()?;
        if !joins.is_empty() {
            return Err(self.gap("merge_using_join", self.offset()));
        }
        self.expect_kw("ON")?;
        let on: Expression = self.condition()?;
        let mut whens: Vec<Expression> = Vec::new();
        while self.eat_kw("WHEN") {
            let matched: bool = !self.eat_kw("NOT");
            self.expect_kw("MATCHED")?;
            let source: Option<Box<Expression>> = match (matched, self.kw("BY")) {
                (false, true) if self.kw_at(1, "TARGET") => {
                    self.at += 2;
                    Some(boolean(false))
                }
                (false, true) if self.kw_at(1, "SOURCE") => {
                    self.at += 2;
                    Some(boolean(true))
                }
                (_, true) => return Err(self.error()),
                _ => None,
            };
            let condition: Option<Box<Expression>> = match self.eat_kw("AND") {
                true => Some(Box::new(self.condition()?)),
                false => None,
            };
            self.expect_kw("THEN")?;
            let inserts: bool = !matched
                && source
                    .as_deref()
                    .is_none_or(|by| matches!(by, Expression::Boolean(boolean) if !boolean.value));
            let then: Expression = match self.word_at(0).as_deref() {
                Some("UPDATE") if !inserts => {
                    self.at += 1;
                    self.expect_kw("SET")?;
                    let assignments: Vec<Expression> = self.comma_list(|parser| {
                        let first: usize = parser.at;
                        let parts: Vec<Identifier> = parser.multipart(4)?;
                        parser.expect_char('=')?;
                        let value: Expression = parser.scalar()?;
                        Ok(parser.binary(first, Expression::Eq, column_from(parts), value))
                    })?;
                    tuple(vec![var("UPDATE"), tuple(assignments)])
                }
                Some("DELETE") if !inserts => {
                    self.at += 1;
                    var("DELETE")
                }
                Some("INSERT") if inserts => {
                    self.at += 1;
                    let mut elements: Vec<Expression> = vec![var("INSERT")];
                    if self.is_char('(') {
                        let columns: Vec<Identifier> = self.parenthesized(Parser::name)?;
                        elements.push(tuple(
                            columns
                                .into_iter()
                                .map(|column| column_from(vec![column]))
                                .collect(),
                        ));
                    }
                    if self.kw("DEFAULT") && self.kw_at(1, "VALUES") {
                        self.at += 2;
                        elements.push(var("DEFAULT VALUES"));
                    } else {
                        self.expect_kw("VALUES")?;
                        elements.push(tuple(self.parenthesized(Parser::scalar)?));
                    }
                    tuple(elements)
                }
                _ => return Err(self.error()),
            };
            whens.push(Expression::When(Box::new(When {
                matched: Some(boolean(matched)),
                source,
                condition,
                then: Box::new(then),
            })));
        }
        if whens.is_empty() {
            return Err(self.error());
        }
        if self.kw("OUTPUT") {
            return Err(self.gap("output", self.offset()));
        }
        if self.kw("OPTION") {
            return Err(self.gap("query_option", self.offset()));
        }
        if !self.is_char(';') {
            return Err(
                self.error_with("A MERGE statement must be terminated by a semi-colon (;).")
            );
        }
        Ok(Expression::Merge(Box::new(Merge {
            this: Box::new(Expression::Table(Box::new(target))),
            using: Box::new(using),
            on: Some(Box::new(on)),
            using_cond: None,
            whens: Some(Box::new(Expression::Whens(Box::new(Whens {
                expressions: whens,
            })))),
            with_: None,
            returning: None,
            returning_aliases: Vec::new(),
        })))
    }

    /// The `FROM` sources of `UPDATE` and `DELETE`.
    fn read_sources(&mut self) -> Parsing<(From, Vec<Join>)> {
        let (first, joins) = self.single_source()?;
        let mut expressions: Vec<Expression> = vec![first];
        while self.eat_char(',') {
            let (next, more) = self.single_source()?;
            expressions.push(match more.is_empty() {
                true => next,
                false => crate::parse::_helpers::tree::nodes::joined_table(next, more, None),
            });
        }
        Ok((From { expressions }, joins))
    }
}
