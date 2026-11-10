//! Queries: `WITH`, `SELECT` and its clauses, set operations, and table sources.

use sqlquack_syntax::expressions::functions::models::{Function, OpenJSON, OpenJSONColumnDef};
use sqlquack_syntax::expressions::names::models::{
    BooleanLiteral, Identifier, Literal, Star, TableRef,
};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    Fetch, From, GroupBy, Having, Join, JoinKind, Offset, OrderBy, Sample, SampleMethod, Select,
    SelectInto, Top, Where, With,
};
use sqlquack_syntax::expressions::statements::models::Version;
use sqlquack_syntax::expressions::tables::models::{IndexTableHint, WithTableHint};
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::{SourceSpan, Span};
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_any_name, is_name};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, alias, cte, function, join, joined_table, literal, null, paren,
    set_operation, subquery, table, values, var,
};
use crate::parse::_helpers::values::primary::variable;
use crate::parse::constants::{
    COMPOUND_ASSIGNMENTS, CONFLICTING_LOCKS, ISOLATION_LEVELS, LOCK_GRANULARITIES,
    PARAMETERIZED_FORCESEEK, SPATIAL_WINDOW_MAX_CELLS, TABLE_HINTS, UNION_OPERATOR,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// Words that end a select item or a table source rather than naming its alias.
const NOT_ALIASES: [&str; 22] = [
    "CROSS",
    "OUTER",
    "APPLY",
    "INNER",
    "LEFT",
    "RIGHT",
    "FULL",
    "JOIN",
    "WHERE",
    "GROUP",
    "HAVING",
    "ORDER",
    "UNION",
    "EXCEPT",
    "INTERSECT",
    "OPTION",
    "FOR",
    "INTO",
    "FROM",
    "ON",
    "WINDOW",
    "OFFSET",
];

/// The physical join hints that may stand between `INNER`, or a side and its `OUTER`, and `JOIN`; `REMOTE` only after `INNER` (1072).
const JOIN_HINTS: [&str; 4] = ["LOOP", "HASH", "MERGE", "REMOTE"];

fn empty_star(table: Option<Identifier>) -> Star {
    Star {
        table,
        ilike: None,
        except: None,
        replace: None,
        rename: None,
        trailing_comments: Vec::new(),
        span: None,
        qualified: None,
    }
}

impl<'sql> Parser<'sql> {
    /// A whole query: `[WITH ...] body [ORDER BY ...] [OFFSET ... FETCH ...] [OPTION (...)]`.
    pub(crate) fn query_statement(&mut self) -> Parsing<Expression> {
        let with: Option<With> = self.with_clause()?;
        let mut query: Expression = self.query_expression()?;
        if let Some(with) = with {
            set_with(&mut query, with);
        }
        let query: Expression = self.browse_clause(query)?;
        self.query_tail()?;
        Ok(query)
    }

    /// The gap for a statement-level `OPTION (...)` after a query.
    pub(crate) fn query_tail(&mut self) -> Parsing<()> {
        if self.kw("OPTION") && self.char_at(1, '(') {
            return Err(self.gap("query_option", self.offset()));
        }
        Ok(())
    }

    /// `WITH name [(columns)] AS (query), ...`, when it comes next.
    pub(crate) fn with_clause(&mut self) -> Parsing<Option<With>> {
        if !self.eat_kw("WITH") {
            return Ok(None);
        }
        if self.kw("XMLNAMESPACES") {
            return Err(self.gap("xmlnamespaces", self.offset()));
        }
        let ctes = self.comma_list(|parser| {
            let name: Identifier = parser.name()?;
            let columns: Vec<Identifier> = match parser.is_char('(') {
                true => parser.parenthesized(Parser::name)?,
                false => Vec::new(),
            };
            parser.expect_kw("AS")?;
            parser.expect_char('(')?;
            let query: Expression = parser.query_expression()?;
            parser.expect_char(')')?;
            Ok(cte(name, columns, query))
        })?;
        Ok(Some(With {
            ctes,
            recursive: false,
            leading_comments: Vec::new(),
            search: None,
        }))
    }

    /// A query expression with its `ORDER BY`, `OFFSET`/`FETCH` and `FOR XML` or `FOR JSON`.
    pub(crate) fn query_expression(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let mut query: Expression = parser.union_expression()?;
            if parser.kw("ORDER") && parser.kw_at(1, "BY") {
                let order = parser.order_by_clause()?;
                let order_by: OrderBy = OrderBy {
                    expressions: order,
                    siblings: false,
                    comments: Vec::new(),
                    all: None,
                };
                let (offset, fetch) = parser.offset_fetch()?;
                let offset_node = |offset: Option<Offset>| {
                    offset.map(|found| Box::new(Expression::Offset(Box::new(found))))
                };
                match &mut query {
                    Expression::Select(select) => {
                        select.order_by = Some(order_by);
                        select.offset = offset;
                        select.fetch = fetch;
                    }
                    Expression::Union(node) if fetch.is_none() => {
                        node.order_by = Some(order_by);
                        node.offset = offset_node(offset);
                    }
                    Expression::Except(node) if fetch.is_none() => {
                        node.order_by = Some(order_by);
                        node.offset = offset_node(offset);
                    }
                    Expression::Intersect(node) if fetch.is_none() => {
                        node.order_by = Some(order_by);
                        node.offset = offset_node(offset);
                    }
                    _ => return Err(parser.gap("set_operation_order", parser.offset())),
                }
            }
            parser.xml_or_json_clause(query)
        })
    }

    /// `OFFSET n {ROW|ROWS} [FETCH {FIRST|NEXT} n {ROW|ROWS} ONLY]` after an `ORDER BY`.
    fn offset_fetch(&mut self) -> Parsing<(Option<Offset>, Option<Fetch>)> {
        if !self.eat_kw("OFFSET") {
            return Ok((None, None));
        }
        let this: Expression = self.scalar()?;
        if !(self.eat_kw("ROW") || self.eat_kw("ROWS")) {
            return Err(self.error());
        }
        let offset: Offset = Offset {
            this,
            rows: Some(true),
        };
        if !self.eat_kw("FETCH") {
            return Ok((Some(offset), None));
        }
        let direction: String = match self.word_at(0).as_deref() {
            Some(word @ ("FIRST" | "NEXT")) => word.to_owned(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        let count: Expression = self.scalar()?;
        if !(self.eat_kw("ROW") || self.eat_kw("ROWS")) {
            return Err(self.error());
        }
        self.expect_kw("ONLY")?;
        Ok((
            Some(offset),
            Some(Fetch {
                direction,
                count: Some(count),
                percent: false,
                rows: true,
                with_ties: false,
            }),
        ))
    }

    /// `UNION [ALL]` and `EXCEPT` over `INTERSECT`, which binds tighter.
    fn union_expression(&mut self) -> Parsing<Expression> {
        let mut left: Expression = self.intersect_expression()?;
        let depth: usize = self.depth();
        loop {
            let operator: &str = match self.word_at(0).as_deref() {
                Some("UNION") => "UNION",
                Some("EXCEPT") => "EXCEPT",
                _ => {
                    self.restore_depth(depth);
                    return Ok(left);
                }
            };
            self.link()?;
            self.at += 1;
            let all: bool = operator == UNION_OPERATOR && self.eat_kw("ALL");
            let right: Expression = self.intersect_expression()?;
            left = set_operation(operator, left, right, (all, false));
        }
    }

    fn intersect_expression(&mut self) -> Parsing<Expression> {
        let mut left: Expression = self.query_term()?;
        let depth: usize = self.depth();
        while self.eat_kw("INTERSECT") {
            self.link()?;
            let right: Expression = self.query_term()?;
            left = set_operation("INTERSECT", left, right, (false, false));
        }
        self.restore_depth(depth);
        Ok(left)
    }

    /// `SELECT ...` or `( query )`.
    fn query_term(&mut self) -> Parsing<Expression> {
        if self.eat_char('(') {
            let query: Expression = self.query_expression()?;
            self.expect_char(')')?;
            return Ok(subquery(query, None, false));
        }
        self.select()
    }

    fn select(&mut self) -> Parsing<Expression> {
        self.expect_kw("SELECT")?;
        let mut select: Select = Select::new();
        if self.eat_kw("DISTINCT") {
            select.distinct = true;
        } else if self.eat_kw("ALL") {
            select.all = true;
        }
        if self.eat_kw("TOP") {
            select.top = Some(self.top()?);
        }
        select.expressions = self.comma_list(Self::select_item)?;
        if self.eat_kw("INTO") {
            let name: Vec<Identifier> = self.multipart(4)?;
            if self.kw("ON") {
                return Err(self.gap("select_into_filegroup", self.offset()));
            }
            let temporary: bool = name.last().is_some_and(|part| part.name.starts_with('#'));
            select.into = Some(SelectInto {
                this: Expression::Table(Box::new(table(name))),
                temporary,
                unlogged: false,
                bulk_collect: false,
                expressions: Vec::new(),
            });
        }
        if self.eat_kw("FROM") {
            let mut sources: Vec<Expression> = Vec::new();
            let mut joins: Vec<Join> = Vec::new();
            loop {
                let source: Expression = self.table_source()?;
                if sources.is_empty() {
                    sources.push(source);
                    joins = self.joins()?;
                } else {
                    let more: Vec<Join> = self.joins()?;
                    sources.push(match more.is_empty() {
                        true => source,
                        false => joined_table(source, more, None),
                    });
                }
                if !self.eat_char(',') {
                    break;
                }
            }
            select.from = Some(From {
                expressions: sources,
            });
            select.joins = joins;
        }
        if self.eat_kw("WHERE") {
            select.where_clause = Some(Where {
                this: self.condition()?,
            });
        }
        if self.kw("GROUP") && self.kw_at(1, "BY") {
            self.at += 2;
            if self.kw_in(0, &["ROLLUP", "CUBE", "GROUPING", "ALL"])
                || (self.is_char('(') && self.char_at(1, ')'))
            {
                return Err(self.gap("group_by_extension", self.offset()));
            }
            let expressions: Vec<Expression> = self.comma_list(Self::scalar)?;
            if self.kw("WITH") && self.kw_in(1, &["CUBE", "ROLLUP"]) {
                return Err(self.gap("group_by_extension", self.offset()));
            }
            select.group_by = Some(GroupBy {
                expressions,
                all: None,
                totals: false,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("HAVING") {
            select.having = Some(Having {
                this: self.condition()?,
                comments: Vec::new(),
            });
        }
        if self.kw("WINDOW") && self.is_at(1, is_name) {
            return Err(self.gap("window_clause", self.offset()));
        }
        Ok(Expression::Select(Box::new(select)))
    }

    /// `TOP (n) [PERCENT] [WITH TIES]`, or `TOP n` with a literal.
    fn top(&mut self) -> Parsing<Top> {
        let (this, parenthesized) = match self.eat_char('(') {
            true => {
                let open: usize = self.at - 1;
                let query: Option<Parsing<Expression>> =
                    (!self.not_subquery.contains(&open) && self.starts_query()).then(|| {
                        self.attempt(|parser| {
                            let query: Expression = parser.query_expression()?;
                            match parser.is_char(')') {
                                true => Ok(subquery(query, None, false)),
                                false => Err(parser.error()),
                            }
                        })
                    });
                let this: Expression = match query {
                    Some(Ok(node)) => node,
                    Some(Err(error)) if self.gapped() || !error.syntax => return Err(error),
                    Some(Err(_)) => {
                        self.not_subquery.insert(open);
                        self.scalar()?
                    }
                    None => self.scalar()?,
                };
                self.expect_char(')')?;
                (this, true)
            }
            false => {
                let token: Token =
                    self.expect_token(|token| matches!(token.kind, TokenKind::Number(_)))?;
                (literal(Literal::Number(token.value)), false)
            }
        };
        let percent: bool = self.eat_kw("PERCENT");
        let with_ties: bool = self.eat_kws(&["WITH", "TIES"]);
        Ok(Top {
            this,
            percent,
            with_ties,
            parenthesized,
        })
    }

    /// One select item: `*`, `t.*`, `expr [[AS] alias]`, `alias = expr` or `@v = expr`.
    fn select_item(&mut self) -> Parsing<Expression> {
        if self.eat_char('*') {
            return Ok(Expression::Star(Box::new(empty_star(None))));
        }
        let mut offset: usize = 0;
        while self.is_at(offset, is_name) && self.char_at(offset + 1, '.') {
            offset += 2;
        }
        if offset > 0 && self.char_at(offset, '*') {
            let parts: Vec<Identifier> = (0..offset / 2)
                .map(|index| identifier(&self.tokens[self.at + index * 2]))
                .collect();
            self.at += offset + 1;
            let qualifier: Identifier = crate::parse::_helpers::tree::nodes::joined(parts);
            return Ok(Expression::Star(Box::new(empty_star(Some(qualifier)))));
        }
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable)
            && (self.char_at(1, '=') || self.assignment_at(1))
        {
            let target: Token = self.advance()?;
            let operator: String = self.advance()?.value;
            let value: Expression = self.scalar()?;
            return Ok(Expression::Operator(Box::new(
                sqlquack_syntax::expressions::functions::models::Operator {
                    this: Box::new(variable(&target)),
                    operator: Some(Box::new(var(&operator))),
                    expression: Box::new(value),
                    comments: Vec::new(),
                },
            )));
        }
        let alias_first: bool = self.char_at(1, '=')
            && self.peek().is_some_and(|token| {
                is_name(token) || matches!(token.kind, TokenKind::String { national: false })
            });
        if alias_first {
            let token: Token = self.advance()?;
            self.at += 1;
            let value: Expression = self.scalar()?;
            let name: Identifier = match token.kind {
                TokenKind::String { .. } => Identifier::quoted(token.value).with_span(token.span),
                _ => identifier(&token),
            };
            return Ok(alias(value, name, false));
        }
        let value: Expression = self.scalar()?;
        let explicit: bool = self.eat_kw("AS");
        let alias_name: Option<Identifier> = match self.peek() {
            Some(token) if matches!(token.kind, TokenKind::String { national: false }) => {
                let token: Token = self.advance()?;
                Some(Identifier::quoted(token.value).with_span(token.span))
            }
            Some(token) if is_name(token) && (explicit || !self.not_alias()) => {
                Some(identifier(&self.advance()?))
            }
            _ if explicit => return Err(self.error()),
            _ => None,
        };
        Ok(match alias_name {
            Some(name) => alias(value, name, explicit),
            None => value,
        })
    }

    /// Whether a compound assignment operator is `offset` tokens ahead.
    pub(crate) fn assignment_at(&self, offset: usize) -> bool {
        self.peek_at(offset).is_some_and(|token| {
            token.kind == TokenKind::Op && COMPOUND_ASSIGNMENTS.contains(&token.value.as_str())
        })
    }

    /// Whether the word at the cursor ends the item instead of naming an alias.
    fn not_alias(&self) -> bool {
        self.word_at(0)
            .is_some_and(|word| NOT_ALIASES.contains(&word.as_str()))
    }

    /// An optional alias for a table source: `[AS] name`.
    pub(crate) fn table_alias(&mut self) -> Parsing<Option<Identifier>> {
        let explicit: bool = self.eat_kw("AS");
        if self.is(is_name) && (explicit || !self.not_alias()) {
            return Ok(Some(identifier(&self.advance()?)));
        }
        match explicit {
            true => Err(self.error()),
            false => Ok(None),
        }
    }

    /// A table source, then any `PIVOT` or `UNPIVOT`; only a table name takes `TABLESAMPLE` (156).
    fn table_source(&mut self) -> Parsing<Expression> {
        let source: Expression = self.deeper(|parser| parser.table_primary())?;
        if self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
            return Err(self.gap("pivot", self.offset()));
        }
        if self.kw("TABLESAMPLE") {
            return Err(self.error());
        }
        Ok(source)
    }

    /// Whether a derived table's alias follows one of these parentheses, as in `((SELECT 1 x) d JOIN t ON 1 = 1)`.
    fn aliased_derived_table(&self) -> bool {
        let opened: usize = self.tokens[self.at..]
            .iter()
            .take_while(|token| token.kind == TokenKind::Char('('))
            .count();
        if opened == 0 {
            return false;
        }
        let (mut depth, mut lowest): (usize, usize) = (0, opened);
        for (offset, token) in self.tokens[self.at..].iter().enumerate() {
            match token.kind {
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') => {
                    depth -= 1;
                    if depth < lowest {
                        lowest = depth;
                        if self.kw_at(offset + 1, "AS") || self.is_at(offset + 1, is_name) {
                            return true;
                        }
                        if depth == 0 {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// A table source; its parentheses hold a query or a joined table, never a lone table (102).
    fn table_primary(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.is_char('(') {
            self.at += 1;
            if self.kw_in(0, &["MERGE", "INSERT", "UPDATE", "DELETE"]) {
                return Err(self.gap("composable_dml", start));
            }
            if self.starts_query() && !self.aliased_derived_table() {
                let query: Expression = self.query_expression()?;
                self.expect_char(')')?;
                let explicit: bool = self.kw("AS");
                let name: Identifier = match self.table_alias()? {
                    Some(name) => name,
                    None => return Err(self.error()),
                };
                let columns: Vec<Identifier> = match self.is_char('(') {
                    true => self.parenthesized(Parser::name)?,
                    false => Vec::new(),
                };
                return Ok(subquery(
                    query,
                    Some(TableAlias {
                        name,
                        columns,
                        explicit,
                    }),
                    false,
                ));
            }
            if self.kw("VALUES") {
                self.at += 1;
                let rows = self.comma_list(|parser| parser.parenthesized(Parser::scalar))?;
                self.expect_char(')')?;
                let explicit: bool = self.kw("AS");
                let name: Identifier = match self.table_alias()? {
                    Some(name) => name,
                    None => return Err(self.error()),
                };
                let columns: Vec<Identifier> = match self.is_char('(') {
                    true => self.parenthesized(Parser::name)?,
                    false => Vec::new(),
                };
                return Ok(values(
                    rows,
                    Some(TableAlias {
                        name,
                        columns,
                        explicit,
                    }),
                ));
            }
            let left: Expression = self.table_source()?;
            let joins: Vec<Join> = self.joins()?;
            self.expect_char(')')?;
            return match (joins.is_empty(), &left) {
                (false, _) => Ok(joined_table(left, joins, None)),
                (true, Expression::JoinedTable(_) | Expression::Paren(_)) => Ok(paren(left)),
                (true, _) => Err(self.error()),
            };
        }
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable)
        {
            let mut reference: TableRef = table(vec![identifier(&self.advance()?)]);
            let explicit: bool = self.kw("AS");
            reference.alias = self.table_alias()?;
            reference.alias_explicit_as = explicit;
            return Ok(Expression::Table(Box::new(reference)));
        }
        if self.kw("OPENJSON") && self.char_at(1, '(') {
            return self.open_json();
        }
        if self.kw("PREDICT") && self.char_at(1, '(') {
            return Err(self.gap("predict", start));
        }
        if self.kw_in(
            0,
            &[
                "OPENROWSET",
                "OPENQUERY",
                "OPENXML",
                "OPENDATASOURCE",
                "CONTAINSTABLE",
                "FREETEXTTABLE",
            ],
        ) {
            return Err(self.gap("rowset_function", start));
        }
        let name: Vec<Identifier> = self.multipart(4)?;
        if self.is_char('(') {
            return self.table_function_call(name);
        }
        let mut reference: TableRef = table(name);
        if self.kw("FOR") && self.kw_at(1, "SYSTEM_TIME") {
            reference.version = Some(Box::new(self.system_time()?));
        }
        let explicit: bool = self.kw("AS");
        reference.alias = self.table_alias()?;
        reference.alias_explicit_as = explicit;
        if self.kw("TABLESAMPLE") {
            reference.table_sample = Some(Box::new(self.table_sample()?));
        }
        let old_style_hint: Option<String> = self
            .word_at(1)
            .filter(|word| TABLE_HINTS.contains(&word.as_str()));
        if reference.alias.is_some()
            && self.char_at(0, '(')
            && self.char_at(2, ')')
            && let Some(word) = old_style_hint
        {
            self.at += 3;
            reference.hints = vec![Expression::WithTableHint(Box::new(WithTableHint {
                expressions: vec![var(&word)],
            }))];
        } else if self.kw("WITH") && self.char_at(1, '(') {
            reference.hints = vec![self.table_hints(&[])?];
        }
        Ok(Expression::Table(Box::new(reference)))
    }

    /// `FOR SYSTEM_TIME { AS OF b | FROM b TO b | BETWEEN b AND b | CONTAINED IN (b, b) | ALL }`, the cursor at `FOR`.
    fn system_time(&mut self) -> Parsing<Version> {
        self.at += 2;
        let (kind, bounds): (&str, Vec<Expression>) = if self.eat_kw("ALL") {
            ("ALL", Vec::new())
        } else if self.kw("AS") && self.kw_at(1, "OF") {
            self.at += 2;
            ("AS OF", vec![self.system_time_bound()?])
        } else if self.eat_kw("FROM") {
            let from: Expression = self.system_time_bound()?;
            self.expect_kw("TO")?;
            ("FROM", vec![from, self.system_time_bound()?])
        } else if self.eat_kw("BETWEEN") {
            let from: Expression = self.system_time_bound()?;
            self.expect_kw("AND")?;
            ("BETWEEN", vec![from, self.system_time_bound()?])
        } else if self.kw("CONTAINED") && self.kw_at(1, "IN") {
            self.at += 2;
            self.expect_char('(')?;
            let from: Expression = self.system_time_bound()?;
            self.expect_char(',')?;
            let to: Expression = self.system_time_bound()?;
            self.expect_char(')')?;
            ("CONTAINED IN", vec![from, to])
        } else {
            return Err(self.error());
        };
        let expression: Option<Box<Expression>> = match bounds.len() {
            0 => None,
            1 => bounds.into_iter().next().map(Box::new),
            _ => Some(Box::new(Expression::Tuple(Box::new(Tuple {
                expressions: bounds,
            })))),
        };
        Ok(Version {
            this: Box::new(Expression::Identifier(Identifier::new("SYSTEM_TIME"))),
            kind: kind.to_owned(),
            expression,
        })
    }

    /// `WITH ( hint [[,] hint ...] )`, the cursor at `WITH`, checked as SQL Server checks the hints of one table; `refused` holds those not allowed where the table stands.
    pub(crate) fn table_hints(&mut self, refused: &[(&str, &str)]) -> Parsing<Expression> {
        self.at += 2;
        let (mut hints, mut names): (Vec<Expression>, Vec<String>) = (Vec::new(), Vec::new());
        loop {
            let Some(word) = self.word_at(0) else {
                return Err(self.error());
            };
            let (hint, name): (Expression, &str) = match word.as_str() {
                "INDEX" => {
                    self.at += 1;
                    let values: Vec<Expression> = match self.eat_char('=') {
                        true => vec![self.index_value()?],
                        false => self.parenthesized(Parser::index_value)?,
                    };
                    (index_hint("INDEX", values, None), "INDEX")
                }
                "FORCESEEK" if self.char_at(1, '(') => {
                    self.at += 2;
                    let index: Expression = self.index_value()?;
                    let columns: Vec<Expression> =
                        self.parenthesized(|parser| Ok(Expression::Identifier(parser.name()?)))?;
                    self.expect_char(')')?;
                    let columns = Expression::Tuple(Box::new(Tuple {
                        expressions: columns,
                    }));
                    (
                        index_hint("FORCESEEK", vec![index], Some(columns)),
                        "FORCESEEK()",
                    )
                }
                SPATIAL_WINDOW_MAX_CELLS => {
                    let first: usize = self.at;
                    self.at += 1;
                    self.expect_char('=')?;
                    let cells: Expression = self.integer()?;
                    let hint: Expression =
                        self.binary(first, Expression::PropertyEQ, var(&word), cells);
                    (hint, SPATIAL_WINDOW_MAX_CELLS)
                }
                hint if TABLE_HINTS.contains(&hint) => {
                    self.at += 1;
                    (var(hint), hint)
                }
                _ => {
                    let raw: String = self.tokens[self.at].value.clone();
                    return Err(self.error_with(&format!(
                        "\"{raw}\" is not a recognized table hints option."
                    )));
                }
            };
            if let Some((_, message)) = refused.iter().find(|(hint, _)| *hint == name) {
                return Err(self.error_with(message));
            }
            hints.push(hint);
            names.push(name.to_owned());
            if !self.eat_char(',') && self.is_char(')') {
                break;
            }
        }
        self.expect_char(')')?;
        if let Some(message) = hint_conflict(&names) {
            return Err(self.error_with(message));
        }
        Ok(Expression::WithTableHint(Box::new(WithTableHint {
            expressions: hints,
        })))
    }

    /// An index in a table hint: its name, or its id as an integer.
    fn index_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number(_)) => self.integer(),
            _ => Ok(Expression::Identifier(self.name()?)),
        }
    }

    /// An unsigned integer literal.
    fn integer(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number(NumberKind::Integer)) => self.primary_value(),
            _ => Err(self.error()),
        }
    }

    /// `TABLESAMPLE [SYSTEM] (size [PERCENT | ROWS]) [REPEATABLE (seed)]`, the cursor at `TABLESAMPLE`.
    fn table_sample(&mut self) -> Parsing<Sample> {
        self.at += 1;
        let explicit_method: bool = self.eat_kw("SYSTEM");
        self.expect_char('(')?;
        let size: Expression = self.scalar()?;
        let is_percent: bool = self.eat_kw("PERCENT");
        let rows: bool = !is_percent && self.eat_kw("ROWS");
        self.expect_char(')')?;
        let seed: Option<Expression> = match self.eat_kw("REPEATABLE") {
            true => {
                self.expect_char('(')?;
                let seed: Expression = self.scalar()?;
                self.expect_char(')')?;
                Some(seed)
            }
            false => None,
        };
        Ok(Sample {
            method: match (explicit_method, rows) {
                (true, _) => SampleMethod::System,
                (false, true) => SampleMethod::Row,
                (false, false) => SampleMethod::Percent,
            },
            size,
            seed,
            offset: None,
            unit_after_size: is_percent || rows,
            use_sample_keyword: false,
            explicit_method,
            method_before_size: explicit_method,
            use_seed_keyword: false,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample: false,
            is_percent,
            suppress_method_output: false,
        })
    }

    /// A `FOR SYSTEM_TIME` bound: a constant, `NULL` or a variable; SQL Server refuses an expression there (102).
    fn system_time_bound(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(
                TokenKind::String { .. }
                | TokenKind::Number(_)
                | TokenKind::Variable
                | TokenKind::Char('{'),
            ) => self.primary_value(),
            _ if self.kw("NULL") => {
                self.at += 1;
                Ok(null())
            }
            _ => Err(self.error()),
        }
    }

    /// `name ( [value | DEFAULT, ...] ) [[AS] alias [(columns)]]`, a table-valued function, the cursor at the `(`.
    fn table_function_call(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        if name.len() > 1
            && name
                .iter()
                .skip(1)
                .any(|part| !part.quoted && part.name.eq_ignore_ascii_case("OPENJSON"))
        {
            return Err(self.error());
        }
        self.expect_char('(')?;
        let args: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| match parser.kw("DEFAULT") {
                true => {
                    parser.at += 1;
                    Ok(var("DEFAULT"))
                }
                false => parser.scalar(),
            })?,
        };
        self.expect_char(')')?;
        let name_start: Option<usize> = name
            .first()
            .and_then(|part| part.span)
            .map(|span| span.start);
        let mut node: Function = function(name, args);
        node.span = SourceSpan(name_start.map(|start| Span {
            start,
            end: self.tokens[self.at - 1].span.end,
            line: 0,
            column: 0,
        }));
        let call: Expression = Expression::Function(Box::new(node));
        let explicit: bool = self.kw("AS");
        let Some(alias) = self.table_alias()? else {
            return Ok(call);
        };
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.parenthesized(Parser::name)?,
            false => Vec::new(),
        };
        let mut node: Alias = Alias::new(call, alias);
        node.column_aliases = columns;
        node.alias_explicit_as = explicit;
        Ok(Expression::Alias(Box::new(node)))
    }

    /// `OPENJSON ( json [, path] ) [WITH ( column type ['path'] [AS JSON], ... )] [[AS] alias]`.
    fn open_json(&mut self) -> Parsing<Expression> {
        self.at += 1;
        self.expect_char('(')?;
        let this: Expression = self.scalar()?;
        let path: Option<Box<Expression>> = match self.eat_char(',') {
            true => Some(Box::new(self.scalar()?)),
            false => None,
        };
        self.expect_char(')')?;
        let mut columns: Vec<Expression> = Vec::new();
        if self.kw("WITH") && self.char_at(1, '(') {
            self.at += 1;
            columns = self.parenthesized(|parser| {
                let name: Identifier = identifier(&parser.expect_token(is_any_name)?);
                let data_type: DataType = parser.data_type()?;
                if parser.kw("COLLATE") {
                    return Err(parser.gap("openjson_collate", parser.offset()));
                }
                let path: Option<Box<Expression>> = match parser.peek() {
                    Some(token) if matches!(token.kind, TokenKind::String { .. }) => {
                        Some(Box::new(parser.primary()?))
                    }
                    _ => None,
                };
                let as_json: Option<Box<Expression>> =
                    match parser.kw("AS") && parser.kw_at(1, "JSON") {
                        true => {
                            parser.at += 2;
                            Some(Box::new(Expression::Boolean(BooleanLiteral {
                                value: true,
                            })))
                        }
                        false => None,
                    };
                Ok(Expression::OpenJSONColumnDef(Box::new(OpenJSONColumnDef {
                    this: Box::new(Expression::Identifier(name)),
                    kind: String::new(),
                    path,
                    as_json,
                    data_type: Some(data_type),
                })))
            })?;
        }
        let node: Expression = Expression::OpenJSON(Box::new(OpenJSON {
            this: Box::new(this),
            path,
            expressions: columns,
        }));
        let explicit: bool = self.kw("AS");
        Ok(match self.table_alias()? {
            Some(alias) => {
                let mut aliased: Alias = Alias::new(node, alias);
                aliased.alias_explicit_as = explicit;
                Expression::Alias(Box::new(aliased))
            }
            None => node,
        })
    }

    /// One table source and the joins after it.
    pub(crate) fn single_source(&mut self) -> Parsing<(Expression, Vec<Join>)> {
        let source: Expression = self.table_source()?;
        Ok((source, self.joins()?))
    }

    /// The joins after a table source.
    fn joins(&mut self) -> Parsing<Vec<Join>> {
        let mut joins: Vec<Join> = Vec::new();
        loop {
            let hint_at = |offset: usize| -> bool {
                self.word_at(offset)
                    .is_some_and(|word| JOIN_HINTS.contains(&word.as_str()))
                    && self.kw_at(offset + 1, "JOIN")
            };
            let (kind, words, keywords): (JoinKind, usize, (bool, bool)) =
                match (self.word_at(0).as_deref(), self.word_at(1).as_deref()) {
                    (Some("CROSS"), Some("JOIN")) => (JoinKind::Cross, 2, (false, false)),
                    (Some("CROSS"), Some("APPLY")) => (JoinKind::CrossApply, 2, (false, false)),
                    (Some("OUTER"), Some("APPLY")) => (JoinKind::OuterApply, 2, (false, false)),
                    (Some("JOIN"), _) => (JoinKind::Inner, 1, (false, false)),
                    (Some("INNER"), Some("JOIN")) => (JoinKind::Inner, 2, (true, false)),
                    (Some("INNER"), Some(_)) => match hint_at(1) {
                        true => (JoinKind::Inner, 3, (true, false)),
                        false => return Err(self.error_at(self.at + 1)),
                    },
                    (Some(side @ ("LEFT" | "RIGHT" | "FULL")), Some(next)) => {
                        let kind: JoinKind = match side {
                            "LEFT" => JoinKind::Left,
                            "RIGHT" => JoinKind::Right,
                            _ => JoinKind::Full,
                        };
                        let outer: bool = self.kw_at(1, "OUTER");
                        let at: usize = 1 + usize::from(outer);
                        match (self.kw_at(at, "JOIN"), hint_at(at)) {
                            (true, _) => (kind, at + 1, (false, outer)),
                            (false, true) if self.kw_at(at, "REMOTE") => {
                                return Err(self.error_at(self.at + at));
                            }
                            (false, true) => (kind, at + 2, (false, outer)),
                            (false, false) if outer || JOIN_HINTS.contains(&next) => {
                                return Err(self.error_at(self.at + at));
                            }
                            (false, false) => return Ok(joins),
                        }
                    }
                    (Some("LOOP" | "HASH" | "MERGE" | "REMOTE"), Some(_))
                        if !self.kw_at(0, "MERGE") || self.kw_at(1, "JOIN") =>
                    {
                        return Err(self.error());
                    }
                    _ => return Ok(joins),
                };
            let hint: Option<String> = words
                .checked_sub(2)
                .and_then(|at| self.word_at(at))
                .filter(|word| JOIN_HINTS.contains(&word.as_str()));
            self.at += words;
            let right: Expression = self.table_source()?;
            let constraint: JoinConstraint = match kind {
                JoinKind::Cross | JoinKind::CrossApply | JoinKind::OuterApply => {
                    JoinConstraint::None
                }
                _ => {
                    let nested: Vec<Join> = self.joins()?;
                    if !nested.is_empty() {
                        return Err(self.gap("nested_join", self.offset()));
                    }
                    self.expect_kw("ON")?;
                    JoinConstraint::On(Box::new(self.condition()?))
                }
            };
            let mut node: Join = join(right, kind, constraint, keywords);
            node.join_hint = hint;
            joins.push(node);
        }
    }
}

