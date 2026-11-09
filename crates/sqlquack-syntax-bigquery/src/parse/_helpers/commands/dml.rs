//! `insertInto` and `dmlStatementNoWith` (SqlBaseParser.g4): `INSERT`, `MERGE`, `UPDATE`, `DELETE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::queries::models::{Where, With};
use sqlquack_syntax::expressions::statements::models::{Merge, When, Whens};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    TableAlias, binary, boolean, call, column_from, delete, insert, joined, star, subquery, table,
    tuple, update, var,
};
use crate::parse::constants::IDENTIFIER_CLAUSE;
use crate::scan::models::TokenKind;

/// Which `WHEN` clauses a `MERGE` may still take: matched, then not matched, then by source.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MergeStage {
    Matched,
    NotMatched,
    BySource,
}

/// `name` in backquotes, escaped.
fn requoted(name: &str) -> String {
    format!("`{}`", name.replace('\\', "\\\\").replace('`', "\\`"))
}

/// An `UPDATE` key's path as one name, requoting parts so differently quoted paths keep apart.
fn set_key(parts: Vec<Identifier>) -> Identifier {
    let whole: bool = parts.len() > 1
        && parts
            .windows(2)
            .all(|pair| pair[0].quoted && pair[0].span.is_some() && pair[0].span == pair[1].span);
    if whole {
        let token: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
        return Identifier::new(requoted(&token.join(".")));
    }
    let special = |part: &Identifier| part.quoted && part.name.contains(['.', '`', '\\']);
    if parts.len() == 1 || !parts.iter().any(special) {
        return joined(parts);
    }
    let names: Vec<String> = parts
        .iter()
        .map(|part| match special(part) {
            true => requoted(&part.name),
            false => part.name.clone(),
        })
        .collect();
    Identifier::new(names.join("."))
}

fn with_alias(mut relation: TableRef, alias: Option<TableAlias>) -> TableRef {
    if let Some(alias) = alias {
        relation.alias = Some(alias.name);
        relation.alias_explicit_as = alias.explicit;
        relation.column_aliases = alias.columns;
    }
    relation
}

