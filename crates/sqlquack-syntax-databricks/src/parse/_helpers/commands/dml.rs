//! `insertInto` and `dmlStatementNoWith` (SqlBaseParser.g4): `INSERT`, `MERGE`, `UPDATE`, `DELETE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::queries::models::{Where, With};
use sqlquack_syntax::expressions::statements::models::{
    DirectoryInsert, Insert, Merge, When, Whens,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    TableAlias, binary, boolean, call, column_from, default_value, delete, insert, joined, star,
    subquery, table, tuple, update, var,
};
use crate::parse::constants::IDENTIFIER_CLAUSE;

/// Which `WHEN` clauses a `MERGE` may still take: matched, then not matched, then by source.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MergeStage {
    Matched,
    NotMatched,
    BySource,
}

/// The insert's source: a bare `VALUES` list as its rows, any other query as its query.
fn with_source(mut node: Insert, query: Expression) -> Insert {
    match query {
        Expression::Values(rows) if rows.alias.is_none() => {
            node.values = rows
                .expressions
                .into_iter()
                .map(|row| row.expressions.into_iter().map(default_value).collect())
                .collect();
        }
        other => node.query = Some(other),
    }
    node
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

    /// Whether `WITH SCHEMA EVOLUTION` follows `INSERT` or `MERGE`.
    fn schema_evolution(&mut self) -> bool {
        self.eat_kws(&["WITH", "SCHEMA", "EVOLUTION"])
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
    pub(crate) fn partition_spec(&mut self) -> Parsing<Vec<(Identifier, Option<Expression>)>> {
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

    /// `[LOCAL] DIRECTORY …` after `INSERT OVERWRITE`, in Spark's or Hive's form.
    fn overwrite_directory(&mut self) -> Parsing<DirectoryInsert> {
        let start: usize = self.offset();
        let local: bool = self.eat_kw("LOCAL");
        self.expect_kw("DIRECTORY")?;
        let path: bool = self.string_at(0);
        let mut node: DirectoryInsert = DirectoryInsert {
            local,
            path: match path {
                true => self.string_lit()?,
                false => String::new(),
            },
            row_format: None,
            stored_as: None,
            input_format: None,
            output_format: None,
            using: None,
            options: Vec::new(),
            options_without_equals: Vec::new(),
        };
        if !self.kw("USING") && path {
            if self.kw("ROW") {
                let row: usize = self.offset();
                node.row_format = self.row_format()?;
                if node.row_format.is_none() {
                    self.gap("directory_row_format", row);
                }
            }
            if self.kw("STORED") {
                return self.directory_file_format(node);
            }
            return Ok(node);
        }
        self.expect_kw("USING")?;
        node.using = Some(joined(self.multipart_identifier()?).name);
        if self.eat_kw("OPTIONS") {
            let spelled: Vec<((Expression, Expression), bool)> =
                self.spelled_valued_property_list()?;
            node.options_without_equals = (0..spelled.len())
                .filter(|position| spelled[*position].1)
                .collect();
            node.options = spelled.into_iter().map(|(pair, _)| pair).collect();
        }
        let path_option: bool = node.options.iter().any(|(key, _)| {
            crate::parse::_helpers::commands::alters::property_text(key)
                .eq_ignore_ascii_case("path")
        });
        if path == path_option {
            return Err(self.error_with(
                "Directory path and 'path' in OPTIONS should be specified one, but not both",
                start,
                true,
            ));
        }
        Ok(node)
    }

    /// Hive's `STORED AS` a format name or input and output classes; a storage handler is a gap.
    fn directory_file_format(&mut self, mut node: DirectoryInsert) -> Parsing<DirectoryInsert> {
        if !self.kw_at(1, "AS") {
            let start: usize = self.offset();
            self.create_file_format()?;
            self.gap("directory_storage_handler", start);
            return Ok(node);
        }
        self.at += 2;
        if self.eat_kw("INPUTFORMAT") {
            node.input_format = Some(self.string_lit()?);
            self.expect_kw("OUTPUTFORMAT")?;
            node.output_format = Some(self.string_lit()?);
        } else {
            node.stored_as = Some(self.ident()?.name);
        }
        Ok(node)
    }

    /// `singleInsertQuery`: `insertInto` and its query, or `( query ) alias`.
    pub(crate) fn insert_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("INSERT")?;
        let schema_evolution: bool = self.schema_evolution();
        let overwrite: bool = self.eat_kw("OVERWRITE");
        if !overwrite {
            self.expect_kw("INTO")?;
        }
        if overwrite && (self.kw("DIRECTORY") || (self.kw("LOCAL") && self.kw_at(1, "DIRECTORY"))) {
            let mut node = insert(TableRef::new(String::new()));
            node.overwrite = true;
            node.with = with;
            node.schema_evolution = schema_evolution;
            node.directory = Some(self.overwrite_directory()?);
            let query: Expression = self.query()?;
            return Ok(Expression::Insert(Box::new(with_source(node, query))));
        }
        let table_keyword: bool = self.eat_kw("TABLE");
        let mut node = insert(self.table_reference()?);
        node.schema_evolution = schema_evolution;
        node.table_keyword = table_keyword;
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
            if self.eat_kw("WHERE") {
                node.replace_where = Some(Box::new(self.expression()?));
            } else if self.eat_kw("ON") {
                node.replace_on = Some(Box::new(self.expression()?));
            } else {
                self.expect_kw("USING")?;
                node.replace_using = self.identifier_list()?;
            }
        } else if alias.is_some() {
            return Err(self.error());
        }
        if let Some(alias) = alias {
            node.alias = Some(alias.name);
            node.alias_explicit_as = alias.explicit;
        }
        let query: Expression = match self.is_char('(') && self.starts_query_at(1) {
            true => {
                self.at += 1;
                let query: Expression = self.query()?;
                self.expect_char(')')?;
                let alias: Option<TableAlias> = self.table_alias()?;
                match alias {
                    Some(alias) => subquery(query, Some(alias), false),
                    None if node.replace_on.is_some() => subquery(query, None, false),
                    None => query,
                }
            }
            false => self.query()?,
        };
        Ok(Expression::Insert(Box::new(with_source(node, query))))
    }

    /// `assignmentList`: `key = value, ...`, each as an equality of a column and its value.
    fn assignments(&mut self) -> Parsing<Vec<Expression>> {
        self.comma_list(|parser| {
            let key: Expression = column_from(parser.multipart_identifier()?);
            parser.expect_char('=')?;
            Ok(binary(
                Expression::Eq,
                key,
                default_value(parser.expression()?),
            ))
        })
    }

    /// `mergeIntoTable`.
    pub(crate) fn merge_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("MERGE")?;
        let schema_evolution: bool = self.schema_evolution();
        self.expect_kw("INTO")?;
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
        let mut stage: MergeStage = MergeStage::Matched;
        let mut whens: Vec<Expression> = Vec::new();
        while self.kw("WHEN") {
            let (when, reached) = self.merge_when()?;
            if reached < stage {
                return Err(self.error());
            }
            stage = reached;
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
            schema_evolution,
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
                if self.eat_char('*') {
                    tuple(vec![
                        var("INSERT"),
                        Expression::Star(Box::new(star(None, None))),
                    ])
                } else {
                    let columns: Vec<Expression> = self
                        .parenthesized(|parser| Ok(column_from(parser.multipart_identifier()?)))?;
                    self.expect_kw("VALUES")?;
                    let values: Vec<Expression> =
                        self.parenthesized(|parser| parser.expression().map(default_value))?;
                    tuple(vec![var("INSERT"), tuple(columns), tuple(values)])
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
            let key: Identifier = joined(parser.multipart_identifier()?);
            parser.expect_char('=')?;
            Ok((key, default_value(parser.expression()?)))
        })?;
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
        self.expect_kw("FROM")?;
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