/// An `INDEX` or parameterized `FORCESEEK` table hint.
fn index_hint(name: &str, values: Vec<Expression>, columns: Option<Expression>) -> Expression {
    Expression::IndexTableHint(Box::new(IndexTableHint {
        this: Box::new(var(name)),
        expressions: values,
        target: columns.map(Box::new),
    }))
}

/// Whether `names` holds any of `hints`.
fn names_any(names: &[String], hints: &[&str]) -> bool {
    names.iter().any(|name| hints.contains(&name.as_str()))
}

/// SQL Server's error for one table's hints that cannot stand together, by the name of each hint.
fn hint_conflict(names: &[String]) -> Option<&'static str> {
    let has = |hint: &str| names.iter().any(|name| name == hint);
    let levels: usize = ISOLATION_LEVELS
        .iter()
        .filter(|level| names_any(names, level))
        .count();
    let granularities: usize = LOCK_GRANULARITIES.iter().filter(|hint| has(hint)).count();
    let dirty: bool = names_any(names, ISOLATION_LEVELS[0]);
    let locks: bool = (granularities > 0 || has("UPDLOCK") || has("XLOCK")) && dirty;
    let seeks: usize = names
        .iter()
        .filter(|name| *name == PARAMETERIZED_FORCESEEK)
        .count();
    if levels > 1 || granularities > 1 || (has("UPDLOCK") && has("XLOCK")) || locks {
        return Some(CONFLICTING_LOCKS);
    }
    if has("FORCESCAN") && (has("FORCESEEK") || seeks > 0) {
        return Some(
            "The FORCESCAN hint is specified simultaneously with the FORCESEEK hint. Remove one of the hints and resubmit the query.",
        );
    }
    if seeks > 1 {
        return Some(
            "There are multiple FORCESEEK hints specified on the same table or view. Remove the extra FORCESEEK hints and resubmit the query.",
        );
    }
    if seeks > 0 && (has("INDEX") || has("FORCESEEK")) {
        return Some(
            "The parameterized FORCESEEK hint cannot be simultaneously used with INDEX hints or a non-parameterized FORCESEEK hint on the same object.",
        );
    }
    if names
        .iter()
        .filter(|name| *name == SPATIAL_WINDOW_MAX_CELLS)
        .count()
        > 1
    {
        return Some("The SPATIAL_WINDOW_MAX_CELLS hint cannot be specified more than once.");
    }
    None
}

/// Puts a `WITH` on the query it heads.
pub(crate) fn set_with(query: &mut Expression, with: With) {
    match query {
        Expression::Select(select) => select.with = Some(with),
        Expression::Union(node) => node.with = Some(with),
        Expression::Except(node) => node.with = Some(with),
        Expression::Intersect(node) => node.with = Some(with),
        Expression::Insert(node) => node.with = Some(with),
        Expression::Update(node) => node.with = Some(with),
        Expression::Delete(node) => node.with = Some(with),
        Expression::Subquery(node) => set_with(&mut node.this, with),
        _ => {}
    }
}