impl Parser<'_> {
    /// `identifierReference` as a table: a name, or an `IDENTIFIER (...)` clause.
    pub(crate) fn table_reference(&mut self) -> Parsing<TableRef> {
        if self.kw("IDENTIFIER") && self.char_at(1, '(') {
            self.at += 2;
            let name: Expression = self.expression()?;
            self.expect_char(')')?;
            let mut relation: TableRef = TableRef::new(String::new());
            relation.identifier_func = Some(Box::new(call(IDENTIFIER_CLAUSE, vec![name])));
            return Ok(relation);
        }
        Ok(table(self.multipart_identifier()?))
    }

    /// `WITH (options)` after an `INSERT` target, as a gap.
    fn insert_options(&mut self) -> Parsing<()> {
        if self.kw("WITH") && self.char_at(1, '(') {
            let start: usize = self.offset();
            self.at += 1;
            self.property_list()?;
            self.gap("insert_options", start);
        }
        Ok(())
    }

    /// `partitionSpec`: `PARTITION ( name [= constant | = DEFAULT], ... )`.
    fn partition_spec(&mut self) -> Parsing<Vec<(Identifier, Option<Expression>)>> {
        self.expect_kw("PARTITION")?;
        self.parenthesized(|parser| {
            let name: Identifier = parser.ident()?;
            if !parser.eat_char('=') {
                return Ok((name, None));
            }
            let value: Expression = match parser.eat_kw("DEFAULT") {
                true => var("DEFAULT"),
                false => parser.expression()?,
            };
            Ok((name, Some(value)))
        })
    }

    /// `singleInsertQuery`: `insertInto` and its query, or `( query ) alias`.
    pub(crate) fn insert_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("INSERT")?;
        let overwrite: bool = false;
        self.eat_kw("INTO");
        if overwrite && (self.kw("DIRECTORY") || (self.kw("LOCAL") && self.kw_at(1, "DIRECTORY"))) {
            let start: usize = self.offset();
            self.skip_statement_rest()?;
            self.gap("insert_overwrite_directory", start);
            return Ok(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ));
        }
        self.eat_kw("TABLE");
        let mut node = insert(self.table_reference()?);
        node.overwrite = overwrite;
        node.with = with;
        let alias: Option<TableAlias> = match overwrite {
            true => None,
            false => self.table_alias()?,
        };
        self.insert_options()?;
        if self.kw("PARTITION") && self.char_at(1, '(') {
            node.partition = self.partition_spec()?;
        }
        if self.kw("IF") && self.kw_at(1, "NOT") && self.kw_at(2, "EXISTS") {
            self.at += 3;
            node.if_exists = true;
        }
        if self.kw("BY") && self.kw_at(1, "NAME") {
            self.at += 2;
            node.by_name = true;
        } else if self.is_char('(')
            && self.is_at(1, crate::parse::_helpers::tree::cursor::is_identifier)
            && (self.char_at(2, ',') || self.char_at(2, ')'))
        {
            node.columns = self.identifier_list()?;
        }
        let alias: Option<TableAlias> = match alias.is_none()
            && !overwrite
            && self.bare_alias_at(0, true)
            && self.kw_at(1, "REPLACE")
        {
            true => self.table_alias()?,
            false => alias,
        };
        if !overwrite && self.kw("REPLACE") {
            self.at += 1;
            let start: usize = self.offset();
            if self.eat_kw("WHERE") {
                node.replace_where = Some(Box::new(self.expression()?));
            } else if self.eat_kw("ON") {
                self.expression()?;
                self.gap("insert_replace_on", start);
            } else {
                self.expect_kw("USING")?;
                self.identifier_list()?;
                self.gap("insert_replace_using", start);
            }
        } else if alias.is_some() {
            return Err(self.error());
        }
        if let Some(alias) = alias {
            node.alias = Some(alias.name);
            node.alias_explicit_as = alias.explicit;
        }
        if self.eat_kw("VALUES") {
            node.values = self.comma_list(|parser| parser.parenthesized(|row| row.dml_value()))?;
            return Ok(Expression::Insert(Box::new(node)));
        }
        match self.query()? {
            Expression::Values(rows) if rows.alias.is_none() => {
                node.values = rows
                    .expressions
                    .into_iter()
                    .map(|row| row.expressions)
                    .collect();
            }
            other => node.query = Some(other),
        }
        Ok(Expression::Insert(Box::new(node)))
    }

    /// `assignmentList`: `key = value, ...`, each as an equality of a column and its value.
    fn assignments(&mut self) -> Parsing<Vec<Expression>> {
        self.comma_list(|parser| {
            let key: Expression = column_from(parser.multipart_identifier()?);
            parser.expect_char('=')?;
            Ok(binary(Expression::Eq, key, parser.dml_value()?))
        })
    }

    /// `expression_or_default`: a DML value, or `DEFAULT` for the column's default.
    fn dml_value(&mut self) -> Parsing<Expression> {
        match self.eat_kw("DEFAULT") {
            true => Ok(var("DEFAULT")),
            false => self.expression(),
        }
    }

    /// `mergeIntoTable`.
    pub(crate) fn merge_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("MERGE")?;
        self.eat_kw("INTO");
        let target_name: TableRef = self.table_reference()?;
        let target: TableRef = with_alias(target_name, self.table_alias()?);
        self.expect_kw("USING")?;
        let source: Expression = match self.is_char('(') {
            true => {
                self.at += 1;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                subquery(query, self.table_alias()?, false)
            }
            false => {
                let relation: TableRef = self.table_reference()?;
                Expression::Table(Box::new(with_alias(relation, self.table_alias()?)))
            }
        };
        self.expect_kw("ON")?;
        let on: Expression = self.expression()?;
        let mut whens: Vec<Expression> = Vec::new();
        while self.kw("WHEN") {
            let (when, _) = self.merge_when()?;
            whens.push(when);
        }
        if whens.is_empty() {
            return Err(self.error_with(
                "There must be at least one WHEN clause in a MERGE statement",
                self.offset(),
                true,
            ));
        }
        Ok(Expression::Merge(Box::new(Merge {
            returning_aliases: Vec::new(),
            this: Box::new(Expression::Table(Box::new(target))),
            using: Box::new(source),
            on: Some(Box::new(on)),
            using_cond: None,
            whens: match whens.is_empty() {
                true => None,
                false => Some(Box::new(Expression::Whens(Box::new(Whens {
                    expressions: whens,
                })))),
            },
            with_: with.map(|with| Box::new(Expression::With(Box::new(with)))),
            returning: None,
        })))
    }

    /// One `WHEN` clause of a `MERGE`, and the stage it belongs to.
    fn merge_when(&mut self) -> Parsing<(Expression, MergeStage)> {
        self.expect_kw("WHEN")?;
        let not: bool = self.eat_kw("NOT") || self.eat_char('!');
        self.expect_kw("MATCHED")?;
        let mut source: Option<bool> = None;
        if not && self.eat_kw("BY") {
            source = Some(match self.word_at(0).as_deref() {
                Some("SOURCE") => true,
                Some("TARGET") => false,
                _ => return Err(self.error()),
            });
            self.at += 1;
        }
        let stage: MergeStage = match (not, source) {
            (false, _) => MergeStage::Matched,
            (true, Some(true)) => MergeStage::BySource,
            (true, _) => MergeStage::NotMatched,
        };
        let condition: Option<Box<Expression>> = match self.eat_kw("AND") {
            true => Some(Box::new(self.expression()?)),
            false => None,
        };
        self.expect_kw("THEN")?;
        let then: Expression = match (stage, self.word_at(0).as_deref()) {
            (MergeStage::Matched | MergeStage::BySource, Some("DELETE")) => {
                self.at += 1;
                var("DELETE")
            }
            (MergeStage::Matched | MergeStage::BySource, Some("UPDATE")) => {
                self.at += 1;
                self.expect_kw("SET")?;
                if stage == MergeStage::Matched && self.eat_char('*') {
                    tuple(vec![
                        var("UPDATE"),
                        Expression::Star(Box::new(star(None, None))),
                    ])
                } else {
                    tuple(vec![var("UPDATE"), tuple(self.assignments()?)])
                }
            }
            (MergeStage::NotMatched, Some("INSERT")) => {
                self.at += 1;
                if self.eat_kw("ROW") {
                    tuple(vec![
                        var("INSERT"),
                        Expression::Star(Box::new(star(None, None))),
                    ])
                } else if self.kw("VALUES") {
                    self.at += 1;
                    let values: Vec<Expression> =
                        self.parenthesized(|parser| parser.dml_value())?;
                    tuple(vec![var("INSERT"), tuple(Vec::new()), tuple(values)])
                } else if self.eat_char('*') {
                    tuple(vec![
                        var("INSERT"),
                        Expression::Star(Box::new(star(None, None))),
                    ])
                } else {
                    let columns: Vec<Expression> = self
                        .parenthesized(|parser| Ok(column_from(parser.multipart_identifier()?)))?;
                    if self.eat_kw("ROW") {
                        tuple(vec![
                            var("INSERT"),
                            tuple(columns),
                            Expression::Star(Box::new(star(None, None))),
                        ])
                    } else {
                        self.expect_kw("VALUES")?;
                        let values: Vec<Expression> =
                            self.parenthesized(|parser| parser.dml_value())?;
                        tuple(vec![var("INSERT"), tuple(columns), tuple(values)])
                    }
                }
            }
            _ => return Err(self.error()),
        };
        let when = Expression::When(Box::new(When {
            matched: Some(Box::new(boolean(!not))),
            source: source.map(|source| Box::new(boolean(source))),
            condition,
            then: Box::new(then),
        }));
        Ok((when, stage))
    }

    /// `updateTable`: `UPDATE t [alias] SET assignments [WHERE ...]`.
    pub(crate) fn update_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("UPDATE")?;
        let relation: TableRef = self.table_reference()?;
        let mut node = update(with_alias(relation, self.table_alias()?));
        self.expect_kw("SET")?;
        node.set = self.comma_list(|parser| {
            let whole: Option<Identifier> = match parser.peek() {
                Some(token)
                    if token.kind == TokenKind::Quoted
                        && token.value.contains('.')
                        && !parser.char_at(1, '.') =>
                {
                    Some(Identifier::quoted(token.value.clone()))
                }
                _ => None,
            };
            let key: Identifier = match whole {
                Some(name) => {
                    parser.at += 1;
                    name
                }
                None => set_key(parser.multipart_identifier()?),
            };
            parser.expect_char('=')?;
            Ok((key, parser.dml_value()?))
        })?;
        if self.kw("FROM") {
            let select = self
                .select_from_clause(sqlquack_syntax::expressions::queries::models::Select::new())?;
            node.from_clause = select.from;
            node.from_joins = select.joins;
        }
        if self.eat_kw("WHERE") {
            node.where_clause = Some(Where {
                this: self.expression()?,
            });
        }
        node.with = with;
        Ok(Expression::Update(Box::new(node)))
    }

    /// `deleteFromTable`: `DELETE FROM t [alias] [WHERE ...]`.
    pub(crate) fn delete_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("DELETE")?;
        self.eat_kw("FROM");
        let relation: TableRef = self.table_reference()?;
        let mut node = delete(relation);
        if let Some(alias) = self.table_alias()? {
            node.alias = Some(alias.name);
            node.alias_explicit_as = alias.explicit;
        }
        if self.eat_kw("WHERE") {
            node.where_clause = Some(Where {
                this: self.expression()?,
            });
        }
        node.with = with;
        Ok(Expression::Delete(Box::new(node)))
    }
}
